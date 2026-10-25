//! Resumable uploads into a library root.
//!
//! A browser creates a session, sends chunks with `Upload-Offset`, and finishes
//! when the declared size is reached. The staged file lives inside the target
//! root (`.oplmdash/uploads/<id>/<name>`) so `PlanImport` can read it through
//! the normal allow-list. Import with `transfer: "Move"` consumes it.
//!
//! This is the Phase 0 prototype: sessions are in memory and a server restart
//! loses them. The staged files remain and are safe to delete.

use std::collections::HashMap;
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use yaoplm_api::{ApiError, PathPolicy};
use yaoplm_device::{FsKind, volume_for_path};

use crate::AppState;
use crate::rpc::{api_error, error_response};

/// Browser chunks are normally 8 MiB; leave room for a larger client.
pub const MAX_CHUNK: usize = 16 * 1024 * 1024;
const STAGING_DIR: &str = ".oplmdash";
const UPLOADS_DIR: &str = "uploads";
const MAX_NAME_BYTES: usize = 255;

#[derive(Debug, Default)]
pub(crate) struct UploadStore {
    // Per-session locks: a slow chunk write blocks only its own upload, not
    // every other one behind a global mutex (F-02).
    sessions: HashMap<String, Arc<tokio::sync::Mutex<Upload>>>,
}

#[derive(Debug, Clone)]
struct Upload {
    name: String,
    path: PathBuf,
    size: u64,
    offset: u64,
    complete: bool,
    hasher: Sha256,
    final_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUpload {
    /// Library root, as an absolute host path.
    pub root: String,
    /// File name only; no directories.
    pub name: String,
    /// Total file size in bytes.
    pub size: u64,
}

#[derive(Debug, Serialize)]
pub struct UploadStatus {
    pub id: String,
    pub name: String,
    pub offset: u64,
    pub size: u64,
    pub complete: bool,
    /// Staged path to pass to `PlanImport.sources`.
    pub path: String,
    /// Lower-case hex SHA-256 of the bytes written, set once complete (F-02).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

impl Upload {
    fn status(&self, id: &str) -> UploadStatus {
        UploadStatus {
            id: id.to_owned(),
            name: self.name.clone(),
            offset: self.offset,
            size: self.size,
            complete: self.complete,
            path: self.path.to_string_lossy().into_owned(),
            sha256: self.final_hash.clone(),
        }
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/uploads", post(create))
        .route("/api/uploads/{id}", get(status).delete(cancel))
        .route("/api/uploads/{id}/chunk", post(append))
        .route("/api/uploads/{id}/finish", post(finish))
        .layer(DefaultBodyLimit::max(MAX_CHUNK))
}

pub async fn create(State(state): State<AppState>, Json(body): Json<CreateUpload>) -> Response {
    match create_session(&state, body).await {
        Ok(status) => Json(status).into_response(),
        Err(error) => api_error(error),
    }
}

async fn create_session(state: &AppState, body: CreateUpload) -> Result<UploadStatus, ApiError> {
    if !state.config().uploads_enabled {
        return Err(ApiError::forbidden("uploads are disabled"));
    }
    let policy = PathPolicy::Within(state.config().roots.clone());
    let root = policy.resolve(&body.root)?;
    let meta = tokio::fs::metadata(&root)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    if !meta.is_dir() {
        return Err(ApiError::bad_request("root is not a directory"));
    }
    validate_name(&body.name)?;
    if body.size == 0 {
        return Err(ApiError::bad_request("file is empty"));
    }
    // F-02: enforce the configured limit and preflight the target volume. A
    // FAT32 volume cannot store a file of 4 GiB or more, and no volume can
    // store more than its free space.
    if body.size > state.config().max_upload_size {
        return Err(ApiError::bad_request(format!(
            "file exceeds the {} byte upload limit",
            state.config().max_upload_size
        )));
    }
    let volume = volume_for_path(&root).map_err(|error| ApiError::internal(error.to_string()))?;
    if volume.fs == FsKind::Fat32 && body.size > 4_294_967_295 {
        return Err(ApiError::bad_request(
            "FAT32 cannot store a file of 4 GiB or more",
        ));
    }
    if body.size > volume.free_bytes {
        return Err(ApiError::bad_request(
            "not enough free space on the target volume",
        ));
    }

    // F-01: refuse symlinked staging components before creating anything.
    for component in [STAGING_DIR, UPLOADS_DIR] {
        let staging = root.join(component);
        if let Ok(meta) = tokio::fs::symlink_metadata(&staging).await {
            if meta.file_type().is_symlink() {
                return Err(ApiError::forbidden("staging directory is a symlink"));
            }
        }
    }
    let id = new_id();
    let dir = root.join(STAGING_DIR).join(UPLOADS_DIR).join(&id);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    // The created staging directory must canonicalise inside the root; a
    // pre-existing symlink or a TOCTOU replacement is refused here.
    let canonical_dir = tokio::fs::canonicalize(&dir)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    if !canonical_dir.starts_with(&root) {
        let _ = tokio::fs::remove_dir_all(&dir).await;
        return Err(ApiError::forbidden("staging directory escapes the root"));
    }
    let path = dir.join(&body.name);
    let created = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .await;
    if let Err(error) = created {
        let _ = tokio::fs::remove_dir_all(&dir).await;
        return Err(ApiError::internal(error.to_string()));
    }

    let upload = Upload {
        name: body.name,
        path,
        size: body.size,
        offset: 0,
        complete: false,
        hasher: Sha256::new(),
        final_hash: None,
    };
    let status = upload.status(&id);
    state
        .uploads()
        .lock()
        .await
        .sessions
        .insert(id, Arc::new(tokio::sync::Mutex::new(upload)));
    Ok(status)
}

pub async fn status(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let session = {
        let store = state.uploads().lock().await;
        store.sessions.get(&id).cloned()
    };
    let Some(session) = session else {
        return api_error(ApiError::not_found("unknown upload"));
    };
    let upload = session.lock().await;
    Json(upload.status(&id)).into_response()
}

pub async fn append(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !is_octet_stream(&headers) {
        return error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ApiError::bad_request("expected Content-Type: application/octet-stream"),
        );
    }
    let Some(offset) = upload_offset(&headers) else {
        return error_response(
            StatusCode::BAD_REQUEST,
            ApiError::bad_request("missing or invalid Upload-Offset"),
        );
    };

    let session = {
        let store = state.uploads().lock().await;
        store.sessions.get(&id).cloned()
    };
    let Some(session) = session else {
        return api_error(ApiError::not_found("unknown upload"));
    };
    // Hold the session's lock across its I/O only; another upload proceeds
    // in parallel (F-02).
    let mut upload = session.lock().await;
    if upload.complete {
        return error_response(
            StatusCode::CONFLICT,
            ApiError::bad_request("upload is already complete"),
        );
    }
    if offset != upload.offset {
        return error_response(
            StatusCode::CONFLICT,
            ApiError::bad_request(format!("offset mismatch: expected {}", upload.offset)),
        );
    }
    let len = body.len() as u64;
    if len > upload.size.saturating_sub(upload.offset) {
        return error_response(
            StatusCode::BAD_REQUEST,
            ApiError::bad_request("chunk exceeds the declared file size"),
        );
    }

    let result = write_chunk(&upload.path, upload.offset, &body).await;
    if let Err(error) = result {
        // A failed write may have left a partial chunk; cut it back so the
        // next request with the same offset is safe.
        if let Ok(file) = tokio::fs::OpenOptions::new()
            .write(true)
            .open(&upload.path)
            .await
        {
            let _ = file.set_len(upload.offset).await;
        }
        return api_error(ApiError::internal(error.to_string()));
    }
    upload.hasher.update(&body);
    upload.offset += len;
    let status = upload.status(&id);
    Json(status).into_response()
}

async fn write_chunk(path: &FsPath, offset: u64, body: &[u8]) -> std::io::Result<()> {
    let mut file = tokio::fs::OpenOptions::new().write(true).open(path).await?;
    file.seek(std::io::SeekFrom::Start(offset)).await?;
    file.write_all(body).await?;
    file.flush().await
}

pub async fn finish(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let session = {
        let store = state.uploads().lock().await;
        store.sessions.get(&id).cloned()
    };
    let Some(session) = session else {
        return api_error(ApiError::not_found("unknown upload"));
    };
    let mut upload = session.lock().await;
    if upload.complete {
        return Json(upload.status(&id)).into_response();
    }
    if upload.offset != upload.size {
        return error_response(
            StatusCode::CONFLICT,
            ApiError::bad_request(format!(
                "upload is incomplete: {} of {} bytes",
                upload.offset, upload.size
            )),
        );
    }
    let meta = match tokio::fs::metadata(&upload.path).await {
        Ok(meta) => meta,
        Err(error) => return api_error(ApiError::internal(error.to_string())),
    };
    if meta.len() != upload.size {
        return api_error(ApiError::internal("staged file has the wrong size"));
    }
    let synced = async {
        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .open(&upload.path)
            .await?;
        file.sync_all().await
    }
    .await;
    if let Err(error) = synced {
        return api_error(ApiError::internal(error.to_string()));
    }
    let digest = upload.hasher.clone().finalize();
    upload.final_hash = Some(digest.iter().map(|b| format!("{b:02x}")).collect());
    upload.complete = true;
    Json(upload.status(&id)).into_response()
}

pub async fn cancel(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let session = {
        let mut store = state.uploads().lock().await;
        store.sessions.remove(&id)
    };
    let Some(session) = session else {
        return api_error(ApiError::not_found("unknown upload"));
    };
    let upload = session.lock().await;
    if let Some(dir) = upload.path.parent() {
        match tokio::fs::remove_dir_all(dir).await {
            Ok(()) => {}
            // `PlanImport` with `move` may already have removed the file and
            // the directory; cancelling is still idempotent.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return api_error(ApiError::internal(error.to_string())),
        }
    }
    StatusCode::NO_CONTENT.into_response()
}

/// Removes staged files whose session is gone and whose directory is older
/// than a day. Called on startup and hourly by the server (F-02).
pub async fn cleanup_stale(state: &AppState) {
    let roots = state.config().roots.clone();
    let active: std::collections::HashSet<String> = {
        let store = state.uploads().lock().await;
        store.sessions.keys().cloned().collect()
    };
    for root in roots {
        let staging = root.join(STAGING_DIR).join(UPLOADS_DIR);
        let mut read = match tokio::fs::read_dir(&staging).await {
            Ok(read) => read,
            Err(_) => continue,
        };
        let now = SystemTime::now();
        while let Ok(Some(entry)) = read.next_entry().await {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if active.contains(&name) {
                continue;
            }
            let Ok(meta) = entry.metadata().await else {
                continue;
            };
            if !meta.is_dir() {
                continue;
            }
            let Ok(modified) = meta.modified() else {
                continue;
            };
            let Ok(age) = now.duration_since(modified) else {
                continue;
            };
            if age.as_secs() > 24 * 60 * 60 {
                let _ = tokio::fs::remove_dir_all(entry.path()).await;
            }
        }
    }
}

fn validate_name(name: &str) -> Result<(), ApiError> {
    if name.is_empty() || name.len() > MAX_NAME_BYTES {
        return Err(ApiError::bad_request("file name is empty or too long"));
    }
    if name == "." || name == ".." || name.contains('/') || name.contains('\\') {
        return Err(ApiError::bad_request("file name must not contain a path"));
    }
    if name.chars().any(char::is_control) {
        return Err(ApiError::bad_request(
            "file name contains a control character",
        ));
    }
    if FsPath::new(name).file_name().and_then(|part| part.to_str()) != Some(name) {
        return Err(ApiError::bad_request("invalid file name"));
    }
    Ok(())
}

fn is_octet_stream(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("application/octet-stream"))
}

fn upload_offset(headers: &HeaderMap) -> Option<u64> {
    headers
        .get("upload-offset")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
}

fn new_id() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!(
        "{:x}-{:x}-{:x}",
        std::process::id(),
        nanos,
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::Request;
    use serde_json::{Value, json};
    use tower::ServiceExt;
    use yaoplm_api::Api;

    use super::*;
    use crate::Config;

    const HOST: &str = "127.0.0.1:7878";

    fn state(root: &FsPath, token: Option<&str>) -> AppState {
        let root = std::fs::canonicalize(root).unwrap();
        AppState::new(
            Arc::new(Api::with_roots(vec![root.clone()])),
            Config {
                token: token.map(str::to_owned),
                roots: vec![root],
                loopback_only: true,
                uploads_enabled: true,
                rpc_allow_all: false,
                max_upload_size: 4 * 1024 * 1024 * 1024,
            },
        )
    }

    async fn send(state: AppState, request: Request<Body>) -> (StatusCode, Vec<u8>) {
        let response = crate::app(state).oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec();
        (status, bytes)
    }

    async fn send_json(state: AppState, request: Request<Body>) -> (StatusCode, Value) {
        let (status, bytes) = send(state, request).await;
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    fn create_request(root: &FsPath, name: &str, size: u64) -> Request<Body> {
        Request::post("/api/uploads")
            .header(header::HOST, HOST)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "root": root.to_string_lossy(), "name": name, "size": size }).to_string(),
            ))
            .unwrap()
    }

    fn chunk_request(id: &str, offset: u64, body: &str) -> Request<Body> {
        Request::post(format!("/api/uploads/{id}/chunk"))
            .header(header::HOST, HOST)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header("upload-offset", offset.to_string())
            .body(Body::from(body.to_owned()))
            .unwrap()
    }

    #[tokio::test]
    async fn upload_resumes_and_finishes() {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path(), None);
        let (status, body) =
            send_json(state.clone(), create_request(dir.path(), "game.iso", 11)).await;
        assert_eq!(status, StatusCode::OK);
        let id = body["id"].as_str().unwrap().to_owned();
        let path = body["path"].as_str().unwrap().to_owned();
        assert!(path.contains(".oplmdash/uploads"));

        let (status, _) = send(state.clone(), chunk_request(&id, 0, "hello")).await;
        assert_eq!(status, StatusCode::OK);
        let (status, body) = send_json(
            state.clone(),
            Request::get(format!("/api/uploads/{id}"))
                .header(header::HOST, HOST)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["offset"].as_u64(), Some(5));

        let (status, _) = send(state.clone(), chunk_request(&id, 4, "x")).await;
        assert_eq!(status, StatusCode::CONFLICT);
        let (status, _) = send(state.clone(), chunk_request(&id, 5, " world")).await;
        assert_eq!(status, StatusCode::OK);

        let (status, body) = send_json(
            state.clone(),
            Request::post(format!("/api/uploads/{id}/finish"))
                .header(header::HOST, HOST)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["complete"].as_bool(), Some(true));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello world");

        let (status, _) = send(
            state,
            Request::delete(format!("/api/uploads/{id}"))
                .header(header::HOST, HOST)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(!FsPath::new(&path).exists());
    }

    #[tokio::test]
    async fn upload_rejects_escape_names_and_foreign_roots() {
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let state = state(dir.path(), None);
        let (status, _) =
            send_json(state.clone(), create_request(dir.path(), "../evil.iso", 1)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let (status, _) = send_json(state, create_request(other.path(), "game.iso", 1)).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn upload_rejects_oversized_chunks_and_wrong_content_type() {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path(), None);
        let (_, body) = send_json(state.clone(), create_request(dir.path(), "game.iso", 3)).await;
        let id = body["id"].as_str().unwrap();
        let (status, _) = send(state.clone(), chunk_request(id, 0, "hello")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);

        let wrong = Request::post(format!("/api/uploads/{id}/chunk"))
            .header(header::HOST, HOST)
            .header(header::CONTENT_TYPE, "text/plain")
            .header("upload-offset", "0")
            .body(Body::from("hello"))
            .unwrap();
        assert_eq!(
            send(state, wrong).await.0,
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
    }

    fn rpc_post() -> axum::http::request::Builder {
        Request::post("/api/rpc")
            .header(header::HOST, HOST)
            .header(header::CONTENT_TYPE, "application/json")
    }

    #[tokio::test]
    async fn plan_import_accepts_the_staged_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("CD")).unwrap();
        std::fs::create_dir_all(dir.path().join("DVD")).unwrap();
        let state = state(dir.path(), None);
        let (_, body) = send_json(state.clone(), create_request(dir.path(), "fake.iso", 4)).await;
        let id = body["id"].as_str().unwrap().to_owned();
        let path = body["path"].as_str().unwrap().to_owned();
        assert_eq!(
            send(state.clone(), chunk_request(&id, 0, "fake")).await.0,
            StatusCode::OK
        );
        assert_eq!(
            send_json(
                state.clone(),
                Request::post(format!("/api/uploads/{id}/finish"))
                    .header(header::HOST, HOST)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .0,
            StatusCode::OK
        );

        let request = json!({
            "type": "PlanImport",
            "root": dir.path().to_string_lossy(),
            "sources": [path],
            "transfer": "move",
        })
        .to_string();
        let (status, body) = send_json(state, rpc_post().body(Body::from(request)).unwrap()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["type"], "Plan");
    }

    #[tokio::test]
    async fn upload_requires_the_token() {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path(), Some("s3cret"));
        let (status, _) = send_json(state, create_request(dir.path(), "game.iso", 1)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
