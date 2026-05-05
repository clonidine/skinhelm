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
    pub fn message(&self) -> &'static str {
        match self {
            Self::InvalidPlayer => "invalid player",
            Self::InvalidSize => "invalid size",
            Self::UsernameNotFound => "username not found",
            Self::ProfileHasNoSkin => "skin not found",
            Self::MojangRequest => "mojang request failed",
            Self::SkinDownload => "skin download failed",
            Self::MalformedUpstream => "malformed upstream response",
            Self::TextureDecode => "invalid texture data",
            Self::TextureJson => "invalid texture json",
            Self::InvalidSkinImage => "invalid skin image",
            Self::Internal => "internal error",
        }
    }

    pub fn is_bad_request(&self) -> bool {
        matches!(self, Self::InvalidPlayer | Self::InvalidSize)
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::UsernameNotFound | Self::ProfileHasNoSkin)
    }

    pub fn is_internal(&self) -> bool {
        matches!(self, Self::Internal)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message())
    }
}
