use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::types::ErrorResponse;

#[derive(Debug)]
pub enum AppError {
    InvalidPlayer,
    InvalidSize,
    UsernameNotFound,
    ProfileHasNoSkin,
    MojangRequest,
    SkinDownload,
    MalformedUpstream,
    TextureDecode,
    TextureJson,
    InvalidSkinImage,
    Internal,
}

impl AppError {
    fn status_and_message(&self) -> (StatusCode, &'static str) {
        match self {
            Self::InvalidPlayer => (StatusCode::BAD_REQUEST, "invalid player"),
            Self::InvalidSize => (StatusCode::BAD_REQUEST, "invalid size"),
            Self::UsernameNotFound => (StatusCode::NOT_FOUND, "username not found"),
            Self::ProfileHasNoSkin => (StatusCode::NOT_FOUND, "skin not found"),
            Self::MojangRequest => (StatusCode::BAD_GATEWAY, "mojang request failed"),
            Self::SkinDownload => (StatusCode::BAD_GATEWAY, "skin download failed"),
            Self::MalformedUpstream => (StatusCode::BAD_GATEWAY, "malformed upstream response"),
            Self::TextureDecode => (StatusCode::BAD_GATEWAY, "invalid texture data"),
            Self::TextureJson => (StatusCode::BAD_GATEWAY, "invalid texture json"),
            Self::InvalidSkinImage => (StatusCode::BAD_GATEWAY, "invalid skin image"),
            Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal error"),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (_, message) = self.status_and_message();
        formatter.write_str(message)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = self.status_and_message();

        match status {
            StatusCode::INTERNAL_SERVER_ERROR => {
                tracing::error!(error = ?self, "internal error");
            }
            StatusCode::BAD_GATEWAY => {
                tracing::warn!(error = ?self, "upstream error");
            }
            _ => {}
        }

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}
