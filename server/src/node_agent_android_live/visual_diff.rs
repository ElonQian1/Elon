use std::fs;
use std::io::Cursor;
use std::path::Path;

use anyhow::{bail, Context, Result};
use image::DynamicImage;
use serde::{Deserialize, Serialize};

#[path = "visual_score/mod.rs"]
mod visual_score;

pub(crate) use visual_score::types::{
    AdaptiveIconMask, AdaptiveIconMaskShape, VisualMask, VisualScoreProfile, VisualScoreReport,
};
use visual_score::{score_images, ScoreInput};

const MAX_IMAGE_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PixelRect {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VisualDiffRequest {
    pub(crate) target_path: String,
    pub(crate) current_path: String,
    pub(crate) target_rect: Option<PixelRect>,
    pub(crate) current_rect: Option<PixelRect>,
    #[serde(default)]
    pub(crate) projected_current_rect: Option<PixelRect>,
    #[serde(default)]
    pub(crate) mask: VisualMask,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VisualDiffResult {
    pub(crate) target_width: u32,
    pub(crate) target_height: u32,
    pub(crate) current_width: u32,
    pub(crate) current_height: u32,
    pub(crate) mean_absolute_color_error: f64,
    pub(crate) edge_error: f64,
    pub(crate) alpha_error: f64,
    pub(crate) geometry_error: f64,
    pub(crate) visual_loss: f64,
    pub(crate) sample_size: u32,
    pub(crate) score_report: VisualScoreReport,
}

pub(crate) fn compare_images(request: &VisualDiffRequest) -> Result<VisualDiffResult> {
    let target = open_image(&request.target_path)?;
    let current = open_image(&request.current_path)?;
    compare_dynamic_images_with_projection(
        &target,
        &current,
        request.target_rect,
        request.current_rect,
        request.projected_current_rect,
        &request.mask,
        VisualScoreProfile::default(),
    )
}

pub(crate) fn compare_target_with_png(
    target_path: &str,
    current_png: &[u8],
    target_rect: Option<PixelRect>,
    current_rect: Option<PixelRect>,
) -> Result<VisualDiffResult> {
    compare_target_with_png_projected(target_path, current_png, target_rect, current_rect, None)
}

pub(crate) fn compare_target_with_png_projected(
    target_path: &str,
    current_png: &[u8],
    target_rect: Option<PixelRect>,
    current_rect: Option<PixelRect>,
    projected_current_rect: Option<PixelRect>,
) -> Result<VisualDiffResult> {
    compare_target_with_png_projected_masked(
        target_path,
        current_png,
        target_rect,
        current_rect,
        projected_current_rect,
        &VisualMask::default(),
    )
}

pub(crate) fn compare_target_with_png_projected_masked(
    target_path: &str,
    current_png: &[u8],
    target_rect: Option<PixelRect>,
    current_rect: Option<PixelRect>,
    projected_current_rect: Option<PixelRect>,
    mask: &VisualMask,
) -> Result<VisualDiffResult> {
    let target = open_image(target_path)?;
    let current = decode_png(current_png)?;
    compare_dynamic_images_with_projection(
        &target,
        &current,
        target_rect,
        current_rect,
        projected_current_rect,
        mask,
        VisualScoreProfile::default(),
    )
}

pub(crate) fn compare_pngs(
    target_png: &[u8],
    current_png: &[u8],
    target_rect: Option<PixelRect>,
    current_rect: Option<PixelRect>,
) -> Result<VisualDiffResult> {
    // Both PNGs come from the Android display, so target_rect already lives in
    // the same coordinate space as current_rect and can be used as the expected
    // projection. Design images use compare_target_with_png_projected instead.
    compare_dynamic_images_with_projection(
        &decode_png(target_png)?,
        &decode_png(current_png)?,
        target_rect,
        current_rect,
        target_rect,
        &VisualMask::default(),
        VisualScoreProfile::default(),
    )
}

pub(crate) fn compare_dynamic_images(
    target: &DynamicImage,
    current: &DynamicImage,
    target_rect: Option<PixelRect>,
    current_rect: Option<PixelRect>,
) -> Result<VisualDiffResult> {
    compare_dynamic_images_with_projection(
        target,
        current,
        target_rect,
        current_rect,
        None,
        &VisualMask::default(),
        VisualScoreProfile::default(),
    )
}

pub(crate) fn compare_dynamic_images_with_projection(
    target: &DynamicImage,
    current: &DynamicImage,
    target_rect: Option<PixelRect>,
    current_rect: Option<PixelRect>,
    projected_current_rect: Option<PixelRect>,
    mask: &VisualMask,
    profile: VisualScoreProfile,
) -> Result<VisualDiffResult> {
    let scored = score_images(ScoreInput {
        target,
        current,
        target_rect,
        current_rect,
        projected_target_rect: projected_current_rect,
        mask,
        profile,
    })?;
    let report = scored.report;
    let geometry_error =
        (report.geometry.size_error_ratio + report.geometry.aspect_error_ratio) / 2.0;
    Ok(VisualDiffResult {
        target_width: scored.target_width,
        target_height: scored.target_height,
        current_width: scored.current_width,
        current_height: scored.current_height,
        mean_absolute_color_error: report.color.mean_absolute_error,
        edge_error: report.edge.error,
        alpha_error: report.color.alpha_error,
        geometry_error: round6(geometry_error),
        visual_loss: report.optimization_score,
        sample_size: report.comparison_width.max(report.comparison_height),
        score_report: report,
    })
}

fn open_image(path: &str) -> Result<DynamicImage> {
    let path = Path::new(path);
    let metadata =
        fs::metadata(path).with_context(|| format!("视觉比较图片不存在: {}", path.display()))?;
    if metadata.len() == 0 || metadata.len() > MAX_IMAGE_BYTES {
        bail!("视觉比较图片大小必须在 1..32MiB");
    }
    image::open(path).with_context(|| format!("无法解码图片: {}", path.display()))
}

fn decode_png(bytes: &[u8]) -> Result<DynamicImage> {
    image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .context("无法识别真机截图格式")?
        .decode()
        .context("无法解码真机截图")
}

fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

#[cfg(test)]
mod runtime_image_format_tests {
    use super::*;
    use image::{DynamicImage, ImageFormat};

    #[test]
    fn compares_process_runtime_webp_frames() {
        let mut encoded = Cursor::new(Vec::new());
        DynamicImage::new_rgba8(2, 2)
            .write_to(&mut encoded, ImageFormat::WebP)
            .expect("encode WebP fixture");
        let result = compare_pngs(encoded.get_ref(), encoded.get_ref(), None, None)
            .expect("decode and compare WebP runtime frame");
        assert_eq!((result.target_width, result.target_height), (2, 2));
        assert_eq!((result.current_width, result.current_height), (2, 2));
        assert_eq!(result.mean_absolute_color_error, 0.0);
    }
}
