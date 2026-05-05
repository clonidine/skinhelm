use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use reqwest::StatusCode;

use crate::error::AppError;
use crate::types::{
    normalize_uuid, MojangProfileResponse, SkinProfile, TexturesPayload, UsernameProfileResponse,
};

const MOJANG_USERNAME_URL: &str = "https://api.mojang.com/users/profiles/minecraft/";
const MOJANG_PROFILE_URL: &str = "https://sessionserver.mojang.com/session/minecraft/profile/";
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct MojangClient {
    client: reqwest::Client,
}

impl MojangClient {
    pub fn new() -> Result<Self, AppError> {
        let client = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .user_agent("skinhelm/0.1.0")
            .build()
            .map_err(|err| {
                tracing::error!(error = %err, "failed to build reqwest client");
                AppError::Internal
            })?;

        Ok(Self { client })
    }

    pub async fn resolve_username(&self, username: &str) -> Result<String, AppError> {
        let url = format!("{MOJANG_USERNAME_URL}{username}");
        let response = self.client.get(url).send().await.map_err(|err| {
            tracing::warn!(error = %err, username, "username resolution request failed");
            AppError::MojangRequest
        })?;

        match response.status() {
            StatusCode::OK => {
                let profile = response
                    .json::<UsernameProfileResponse>()
                    .await
                    .map_err(|err| {
                        tracing::warn!(error = %err, username, "username response malformed");
                        AppError::MalformedUpstream
                    })?;

                normalize_uuid(&profile.id).ok_or(AppError::MalformedUpstream)
            }
            StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => Err(AppError::UsernameNotFound),
            status => {
                tracing::warn!(%status, username, "username resolution returned error status");
                Err(AppError::MojangRequest)
            }
        }
    }

    pub async fn fetch_skin_url(&self, uuid: &str) -> Result<String, AppError> {
        self.fetch_skin_profile(uuid)
            .await
            .map(|profile| profile.url)
    }

    pub async fn fetch_skin_profile(&self, uuid: &str) -> Result<SkinProfile, AppError> {
        let url = format!("{MOJANG_PROFILE_URL}{uuid}?unsigned=false");
        let response = self.client.get(url).send().await.map_err(|err| {
            tracing::warn!(error = %err, uuid, "profile request failed");
            AppError::MojangRequest
        })?;

        if !response.status().is_success() {
            let status = response.status();
            tracing::warn!(%status, uuid, "profile request returned error status");
            return Err(AppError::MojangRequest);
        }

        let profile = response
            .json::<MojangProfileResponse>()
            .await
            .map_err(|err| {
                tracing::warn!(error = %err, uuid, "profile response malformed");
                AppError::MalformedUpstream
            })?;

        let property = profile
            .properties
            .into_iter()
            .find(|property| property.name == "textures")
            .ok_or(AppError::ProfileHasNoSkin)?;

        let decoded = STANDARD.decode(property.value).map_err(|err| {
            tracing::warn!(error = %err, uuid, "texture property base64 decode failed");
            AppError::TextureDecode
        })?;

        let textures = serde_json::from_slice::<TexturesPayload>(&decoded).map_err(|err| {
            tracing::warn!(error = %err, uuid, "texture property json parse failed");
            AppError::TextureJson
        })?;

        let cape_url = textures
            .textures
            .cape
            .and_then(|cape| valid_texture_url(cape.url));
        let skin = textures.textures.skin.ok_or(AppError::ProfileHasNoSkin)?;
        let skin_url = skin.url;

        if let Some(skin_url) = valid_texture_url(skin_url) {
            let slim = skin
                .metadata
                .and_then(|metadata| metadata.model)
                .is_some_and(|model| model == "slim");
            Ok(SkinProfile {
                url: skin_url,
                slim,
                cape_url,
            })
        } else {
            Err(AppError::MalformedUpstream)
        }
    }

    pub async fn download_skin(&self, skin_url: &str) -> Result<Vec<u8>, AppError> {
        self.download_texture(skin_url, AppError::SkinDownload)
            .await
    }

    pub async fn download_cape(&self, cape_url: &str) -> Result<Vec<u8>, AppError> {
        self.download_texture(cape_url, AppError::CapeDownload)
            .await
    }

    async fn download_texture(
        &self,
        texture_url: &str,
        error: AppError,
    ) -> Result<Vec<u8>, AppError> {
        let response = self.client.get(texture_url).send().await.map_err(|err| {
            tracing::warn!(error = %err, "texture download request failed");
            error
        })?;

        if !response.status().is_success() {
            let status = response.status();
            tracing::warn!(%status, "texture download returned error status");
            return Err(error);
        }

        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|err| {
                tracing::warn!(error = %err, "texture download body read failed");
                error
            })
    }
}

fn valid_texture_url(url: String) -> Option<String> {
    if url.starts_with("http://") || url.starts_with("https://") {
        Some(url)
    } else {
        None
    }
}
