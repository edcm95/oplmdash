//! `yaoplm-server`: the web UI and the RPC API over HTTP (ARCHITECTURE.md §9).
//!
//! Routes:
//! - `GET /health`: liveness, no auth;
//! - `POST /api/rpc`: one `yaoplm_api::Request` in, one `Response` or `ApiError` out;
//! - `GET /api/events`: WebSocket stream of `JobEvent`s as JSON text frames;
//! - `GET /media/art?path=`: a cover from a library root (ART-2);
//! - `/api/uploads/*`: resumable uploads into a root (Phase 0 prototype);
//! - everything else: the embedded SPA from `web/dist`.

mod assets;
mod auth;
mod events;
mod media;
mod rpc;
mod upload;

use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::{Json, Router, middleware};
use tokio::sync::broadcast;
use yaoplm_api::{Api, JobEvent, Subscription};

#[derive(Debug, Clone)]
pub struct Config {
    /// Required on every `/api` request when set.
    pub token: Option<String>,
    /// Allow-list of library roots the UI may open.
    pub roots: Vec<PathBuf>,
    /// The server listens on loopback only. Without a token, `/api` then
    /// also rejects foreign `Host` headers (DNS rebinding).
    pub loopback_only: bool,
    /// Accept uploads into a library root. Off by default: a server that only
    /// reads and imports existing files needs no staging area (F-01, F-02).
    pub uploads_enabled: bool,
    /// Expose the full RPC surface. Server mode allows a safe subset by
    /// default; the rest is deferred v0.1 scope (F-03).
    pub rpc_allow_all: bool,
    /// Largest upload accepted, in bytes. 4 GiB by default: the largest file
    /// FAT32 stores (F-02).
    pub max_upload_size: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            token: None,
            roots: Vec::new(),
            loopback_only: true,
            uploads_enabled: false,
            rpc_allow_all: false,
            max_upload_size: 4 * 1024 * 1024 * 1024,
        }
    }
}

#[derive(Clone)]
pub struct AppState(Arc<Inner>);

struct Inner {
    api: Arc<Api>,
    config: Config,
    events: broadcast::Sender<JobEvent>,
    uploads: tokio::sync::Mutex<upload::UploadStore>,
    _subscription: Subscription,
}

impl AppState {
    pub fn new(api: Arc<Api>, config: Config) -> Self {
        let (events, _) = broadcast::channel(256);
        let forward = events.clone();
        // No receivers just means no browser is connected; dropping is fine.
        let subscription = api.subscribe(move |event| {
            let _ = forward.send(event.clone());
        });
        Self(Arc::new(Inner {
            api,
            config,
            events,
            uploads: tokio::sync::Mutex::default(),
            _subscription: subscription,
        }))
    }

    pub fn api(&self) -> &Arc<Api> {
        &self.0.api
    }

    pub fn config(&self) -> &Config {
        &self.0.config
    }

    pub(crate) fn uploads(&self) -> &tokio::sync::Mutex<upload::UploadStore> {
        &self.0.uploads
    }
}

pub fn app(state: AppState) -> Router {
    let mut api = Router::new()
        .route("/api/rpc", post(rpc::handle))
        .route("/api/events", get(events::handle))
        .route("/media/art", get(media::art));
    // F-01/F-02: uploads are an opt-in prototype. Without the flag the routes
    // do not exist at all, so no request can reach the staging code.
    if state.config().uploads_enabled {
        api = api.merge(upload::router());
    }
    let api = api.route_layer(middleware::from_fn_with_state(state.clone(), auth::require));

    Router::new()
        .route("/health", get(health))
        .merge(api)
        .fallback(assets::serve)
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "version": yaoplm_api::VERSION }))
}

/// Removes stale upload staging directories. Call on startup and hourly (F-02).
pub async fn cleanup_stale_uploads(state: &AppState) {
    upload::cleanup_stale(state).await;
}
