use image::GenericImageView;

use crate::error::ViewerError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinFormat {
    Modern64x64,
    Legacy64x32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelVariant {
    Classic,
    Slim,
}

#[derive(Debug, Clone)]
pub struct SkinImage {
    pub width: u32,
    pub height: u32,
    pub format: SkinFormat,
    pub model: ModelVariant,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct CapeImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub fn validate_dimensions(width: u32, height: u32) -> Result<SkinFormat, ViewerError> {
    match (width, height) {
        (64, 64) => Ok(SkinFormat::Modern64x64),
        (64, 32) => Ok(SkinFormat::Legacy64x32),
        _ => Err(ViewerError::UnsupportedSkinDimensions(width, height)),
    }
}

pub fn decode_png(bytes: &[u8]) -> Result<SkinImage, ViewerError> {
    let image = image::load_from_memory(bytes).map_err(|_| ViewerError::InvalidPng)?;
    let (width, height) = image.dimensions();
    let format = validate_dimensions(width, height)?;
    Ok(SkinImage {
        width,
        height,
        format,
        model: ModelVariant::Classic,
        rgba: image.to_rgba8().into_raw(),
    })
}

pub fn validate_cape_dimensions(width: u32, height: u32) -> Result<(), ViewerError> {
    match (width, height) {
        (64, 32) | (64, 64) => Ok(()),
        _ => Err(ViewerError::UnsupportedCapeDimensions(width, height)),
    }
}

pub fn decode_cape_png(bytes: &[u8]) -> Result<CapeImage, ViewerError> {
    let image = image::load_from_memory(bytes).map_err(|_| ViewerError::InvalidCapePng)?;
    let (width, height) = image.dimensions();
    validate_cape_dimensions(width, height)?;
    Ok(CapeImage {
        width,
        height,
        rgba: image.to_rgba8().into_raw(),
    })
}

pub fn default_skin() -> SkinImage {
    let width = 64;
    let height = 64;
    let mut rgba = vec![0_u8; width * height * 4];
    fill_rect(&mut rgba, width, 8, 8, 8, 8, [210, 150, 105, 255]);
    fill_rect(&mut rgba, width, 8, 8, 2, 2, [60, 40, 35, 255]);
    fill_rect(&mut rgba, width, 14, 8, 2, 2, [60, 40, 35, 255]);
    fill_rect(&mut rgba, width, 10, 14, 4, 1, [120, 45, 45, 255]);

    fill_rect(&mut rgba, width, 20, 20, 8, 12, [45, 100, 190, 255]);
    fill_rect(&mut rgba, width, 16, 20, 4, 12, [45, 100, 190, 255]);
    fill_rect(&mut rgba, width, 28, 20, 4, 12, [45, 100, 190, 255]);
    fill_rect(&mut rgba, width, 32, 20, 8, 12, [45, 100, 190, 255]);

    fill_rect(&mut rgba, width, 44, 20, 4, 12, [210, 150, 105, 255]);
    fill_rect(&mut rgba, width, 36, 52, 4, 12, [210, 150, 105, 255]);
    fill_rect(&mut rgba, width, 4, 20, 4, 12, [45, 65, 150, 255]);
    fill_rect(&mut rgba, width, 20, 52, 4, 12, [45, 65, 150, 255]);

    fill_overlay_rect(&mut rgba, width, 40, 8, 8, 8, [30, 30, 30, 180]);
    fill_overlay_rect(&mut rgba, width, 20, 36, 8, 12, [20, 30, 70, 100]);
    fill_overlay_rect(&mut rgba, width, 44, 36, 4, 12, [20, 30, 70, 90]);
    fill_overlay_rect(&mut rgba, width, 52, 52, 4, 12, [20, 30, 70, 90]);
    fill_overlay_rect(&mut rgba, width, 4, 36, 4, 12, [20, 20, 90, 90]);
    fill_overlay_rect(&mut rgba, width, 4, 52, 4, 12, [20, 20, 90, 90]);

    SkinImage {
        width: width as u32,
        height: height as u32,
        format: SkinFormat::Modern64x64,
        model: ModelVariant::Classic,
        rgba,
    }
}

fn fill_rect(
    rgba: &mut [u8],
    width: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: [u8; 4],
) {
    for py in y..y + h {
        for px in x..x + w {
            let offset = (py * width + px) * 4;
            rgba[offset..offset + 4].copy_from_slice(&color);
        }
    }
}

fn fill_overlay_rect(
    rgba: &mut [u8],
    width: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: [u8; 4],
) {
    fill_rect(rgba, width, x, y, w, h, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_64x64() {
        assert_eq!(
            validate_dimensions(64, 64).expect("valid dimensions"),
            SkinFormat::Modern64x64
        );
    }

    #[test]
    fn accepts_64x32() {
        assert_eq!(
            validate_dimensions(64, 32).expect("valid dimensions"),
            SkinFormat::Legacy64x32
        );
    }

    #[test]
    fn rejects_unsupported_size() {
        assert!(validate_dimensions(32, 32).is_err());
    }

    #[test]
    fn accepts_official_cape_size() {
        assert!(validate_cape_dimensions(64, 32).is_ok());
    }

    #[test]
    fn rejects_unsupported_cape_size() {
        assert!(validate_cape_dimensions(32, 32).is_err());
    }
}
