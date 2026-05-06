use base64::Engine;
use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::{ColorType, GenericImageView, ImageEncoder, Rgba, RgbaImage};

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

const HEAD_X: u32 = 8;
const HEAD_Y: u32 = 8;
const OVERLAY_X: u32 = 40;
const OVERLAY_Y: u32 = 8;
const HEAD_SIZE: u32 = 8;

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
    let rgba = image.to_rgba8().into_raw();
    let model = detect_model_variant(width, height, &rgba);
    Ok(SkinImage {
        width,
        height,
        format,
        model,
        rgba,
    })
}

fn detect_model_variant(width: u32, height: u32, rgba: &[u8]) -> ModelVariant {
    if width != 64 || height != 64 {
        return ModelVariant::Classic;
    }

    if classic_arm_only_columns_are_transparent(rgba, width, 54, 20)
        && classic_arm_only_columns_are_transparent(rgba, width, 46, 52)
    {
        ModelVariant::Slim
    } else {
        ModelVariant::Classic
    }
}

fn classic_arm_only_columns_are_transparent(rgba: &[u8], width: u32, x: u32, y: u32) -> bool {
    let mut transparent = 0;
    let mut total = 0;

    for py in y..y + 12 {
        for px in x..x + 2 {
            let offset = ((py * width + px) * 4 + 3) as usize;
            total += 1;
            if rgba.get(offset).copied().unwrap_or(255) == 0 {
                transparent += 1;
            }
        }
    }

    transparent >= total - 2
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

pub fn render_head_png_data_url(
    skin: &SkinImage,
    include_overlay: bool,
    size: u32,
) -> Result<String, ViewerError> {
    let source =
        RgbaImage::from_raw(skin.width, skin.height, skin.rgba.clone()).ok_or_else(|| {
            ViewerError::InvalidTextureData {
                width: skin.width,
                height: skin.height,
                actual_len: skin.rgba.len(),
            }
        })?;
    let mut head = crop_exact(&source, HEAD_X, HEAD_Y, HEAD_SIZE, HEAD_SIZE)?;

    if include_overlay && skin.height >= 64 && has_visible_overlay(&source) {
        composite_overlay(&mut head, &source)?;
    }

    let resized = image::imageops::resize(&head, size, size, FilterType::Nearest);
    let png = encode_png(&resized)?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
    Ok(format!("data:image/png;base64,{encoded}"))
}

fn crop_exact(
    image: &RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<RgbaImage, ViewerError> {
    if x.saturating_add(width) > image.width() || y.saturating_add(height) > image.height() {
        return Err(ViewerError::InvalidPng);
    }

    Ok(image.view(x, y, width, height).to_image())
}

fn has_visible_overlay(skin: &RgbaImage) -> bool {
    (OVERLAY_Y..OVERLAY_Y + HEAD_SIZE).any(|y| {
        (OVERLAY_X..OVERLAY_X + HEAD_SIZE)
            .any(|x| skin.get_pixel(x, y).0.get(3).copied().unwrap_or(0) != 0)
    })
}

fn composite_overlay(base: &mut RgbaImage, skin: &RgbaImage) -> Result<(), ViewerError> {
    let overlay = crop_exact(skin, OVERLAY_X, OVERLAY_Y, HEAD_SIZE, HEAD_SIZE)?;
    for y in 0..HEAD_SIZE {
        for x in 0..HEAD_SIZE {
            let top = *overlay.get_pixel(x, y);
            let bottom = *base.get_pixel(x, y);
            base.put_pixel(x, y, alpha_blend(bottom, top));
        }
    }
    Ok(())
}

fn alpha_blend(bottom: Rgba<u8>, top: Rgba<u8>) -> Rgba<u8> {
    let src_a = u32::from(top[3]);
    if src_a == 0 {
        return bottom;
    }
    if src_a == 255 {
        return top;
    }

    let dst_a = u32::from(bottom[3]);
    let out_a = src_a + ((dst_a * (255 - src_a) + 127) / 255);
    if out_a == 0 {
        return Rgba([0, 0, 0, 0]);
    }

    let mut out = [0_u8; 4];
    for channel in 0..3 {
        let src = u32::from(top[channel]);
        let dst = u32::from(bottom[channel]);
        let premul = src * src_a * 255 + dst * dst_a * (255 - src_a);
        let value = (premul + (out_a * 255 / 2)) / (out_a * 255);
        out[channel] = value.min(255) as u8;
    }
    out[3] = out_a.min(255) as u8;
    Rgba(out)
}

fn encode_png(image: &RgbaImage) -> Result<Vec<u8>, ViewerError> {
    let mut bytes = Vec::new();
    let encoder = PngEncoder::new(&mut bytes);
    encoder
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ColorType::Rgba8.into(),
        )
        .map_err(|_| ViewerError::PngEncode)?;
    Ok(bytes)
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
    use base64::Engine;

    fn decode_data_url(data_url: &str) -> RgbaImage {
        let encoded = data_url
            .strip_prefix("data:image/png;base64,")
            .expect("png data url prefix");
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .expect("base64 decodes");
        image::load_from_memory(&bytes)
            .expect("png decodes")
            .to_rgba8()
    }

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

    #[test]
    fn detects_slim_skin_from_transparent_classic_arm_columns() {
        let mut rgba = opaque_rgba(64, 64);
        clear_alpha_rect(&mut rgba, 64, 54, 20, 2, 12);
        clear_alpha_rect(&mut rgba, 64, 46, 52, 2, 12);

        assert_eq!(detect_model_variant(64, 64, &rgba), ModelVariant::Slim);
    }

    #[test]
    fn opaque_arm_columns_default_to_classic_skin() {
        let rgba = opaque_rgba(64, 64);

        assert_eq!(detect_model_variant(64, 64, &rgba), ModelVariant::Classic);
    }

    #[test]
    fn legacy_skin_detection_defaults_to_classic() {
        let rgba = opaque_rgba(64, 32);

        assert_eq!(detect_model_variant(64, 32, &rgba), ModelVariant::Classic);
    }

    #[test]
    fn exports_flat_head_png_at_requested_size() {
        let mut skin = default_skin();
        fill_rect(
            &mut skin.rgba,
            skin.width as usize,
            8,
            8,
            8,
            8,
            [10, 20, 30, 255],
        );
        fill_rect(
            &mut skin.rgba,
            skin.width as usize,
            40,
            8,
            8,
            8,
            [0, 0, 0, 0],
        );

        let data_url = render_head_png_data_url(&skin, true, 180).expect("exports head");
        let output = decode_data_url(&data_url);

        assert_eq!(output.dimensions(), (180, 180));
        assert_eq!(*output.get_pixel(0, 0), Rgba([10, 20, 30, 255]));
    }

    #[test]
    fn exported_head_respects_overlay_toggle() {
        let mut skin = default_skin();
        fill_rect(
            &mut skin.rgba,
            skin.width as usize,
            8,
            8,
            8,
            8,
            [100, 0, 0, 255],
        );
        fill_rect(
            &mut skin.rgba,
            skin.width as usize,
            40,
            8,
            8,
            8,
            [0, 100, 0, 255],
        );

        let with_overlay = decode_data_url(
            &render_head_png_data_url(&skin, true, 8).expect("exports with overlay"),
        );
        let without_overlay = decode_data_url(
            &render_head_png_data_url(&skin, false, 8).expect("exports without overlay"),
        );

        assert_eq!(*with_overlay.get_pixel(0, 0), Rgba([0, 100, 0, 255]));
        assert_eq!(*without_overlay.get_pixel(0, 0), Rgba([100, 0, 0, 255]));
    }

    fn opaque_rgba(width: u32, height: u32) -> Vec<u8> {
        let mut rgba = vec![255; (width * height * 4) as usize];
        for pixel in rgba.chunks_exact_mut(4) {
            pixel[0] = 120;
            pixel[1] = 80;
            pixel[2] = 40;
        }
        rgba
    }

    fn clear_alpha_rect(rgba: &mut [u8], width: u32, x: u32, y: u32, w: u32, h: u32) {
        for py in y..y + h {
            for px in x..x + w {
                let offset = ((py * width + px) * 4 + 3) as usize;
                rgba[offset] = 0;
            }
        }
    }
}
