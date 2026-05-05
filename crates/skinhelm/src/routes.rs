use std::collections::HashMap;

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, Response, StatusCode};
use axum::response::{IntoResponse, Response as AxumResponse};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use skinhelm_core::error::AppError;
use skinhelm_core::render::render_helm_png;
#[cfg(feature = "wasm-viewer")]
use skinhelm_core::types::normalize_uuid;
use skinhelm_core::types::{parse_player, parse_size, PlayerInput};

#[cfg(feature = "wasm-viewer")]
use crate::viewer::{viewer_bootstrap, viewer_index, viewer_redirect, viewer_wasm, viewer_wasm_js};
use crate::AppState;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: &'static str,
}

struct HttpError(AppError);

impl From<AppError> for HttpError {
    fn from(error: AppError) -> Self {
        Self(error)
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> AxumResponse {
        let status = if self.0.is_bad_request() {
            StatusCode::BAD_REQUEST
        } else if self.0.is_not_found() {
            StatusCode::NOT_FOUND
        } else if self.0.is_internal() {
            StatusCode::INTERNAL_SERVER_ERROR
        } else {
            StatusCode::BAD_GATEWAY
        };
        let message = self.0.message();

        match status {
            StatusCode::INTERNAL_SERVER_ERROR => {
                tracing::error!(error = ?self.0, "internal error");
            }
            StatusCode::BAD_GATEWAY => {
                tracing::warn!(error = ?self.0, "upstream error");
            }
            _ => {}
        }

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

pub fn create_router(state: AppState) -> Router {
    let router = Router::new()
        .route("/health", get(health))
        .route("/helm/{player}", get(helm));

    #[cfg(feature = "wasm-viewer")]
    let router = router
        .route("/viewer", get(viewer_redirect))
        .route("/viewer/", get(viewer_index))
        .route("/viewer/{uuid}", get(viewer_player))
        .route("/viewer/skin/{uuid}", get(viewer_skin))
        .route("/viewer/cape/{uuid}", get(viewer_cape))
        .route("/viewer/bootstrap.js", get(viewer_bootstrap))
        .route("/viewer/pkg/skinhelm_wasm.js", get(viewer_wasm_js))
        .route("/viewer/pkg/skinhelm_wasm_bg.wasm", get(viewer_wasm));

    router.with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn helm(
    State(state): State<AppState>,
    Path(player): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Response<Body>, HttpError> {
    let size = parse_size(query.get("size").map(String::as_str))?;
    let player = parse_player(&player)?;

    let uuid = match player {
        PlayerInput::Uuid(uuid) => uuid,
        PlayerInput::Username(username) => resolve_username_cached(&state, &username).await?,
    };

    let skin_url = resolve_skin_url_cached(&state, &uuid).await?;

    if let Some(png) = state.cache.get_rendered_png(&skin_url, size).await {
        tracing::debug!(uuid, size, "rendered helm cache hit");
        return png_response(png);
    }

    tracing::debug!(uuid, size, "rendering helm");
    let skin_png = state.mojang.download_skin(&skin_url).await?;
    let rendered = render_helm_png(&skin_png, size)?;
    state
        .cache
        .set_rendered_png(&skin_url, size, rendered.clone())
        .await;

    png_response(rendered)
}

#[cfg(feature = "wasm-viewer")]
async fn viewer_player(Path(uuid): Path<String>) -> Result<Response<Body>, HttpError> {
    normalize_uuid(&uuid).ok_or(AppError::InvalidPlayer)?;
    Ok(viewer_index().await)
}

#[cfg(feature = "wasm-viewer")]
async fn viewer_skin(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
) -> Result<Response<Body>, HttpError> {
    let uuid = normalize_uuid(&uuid).ok_or(AppError::InvalidPlayer)?;
    let skin_profile = state.mojang.fetch_skin_profile(&uuid).await?;
    state
        .cache
        .set_skin_url(&uuid, skin_profile.url.clone())
        .await;
    let skin_png = state.mojang.download_skin(&skin_profile.url).await?;
    skin_response(skin_png, skin_profile.slim, skin_profile.cape_url.is_some())
}

#[cfg(feature = "wasm-viewer")]
async fn viewer_cape(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
) -> Result<Response<Body>, HttpError> {
    let uuid = normalize_uuid(&uuid).ok_or(AppError::InvalidPlayer)?;
    let skin_profile = state.mojang.fetch_skin_profile(&uuid).await?;
    let cape_url = skin_profile.cape_url.ok_or(AppError::ProfileHasNoCape)?;
    let cape_png = state.mojang.download_cape(&cape_url).await?;
    cape_response(cape_png)
}

async fn resolve_username_cached(state: &AppState, username: &str) -> Result<String, AppError> {
    let cache_key = username.to_ascii_lowercase();
    if let Some(uuid) = state.cache.get_username_uuid(&cache_key).await {
        tracing::debug!(username = cache_key, "username cache hit");
        return Ok(uuid);
    }

    let uuid = state.mojang.resolve_username(username).await?;
    state
        .cache
        .set_username_uuid(&cache_key, uuid.clone())
        .await;
    Ok(uuid)
}

async fn resolve_skin_url_cached(state: &AppState, uuid: &str) -> Result<String, AppError> {
    if let Some(skin_url) = state.cache.get_skin_url(uuid).await {
        tracing::debug!(uuid, "skin url cache hit");
        return Ok(skin_url);
    }

    let skin_url = state.mojang.fetch_skin_url(uuid).await?;
    state.cache.set_skin_url(uuid, skin_url.clone()).await;
    Ok(skin_url)
}

fn png_response(png: Vec<u8>) -> Result<Response<Body>, HttpError> {
    let mut response = Response::new(Body::from(png));
    *response.status_mut() = StatusCode::OK;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
    response.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600"),
    );
    Ok(response)
}

#[cfg(feature = "wasm-viewer")]
fn skin_response(png: Vec<u8>, slim: bool, has_cape: bool) -> Result<Response<Body>, HttpError> {
    let mut response = Response::new(Body::from(png));
    *response.status_mut() = StatusCode::OK;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
    response.headers_mut().insert(
        "x-skinhelm-model",
        HeaderValue::from_static(if slim { "slim" } else { "classic" }),
    );
    response.headers_mut().insert(
        "x-skinhelm-cape",
        HeaderValue::from_static(if has_cape { "true" } else { "false" }),
    );
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

#[cfg(feature = "wasm-viewer")]
fn cape_response(png: Vec<u8>) -> Result<Response<Body>, HttpError> {
    let mut response = Response::new(Body::from(png));
    *response.status_mut() = StatusCode::OK;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}
