use serde::Deserialize;

use crate::error::AppError;

pub const HEAD_PIXEL_GRID: u32 = 8;
pub const DEFAULT_SIZE: u32 = 184;
pub const MIN_SIZE: u32 = 8;
pub const MAX_SIZE: u32 = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerInput {
    Uuid(String),
    Username(String),
}

#[derive(Debug, Deserialize)]
pub struct UsernameProfileResponse {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct MojangProfileResponse {
    pub properties: Vec<MojangProperty>,
}

#[derive(Debug, Deserialize)]
pub struct MojangProperty {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct TexturesPayload {
    pub textures: Textures,
}

#[derive(Debug, Deserialize)]
pub struct Textures {
    #[serde(rename = "SKIN")]
    pub skin: Option<SkinTexture>,
    #[serde(rename = "CAPE")]
    pub cape: Option<SkinTexture>,
}

#[derive(Debug, Deserialize)]
pub struct SkinTexture {
    pub url: String,
    pub metadata: Option<SkinMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct SkinMetadata {
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinProfile {
    pub url: String,
    pub slim: bool,
    pub cape_url: Option<String>,
}

pub fn parse_player(value: &str) -> Result<PlayerInput, AppError> {
    if let Some(uuid) = normalize_uuid(value) {
        return Ok(PlayerInput::Uuid(uuid));
    }

    if is_username_valid(value) {
        return Ok(PlayerInput::Username(value.to_owned()));
    }

    Err(AppError::InvalidPlayer)
}

pub fn normalize_uuid(value: &str) -> Option<String> {
    if value.len() == 32 && value.bytes().all(is_hex_byte) {
        return Some(value.to_ascii_lowercase());
    }

    if value.len() != 36 {
        return None;
    }

    let mut out = String::with_capacity(32);
    for (index, byte) in value.bytes().enumerate() {
        match index {
            8 | 13 | 18 | 23 if byte == b'-' => {}
            8 | 13 | 18 | 23 => return None,
            _ if is_hex_byte(byte) => out.push((byte as char).to_ascii_lowercase()),
            _ => return None,
        }
    }

    if out.len() == 32 {
        Some(out)
    } else {
        None
    }
}

pub fn is_username_valid(value: &str) -> bool {
    let len = value.len();
    (3..=16).contains(&len)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

pub fn parse_size(value: Option<&str>) -> Result<u32, AppError> {
    match value {
        None => Ok(DEFAULT_SIZE),
        Some(raw) => {
            let size = raw.parse::<u32>().map_err(|_| AppError::InvalidSize)?;
            normalize_size(size)
        }
    }
}

pub fn normalize_size(size: u32) -> Result<u32, AppError> {
    if !(MIN_SIZE..=MAX_SIZE).contains(&size) {
        return Err(AppError::InvalidSize);
    }

    let rounded = ((size + (HEAD_PIXEL_GRID / 2)) / HEAD_PIXEL_GRID) * HEAD_PIXEL_GRID;
    Ok(rounded.clamp(MIN_SIZE, MAX_SIZE))
}

fn is_hex_byte(byte: u8) -> bool {
    byte.is_ascii_hexdigit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_hyphenated_uuid() {
        let input = "BC881E02-9216-4F6E-A80F-7B9DF0CCF9E9";
        assert_eq!(
            normalize_uuid(input).as_deref(),
            Some("bc881e0292164f6ea80f7b9df0ccf9e9")
        );
    }

    #[test]
    fn detects_plain_uuid() {
        let input = "bc881e0292164f6ea80f7b9df0ccf9e9";
        assert_eq!(normalize_uuid(input).as_deref(), Some(input));
    }

    #[test]
    fn rejects_invalid_uuid() {
        assert!(normalize_uuid("bc881e0292164f6ea80f7b9df0ccf9ez").is_none());
        assert!(normalize_uuid("bc881e02-9216-4f6e-a80f-7b9df0ccf9e").is_none());
        assert!(normalize_uuid("bc881e0292164f6ea80f7b9df0ccf9e90").is_none());
    }

    #[test]
    fn validates_username() {
        assert!(is_username_valid("Steve"));
        assert!(is_username_valid("Alex_123"));
        assert!(!is_username_valid(""));
        assert!(!is_username_valid("ab"));
        assert!(!is_username_valid("abcdefghijklmnopq"));
        assert!(!is_username_valid("bad-name"));
        assert!(!is_username_valid("álex"));
    }

    #[test]
    fn validates_size() {
        assert_eq!(parse_size(Some("8")).expect("valid size"), 8);
        assert_eq!(parse_size(Some("512")).expect("valid size"), 512);
        assert!(parse_size(Some("abc")).is_err());
    }

    #[test]
    fn defaults_missing_size_to_even_minecraft_pixels() {
        assert_eq!(parse_size(None).expect("default size"), 184);
    }

    #[test]
    fn rounds_size_to_nearest_minecraft_pixel_multiple() {
        assert_eq!(parse_size(Some("180")).expect("valid size"), 184);
        assert_eq!(parse_size(Some("179")).expect("valid size"), 176);
        assert_eq!(parse_size(Some("511")).expect("valid size"), 512);
    }

    #[test]
    fn rejects_size_above_512() {
        assert!(parse_size(Some("513")).is_err());
    }

    #[test]
    fn rejects_size_below_8() {
        assert!(parse_size(Some("7")).is_err());
    }
}
