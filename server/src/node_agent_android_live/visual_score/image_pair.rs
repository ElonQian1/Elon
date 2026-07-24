use anyhow::{bail, Result};
use image::{imageops, DynamicImage, GenericImageView, Rgba, RgbaImage};

use super::super::PixelRect;
use super::types::{AdaptiveIconMaskShape, VisualMask};

const MAX_COMPARE_SIDE: u32 = 640;

pub(super) struct PreparedImagePair {
    pub(super) target: RgbaImage,
    pub(super) current: RgbaImage,
    pub(super) eligible: Vec<bool>,
}

pub(super) fn crop(image: &DynamicImage, rect: Option<PixelRect>) -> Result<DynamicImage> {
    let (width, height) = image.dimensions();
    let Some(rect) = rect else {
        return Ok(image.clone());
    };
    let left = rect.left.max(0) as u32;
    let top = rect.top.max(0) as u32;
    let right = (rect.right.max(0) as u32).min(width);
    let bottom = (rect.bottom.max(0) as u32).min(height);
    if right <= left || bottom <= top {
        bail!("视觉比较裁剪区域为空或超出图片范围");
    }
    Ok(image.crop_imm(left, top, right - left, bottom - top))
}

pub(super) fn prepare_pair(
    target: &DynamicImage,
    current: &DynamicImage,
    projected_target_rect: PixelRect,
    current_rect: PixelRect,
    mask: &VisualMask,
) -> Result<PreparedImagePair> {
    let target_size = rect_size(projected_target_rect)?;
    let current_size = rect_size(current_rect)?;
    let max_width = target_size.0.max(current_size.0).max(1);
    let max_height = target_size.1.max(current_size.1).max(1);
    let scale = (MAX_COMPARE_SIDE as f64 / max_width.max(max_height) as f64).min(1.0);
    let scaled_target = scaled_size(target_size, scale);
    let scaled_current = scaled_size(current_size, scale);
    let canvas_width = scaled_target.0.max(scaled_current.0).max(1);
    let canvas_height = scaled_target.1.max(scaled_current.1).max(1);

    let target = letterbox(target, scaled_target, canvas_width, canvas_height);
    let current = letterbox(current, scaled_current, canvas_width, canvas_height);
    let mut eligible = build_eligibility(canvas_width, canvas_height, mask, scale);
    apply_target_alpha_eligibility(&target, &mut eligible);
    Ok(PreparedImagePair {
        target,
        current,
        eligible,
    })
}

fn rect_size(rect: PixelRect) -> Result<(u32, u32)> {
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 0 || height <= 0 {
        bail!("视觉比较投影区域为空");
    }
    Ok((width as u32, height as u32))
}

fn scaled_size(size: (u32, u32), scale: f64) -> (u32, u32) {
    (
        (size.0 as f64 * scale).round().max(1.0) as u32,
        (size.1 as f64 * scale).round().max(1.0) as u32,
    )
}

fn letterbox(
    image: &DynamicImage,
    scaled: (u32, u32),
    canvas_width: u32,
    canvas_height: u32,
) -> RgbaImage {
    let resized = image.resize(scaled.0, scaled.1, imageops::FilterType::Triangle);
    let mut canvas = RgbaImage::from_pixel(canvas_width, canvas_height, Rgba([0, 0, 0, 0]));
    let x = (canvas_width - resized.width()) / 2;
    let y = (canvas_height - resized.height()) / 2;
    imageops::overlay(&mut canvas, &resized.to_rgba8(), i64::from(x), i64::from(y));
    canvas
}

fn build_eligibility(width: u32, height: u32, mask: &VisualMask, scale: f64) -> Vec<bool> {
    let mut eligible = vec![true; (width * height) as usize];
    if let Some(adaptive) = mask.adaptive_icon_mask {
        let inset = adaptive.safe_zone_inset_fraction.clamp(0.0, 0.25);
        let half_w = width as f64 / 2.0;
        let half_h = height as f64 / 2.0;
        let radius_x = half_w * (1.0 - inset * 2.0);
        let radius_y = half_h * (1.0 - inset * 2.0);
        for y in 0..height {
            for x in 0..width {
                let nx = ((x as f64 + 0.5) - half_w).abs() / radius_x.max(0.5);
                let ny = ((y as f64 + 0.5) - half_h).abs() / radius_y.max(0.5);
                let inside = match adaptive.shape {
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
                    eligible[(y * width + x) as usize] = false;
                }
            }
        }
    }
    for rect in &mask.exclude_rects {
        let left = (rect.left.max(0) as f64 * scale).round() as u32;
        let top = (rect.top.max(0) as f64 * scale).round() as u32;
        let right = (rect.right.max(0) as f64 * scale).round() as u32;
        let bottom = (rect.bottom.max(0) as f64 * scale).round() as u32;
        for y in top.min(height)..bottom.min(height) {
            for x in left.min(width)..right.min(width) {
                eligible[(y * width + x) as usize] = false;
            }
        }
    }
    eligible
}

fn apply_target_alpha_eligibility(target: &RgbaImage, eligible: &mut [bool]) {
    for (index, pixel) in target.pixels().enumerate() {
        if pixel[3] == 0 {
            if let Some(value) = eligible.get_mut(index) {
                *value = false;
            }
        }
    }
}
