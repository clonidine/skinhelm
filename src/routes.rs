use std::collections::HashMap;

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, Response, StatusCode};
use axum::routing::get;
use axum::{Json, Router};

use crate::error::AppError;
use crate::render::render_helm_png;
use crate::types::{parse_player, parse_size, HealthResponse, PlayerInput};
use crate::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/helm/{player}", get(helm))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn helm(
    State(state): State<AppState>,
    Path(player): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Response<Body>, AppError> {
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

fn png_response(png: Vec<u8>) -> Result<Response<Body>, AppError> {
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
