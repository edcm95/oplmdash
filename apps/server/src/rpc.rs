use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use yaoplm_api::{ApiError, ErrorCode, Layout, Request};

use crate::AppState;

pub async fn handle(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    // Cross-site forms can only send text/plain or form bodies; insisting on
    // JSON forces a CORS preflight that we never answer.
    if !is_json(&headers) {
        return error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ApiError::bad_request("expected Content-Type: application/json"),
        );
    }
    let Ok(body) = String::from_utf8(body.to_vec()) else {
        return api_error(ApiError::bad_request("request body is not UTF-8"));
    };

    let allow_all = state.config().rpc_allow_all;
    let api = state.api().clone();
    match tokio::task::spawn_blocking(move || {
        let request: Request = serde_json::from_str(&body)
            .map_err(|error| ApiError::bad_request(format!("invalid request: {error}")))?;
        // F-03: server mode exposes the v0.1 surface only. The UI hiding a
        // control is not access control; a direct call must be refused too.
        if !allow_all && !allowed_in_server_mode(&request) {
            return Err(ApiError::forbidden(
                "operation is not available in server mode",
            ));
        }
        api.dispatch(request)
    })
    .await
    {
        Ok(Ok(response)) => Json(response).into_response(),
        Ok(Err(error)) => api_error(error),
        Err(join) => api_error(ApiError::internal(join.to_string())),
    }
}

/// Requests a server may answer without `rpc_allow_all`. Everything else is
/// deferred v0.1 scope: conversion, VMC creation, art and loader network
/// fetches, Redump updates and loader installs (F-03).
fn allowed_in_server_mode(request: &Request) -> bool {
    match request {
        Request::Ping
        | Request::Version
        | Request::ListDevices { .. }
        | Request::InspectDevice { .. }
        | Request::ScanLibrary { .. }
        | Request::ImageInfo { .. }
        | Request::ListDir { .. }
        | Request::PlanInit { .. }
        | Request::RenameGame { .. }
        | Request::DeleteGame { .. }
        | Request::MoveGame { .. }
        | Request::PlanRecovery { .. }
        | Request::ExecutePlan { .. }
        | Request::DiscardPlan { .. }
        | Request::GetJob { .. }
        | Request::ListJobs
        | Request::CancelJob { .. }
        | Request::SetArt { .. }
        | Request::ReadCfg { .. }
        | Request::ListVmc { .. }
        | Request::DeleteArt { .. }
        | Request::Attribution
        | Request::ArtCache
        | Request::ClearArtCache => true,
        // Memory-card creation is deferred; settings changes are allowed.
        Request::WriteCfg { cards, .. } => !cards.as_ref().is_some_and(|c| !c.is_empty()),
        Request::PlanImport {
            sources,
            compress,
            layout,
            covers,
            ..
        } => {
            // Only plain .iso/.zso images: no archives, BIN/CUE/CHD, UL
            // conversion, compression or cover writing in server mode.
            compress != &Some(true)
                && covers.is_none()
                && !matches!(layout, Some(Layout::Ul))
                && sources.iter().all(|source| {
                    let lower = source.to_ascii_lowercase();
                    lower.ends_with(".iso") || lower.ends_with(".zso")
                })
        }
        _ => false,
    }
}

pub fn api_error(error: ApiError) -> Response {
    let status = match error.code {
        ErrorCode::BadRequest => StatusCode::BAD_REQUEST,
        ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
        ErrorCode::Forbidden => StatusCode::FORBIDDEN,
        ErrorCode::NotFound => StatusCode::NOT_FOUND,
        ErrorCode::Cancelled => StatusCode::CONFLICT,
        ErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
    };
    error_response(status, error)
}

pub fn error_response(status: StatusCode, error: ApiError) -> Response {
    (status, Json(error)).into_response()
}

fn is_json(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("application/json"))
}
