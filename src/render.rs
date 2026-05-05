use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::{ColorType, GenericImageView, ImageEncoder, Rgba, RgbaImage};

use crate::error::AppError;

const HEAD_X: u32 = 8;
const HEAD_Y: u32 = 8;
const OVERLAY_X: u32 = 40;
const OVERLAY_Y: u32 = 8;
const HEAD_SIZE: u32 = 8;

pub fn render_helm_png(skin_png: &[u8], size: u32) -> Result<Vec<u8>, AppError> {
    let skin = image::load_from_memory(skin_png).map_err(|err| {
        tracing::warn!(error = %err, "failed to decode skin png");
        AppError::InvalidSkinImage
    })?;

    if skin.width() < 16 || skin.height() < 16 {
        return Err(AppError::InvalidSkinImage);
    }

    let skin = skin.to_rgba8();
    let mut head = crop_exact(&skin, HEAD_X, HEAD_Y, HEAD_SIZE, HEAD_SIZE)?;

    if skin.width() >= OVERLAY_X + HEAD_SIZE && skin.height() >= 64 && has_visible_overlay(&skin) {
        composite_overlay(&mut head, &skin)?;
    }

    let resized = image::imageops::resize(&head, size, size, FilterType::Nearest);
    encode_png(&resized)
}

fn crop_exact(
    image: &RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<RgbaImage, AppError> {
    if x.saturating_add(width) > image.width() || y.saturating_add(height) > image.height() {
        return Err(AppError::InvalidSkinImage);
    }

    Ok(image.view(x, y, width, height).to_image())
}

fn has_visible_overlay(skin: &RgbaImage) -> bool {
    (OVERLAY_Y..OVERLAY_Y + HEAD_SIZE).any(|y| {
        (OVERLAY_X..OVERLAY_X + HEAD_SIZE)
            .any(|x| skin.get_pixel(x, y).0.get(3).copied().unwrap_or(0) != 0)
    })
}

fn composite_overlay(base: &mut RgbaImage, skin: &RgbaImage) -> Result<(), AppError> {
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

fn encode_png(image: &RgbaImage) -> Result<Vec<u8>, AppError> {
    let mut bytes = Vec::new();
    let encoder = PngEncoder::new(&mut bytes);
    encoder
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ColorType::Rgba8.into(),
        )
        .map_err(|err| {
            tracing::error!(error = %err, "failed to encode rendered png");
            AppError::Internal
        })?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageBuffer;

    fn encode_test_png(image: &RgbaImage) -> Vec<u8> {
        encode_png(image).expect("test image encodes")
    }

    #[test]
    fn renders_64x64_overlay_with_alpha() {
        let mut skin = ImageBuffer::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
        for y in HEAD_Y..HEAD_Y + HEAD_SIZE {
            for x in HEAD_X..HEAD_X + HEAD_SIZE {
                skin.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        for y in OVERLAY_Y..OVERLAY_Y + HEAD_SIZE {
            for x in OVERLAY_X..OVERLAY_X + HEAD_SIZE {
                skin.put_pixel(x, y, Rgba([0, 0, 255, 128]));
            }
        }

        let rendered = render_helm_png(&encode_test_png(&skin), 8).expect("renders helm");
        let output = image::load_from_memory(&rendered)
            .expect("output decodes")
            .to_rgba8();
        let pixel = *output.get_pixel(0, 0);

        assert_ne!(pixel, Rgba([255, 0, 0, 255]));
        assert_ne!(pixel, Rgba([0, 0, 255, 128]));
    }

    #[test]
    fn renders_64x32_without_overlay() {
        let mut skin = ImageBuffer::from_pixel(64, 32, Rgba([0, 0, 0, 0]));
        for y in HEAD_Y..HEAD_Y + HEAD_SIZE {
            for x in HEAD_X..HEAD_X + HEAD_SIZE {
                skin.put_pixel(x, y, Rgba([20, 40, 60, 255]));
            }
        }
        for y in OVERLAY_Y..OVERLAY_Y + HEAD_SIZE {
            for x in OVERLAY_X..OVERLAY_X + HEAD_SIZE {
                skin.put_pixel(x, y, Rgba([255, 255, 0, 255]));
            }
        }

        let rendered = render_helm_png(&encode_test_png(&skin), 8).expect("renders helm");
        let output = image::load_from_memory(&rendered)
            .expect("output decodes")
            .to_rgba8();

        assert_eq!(*output.get_pixel(0, 0), Rgba([20, 40, 60, 255]));
    }

    #[test]
    fn resizes_with_nearest_neighbor() {
        let mut skin = ImageBuffer::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
        for y in HEAD_Y..HEAD_Y + HEAD_SIZE {
            for x in HEAD_X..HEAD_X + HEAD_SIZE {
                let color = if x < HEAD_X + 4 {
                    Rgba([255, 0, 0, 255])
                } else {
                    Rgba([0, 255, 0, 255])
                };
                skin.put_pixel(x, y, color);
            }
        }

        let rendered = render_helm_png(&encode_test_png(&skin), 16).expect("renders helm");
        let output = image::load_from_memory(&rendered)
            .expect("output decodes")
            .to_rgba8();

        assert_eq!(output.dimensions(), (16, 16));
        assert_eq!(*output.get_pixel(0, 0), Rgba([255, 0, 0, 255]));
        assert_eq!(*output.get_pixel(7, 0), Rgba([255, 0, 0, 255]));
        assert_eq!(*output.get_pixel(8, 0), Rgba([0, 255, 0, 255]));
    }

    #[test]
    fn rejects_invalid_image() {
        let err = render_helm_png(b"not a png", 128).expect_err("invalid image fails");
        assert!(matches!(err, AppError::InvalidSkinImage));
    }
}
