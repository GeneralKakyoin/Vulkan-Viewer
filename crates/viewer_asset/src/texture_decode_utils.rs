use crate::DecodedRgbaImage;

#[derive(Debug, thiserror::Error)]
pub enum TextureDecodeError {
    #[error("unsupported image format")]
    Unsupported,
}

pub fn decode_texture_rgba8(bytes: &[u8]) -> Result<DecodedRgbaImage, TextureDecodeError> {
    if let Ok(img) = image::load_from_memory(bytes) {
        let (width, height) = image::GenericImageView::dimensions(&img);
        let rgba = img.to_rgba8().into_raw();
        return Ok(DecodedRgbaImage {
            width,
            height,
            rgba,
        });
    }

    if let Ok(j2k) = jpeg2k::Image::from_bytes(bytes)
        && let Ok(decoded) = image::DynamicImage::try_from(&j2k)
    {
        let rgba = decoded.to_rgba8();
        return Ok(DecodedRgbaImage {
            width: rgba.width(),
            height: rgba.height(),
            rgba: rgba.into_raw(),
        });
    }

    let jp2 = justjp2::decode(bytes).map_err(|_| TextureDecodeError::Unsupported)?;
    if jp2.components.is_empty() || jp2.width == 0 || jp2.height == 0 {
        return Err(TextureDecodeError::Unsupported);
    }

    let width = jp2.width as usize;
    let height = jp2.height as usize;
    let mut rgba = vec![0u8; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let r = sample_jp2_component_u8(&jp2.components, 0, x, y, width, height);
            let g = sample_jp2_component_u8(&jp2.components, 1, x, y, width, height);
            let b = sample_jp2_component_u8(&jp2.components, 2, x, y, width, height);
            let alpha = sample_jp2_component_u8(&jp2.components, 3, x, y, width, height);
            let idx = (y * width + x) * 4;
            rgba[idx] = r;
            rgba[idx + 1] = g;
            rgba[idx + 2] = b;
            rgba[idx + 3] = if jp2.components.len() >= 4 {
                alpha
            } else {
                255
            };
        }
    }

    Ok(DecodedRgbaImage {
        width: jp2.width,
        height: jp2.height,
        rgba,
    })
}

fn sample_jp2_component_u8(
    components: &[justjp2::Component],
    component_idx: usize,
    x: usize,
    y: usize,
    out_width: usize,
    out_height: usize,
) -> u8 {
    let component = components
        .get(component_idx)
        .or_else(|| components.first())
        .expect("jp2 components non-empty");
    let comp_width = component.width.max(1) as usize;
    let comp_height = component.height.max(1) as usize;
    let sx = (x * comp_width) / out_width.max(1);
    let sy = (y * comp_height) / out_height.max(1);
    let idx = sy.saturating_mul(comp_width).saturating_add(sx);
    let sample = *component.data.get(idx).unwrap_or(&0);
    let precision = component.precision.clamp(1, 31);
    let max = ((1i64 << precision) - 1).max(1);
    let normalized = if component.signed {
        let bias = 1i64 << (precision - 1);
        (i64::from(sample) + bias).clamp(0, max)
    } else {
        i64::from(sample).clamp(0, max)
    };
    ((normalized * 255) / max) as u8
}

pub fn decode_png_rgba8(bytes: &[u8]) -> anyhow::Result<DecodedRgbaImage> {
    decode_texture_rgba8(bytes).map_err(|e| anyhow::anyhow!(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_texture_rgba8_decodes_png() {
        let image = image::RgbaImage::from_raw(1, 1, vec![1, 2, 3, 255]).expect("valid image");
        let mut bytes = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .expect("encode png");

        let decoded = decode_texture_rgba8(&bytes).expect("png should decode");
        assert_eq!(decoded.width, 1);
        assert_eq!(decoded.height, 1);
        assert_eq!(decoded.rgba, vec![1, 2, 3, 255]);
    }

    #[test]
    fn decode_texture_rgba8_decodes_jpeg2000() {
        let bytes = include_bytes!(
            "..\\..\\..\\reference\\firestorm\\indra\\newview\\skins\\starlight\\themes\\mono_teal\\textures\\default_profile_picture.j2c"
        );
        let decoded = decode_texture_rgba8(bytes).expect("jpeg2000 should decode");
        assert!(decoded.width > 0);
        assert!(decoded.height > 0);
        assert_eq!(
            decoded.rgba.len(),
            (decoded.width as usize) * (decoded.height as usize) * 4
        );
    }

    #[test]
    fn decode_texture_rgba8_rejects_unsupported_bytes() {
        let err = decode_texture_rgba8(b"not-an-image").expect_err("must reject invalid bytes");
        assert!(matches!(err, TextureDecodeError::Unsupported));
    }
}
