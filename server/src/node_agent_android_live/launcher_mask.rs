use std::io::Cursor;

use anyhow::{bail, Context, Result};
use image::{DynamicImage, ImageFormat, RgbaImage};
use serde_json::{json, Value};

use super::broker::LiveUiSession;
use super::frame_artifact::{
    persist_launcher_mask_artifact, validate_launcher_crop_artifact, ReusableFrameArtifact,
};
use super::visual_diff::{compare_dynamic_images, AdaptiveIconMaskShape, VisualDiffResult};

pub(crate) fn render(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let reusable: ReusableFrameArtifact = serde_json::from_value(
        arguments
            .get("currentArtifact")
            .cloned()
            .context("缺少 currentArtifact")?,
    )
    .context("currentArtifact 参数无效")?;
    let original_path = validate_launcher_crop_artifact(session, &reusable)?;
    let original = image::open(&original_path).context("无法读取 Launcher iconCrop")?;
    let inset = arguments
        .get("safeZoneInsetFraction")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    if !inset.is_finite() || !(0.0..=0.25).contains(&inset) {
        bail!("safeZoneInsetFraction 必须在 0..0.25");
    }
    let shapes = requested_shapes(arguments)?;
    let mut variants = Vec::new();
    for shape in shapes {
        let masked = apply_mask(&original, shape, inset);
        let png = encode_png(&masked)?;
        let artifact = persist_launcher_mask_artifact(session, &png, shape_slug(shape))?;
        let diff =
            compare_dynamic_images(&original, &DynamicImage::ImageRgba8(masked), None, None)?;
        variants.push(json!({
            "shape": shape_name(shape),
            "maskedArtifact": artifact,
            "diff": compact_diff(&diff),
            "scoreReport": diff.score_report,
        }));
    }
    Ok(json!({
        "source": {
            "source": reusable.source,
            "path": original_path,
            "sha256": reusable.sha256,
        },
        "sameDeviceId": session.device_id,
        "sameRuntimeSessionId": session.id,
        "safeZoneInsetFraction": inset,
        "variants": variants,
        "maskModel": "ANDROID_ADAPTIVE_ICON_PUBLIC_SHAPES_V1"
    }))
}

fn requested_shapes(arguments: &Value) -> Result<Vec<AdaptiveIconMaskShape>> {
    let Some(values) = arguments.get("shapes").and_then(Value::as_array) else {
        return Ok(vec![
            AdaptiveIconMaskShape::Circle,
            AdaptiveIconMaskShape::RoundedSquare,
            AdaptiveIconMaskShape::Squircle,
        ]);
    };
    if values.is_empty() || values.len() > 3 {
        bail!("shapes 数量必须在 1..3");
    }
    let mut shapes = Vec::new();
    for value in values {
        let shape = match value.as_str() {
            Some("CIRCLE") => AdaptiveIconMaskShape::Circle,
            Some("ROUNDED_SQUARE") => AdaptiveIconMaskShape::RoundedSquare,
            Some("SQUIRCLE") => AdaptiveIconMaskShape::Squircle,
            _ => bail!("shapes 只支持 CIRCLE、ROUNDED_SQUARE、SQUIRCLE"),
        };
        if !shapes
            .iter()
            .any(|candidate| shape_name(*candidate) == shape_name(shape))
        {
            shapes.push(shape);
        }
    }
    Ok(shapes)
}

fn apply_mask(image: &DynamicImage, shape: AdaptiveIconMaskShape, inset: f64) -> RgbaImage {
    let mut rgba = image.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    let half_w = width as f64 / 2.0;
    let half_h = height as f64 / 2.0;
    let radius_x = half_w * (1.0 - inset * 2.0);
    let radius_y = half_h * (1.0 - inset * 2.0);
    for y in 0..height {
        for x in 0..width {
            let nx = ((x as f64 + 0.5) - half_w).abs() / radius_x.max(0.5);
            let ny = ((y as f64 + 0.5) - half_h).abs() / radius_y.max(0.5);
            let inside = match shape {
                AdaptiveIconMaskShape::Circle => nx * nx + ny * ny <= 1.0,
                AdaptiveIconMaskShape::RoundedSquare => {
                    let corner = 0.38;
                    let qx = (nx - (1.0 - corner)).max(0.0);
                    let qy = (ny - (1.0 - corner)).max(0.0);
                    nx <= 1.0 && ny <= 1.0 && qx * qx + qy * qy <= corner * corner
                }
                AdaptiveIconMaskShape::Squircle => nx.powi(4) + ny.powi(4) <= 1.0,
            };
            if !inside {
                rgba.get_pixel_mut(x, y).0[3] = 0;
            }
        }
    }
    rgba
}

fn encode_png(image: &RgbaImage) -> Result<Vec<u8>> {
    let mut output = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image.clone())
        .write_to(&mut output, ImageFormat::Png)
        .context("无法编码 adaptive mask PNG")?;
    Ok(output.into_inner())
}

fn compact_diff(diff: &VisualDiffResult) -> Value {
    json!({
        "visualLoss": diff.visual_loss,
        "meanAbsoluteColorError": diff.mean_absolute_color_error,
        "edgeError": diff.edge_error,
        "alphaError": diff.alpha_error,
        "geometryError": diff.geometry_error,
        "sampleSize": diff.sample_size,
    })
}

fn shape_name(shape: AdaptiveIconMaskShape) -> &'static str {
    match shape {
        AdaptiveIconMaskShape::Circle => "CIRCLE",
        AdaptiveIconMaskShape::RoundedSquare => "ROUNDED_SQUARE",
        AdaptiveIconMaskShape::Squircle => "SQUIRCLE",
    }
}

fn shape_slug(shape: AdaptiveIconMaskShape) -> &'static str {
    match shape {
        AdaptiveIconMaskShape::Circle => "circle",
        AdaptiveIconMaskShape::RoundedSquare => "rounded-square",
        AdaptiveIconMaskShape::Squircle => "squircle",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn all_three_masks_produce_distinct_visible_coverage() {
        let source =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(100, 100, Rgba([20, 40, 60, 255])));
        let visible = |shape| {
            apply_mask(&source, shape, 0.0)
                .pixels()
                .filter(|pixel| pixel.0[3] > 0)
                .count()
        };
        let circle = visible(AdaptiveIconMaskShape::Circle);
        let rounded = visible(AdaptiveIconMaskShape::RoundedSquare);
        let squircle = visible(AdaptiveIconMaskShape::Squircle);
        assert!(circle < rounded);
        assert!(circle < squircle);
        assert_ne!(rounded, squircle);
    }
}
