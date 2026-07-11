use std::fs;
use std::io::Cursor;
use std::path::Path;

use anyhow::{bail, Context, Result};
use image::{imageops::FilterType, DynamicImage, GenericImageView, RgbaImage};
use serde::{Deserialize, Serialize};

const SAMPLE_SIZE: u32 = 192;
const MAX_IMAGE_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
}

pub(crate) fn compare_images(request: &VisualDiffRequest) -> Result<VisualDiffResult> {
    let target = open_image(&request.target_path)?;
    let current = open_image(&request.current_path)?;
    compare_dynamic_images(&target, &current, request.target_rect, request.current_rect)
}

pub(crate) fn compare_target_with_png(
    target_path: &str,
    current_png: &[u8],
    target_rect: Option<PixelRect>,
    current_rect: Option<PixelRect>,
) -> Result<VisualDiffResult> {
    let target = open_image(target_path)?;
    let current = image::ImageReader::new(Cursor::new(current_png))
        .with_guessed_format()
        .context("无法识别真机截图格式")?
        .decode()
        .context("无法解码真机截图")?;
    compare_dynamic_images(&target, &current, target_rect, current_rect)
}

pub(crate) fn compare_pngs(
    target_png: &[u8],
    current_png: &[u8],
    target_rect: Option<PixelRect>,
    current_rect: Option<PixelRect>,
) -> Result<VisualDiffResult> {
    let decode = |bytes: &[u8]| {
        image::ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .context("无法识别真机截图格式")?
            .decode()
            .context("无法解码真机截图")
    };
    compare_dynamic_images(
        &decode(target_png)?,
        &decode(current_png)?,
        target_rect,
        current_rect,
    )
}

pub(crate) fn compare_dynamic_images(
    target: &DynamicImage,
    current: &DynamicImage,
    target_rect: Option<PixelRect>,
    current_rect: Option<PixelRect>,
) -> Result<VisualDiffResult> {
    let target_crop = crop(target, target_rect)?;
    let current_crop = crop(current, current_rect)?;
    let (target_width, target_height) = target_crop.dimensions();
    let (current_width, current_height) = current_crop.dimensions();
    let target_sample = target_crop
        .resize_exact(SAMPLE_SIZE, SAMPLE_SIZE, FilterType::Triangle)
        .to_rgba8();
    let current_sample = current_crop
        .resize_exact(SAMPLE_SIZE, SAMPLE_SIZE, FilterType::Triangle)
        .to_rgba8();
    let (color_error, alpha_error) = pixel_error(&target_sample, &current_sample);
    let edge_error = edge_error(&target_sample, &current_sample);
    let geometry_error = geometry_error(target_width, target_height, current_width, current_height);
    let visual_loss =
        round6(color_error * 0.52 + edge_error * 0.30 + geometry_error * 0.15 + alpha_error * 0.03);
    Ok(VisualDiffResult {
        target_width,
        target_height,
        current_width,
        current_height,
        mean_absolute_color_error: round6(color_error),
        edge_error: round6(edge_error),
        alpha_error: round6(alpha_error),
        geometry_error: round6(geometry_error),
        visual_loss,
        sample_size: SAMPLE_SIZE,
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

fn crop(image: &DynamicImage, rect: Option<PixelRect>) -> Result<DynamicImage> {
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

fn pixel_error(left: &RgbaImage, right: &RgbaImage) -> (f64, f64) {
    let mut color_sum = 0_u64;
    let mut alpha_sum = 0_u64;
    for (a, b) in left.pixels().zip(right.pixels()) {
        color_sum += a[0].abs_diff(b[0]) as u64;
        color_sum += a[1].abs_diff(b[1]) as u64;
        color_sum += a[2].abs_diff(b[2]) as u64;
        alpha_sum += a[3].abs_diff(b[3]) as u64;
    }
    let pixels = (left.width() * left.height()).max(1) as f64;
    (
        color_sum as f64 / (pixels * 3.0 * 255.0),
        alpha_sum as f64 / (pixels * 255.0),
    )
}

fn edge_error(left: &RgbaImage, right: &RgbaImage) -> f64 {
    let left_edges = edge_map(left);
    let right_edges = edge_map(right);
    let sum = left_edges
        .iter()
        .zip(right_edges.iter())
        .map(|(a, b)| a.abs_diff(*b) as u64)
        .sum::<u64>();
    sum as f64 / (left_edges.len().max(1) as f64 * 255.0)
}

fn edge_map(image: &RgbaImage) -> Vec<u8> {
    let width = image.width() as usize;
    let height = image.height() as usize;
    let mut gray = vec![0_u8; width * height];
    for (index, pixel) in image.pixels().enumerate() {
        gray[index] =
            ((pixel[0] as u16 * 77 + pixel[1] as u16 * 150 + pixel[2] as u16 * 29) >> 8) as u8;
    }
    let mut edges = vec![0_u8; gray.len()];
    for y in 1..height.saturating_sub(1) {
        for x in 1..width.saturating_sub(1) {
            let index = y * width + x;
            let dx = gray[index + 1].abs_diff(gray[index - 1]) as u16;
            let dy = gray[index + width].abs_diff(gray[index - width]) as u16;
            edges[index] = (dx + dy).min(255) as u8;
        }
    }
    edges
}

fn geometry_error(
    target_width: u32,
    target_height: u32,
    current_width: u32,
    current_height: u32,
) -> f64 {
    let width_scale = target_width.max(current_width).max(1) as f64;
    let height_scale = target_height.max(current_height).max(1) as f64;
    ((target_width.abs_diff(current_width) as f64 / width_scale)
        + (target_height.abs_diff(current_height) as f64 / height_scale))
        / 2.0
}

fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}
