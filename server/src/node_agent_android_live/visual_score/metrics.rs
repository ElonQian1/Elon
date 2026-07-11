use image::RgbaImage;

use super::super::PixelRect;
use super::types::{
    ColorMetrics, CoverageMetrics, EdgeMetrics, GeometryMetrics, PerceptualMetrics, PositionMetrics,
};

pub(super) fn geometry_metrics(projected_target: PixelRect, current: PixelRect) -> GeometryMetrics {
    let target_width = rect_width(projected_target);
    let target_height = rect_height(projected_target);
    let current_width = rect_width(current);
    let current_height = rect_height(current);
    let width_error = (target_width - current_width).abs();
    let height_error = (target_height - current_height).abs();
    let width_ratio = width_error / target_width.max(current_width).max(1.0);
    let height_ratio = height_error / target_height.max(current_height).max(1.0);
    let target_aspect = target_width / target_height.max(1.0);
    let current_aspect = current_width / current_height.max(1.0);
    let aspect_error = (target_aspect - current_aspect).abs()
        / target_aspect.max(current_aspect).max(f64::EPSILON);
    GeometryMetrics {
        width_error_px: round6(width_error),
        height_error_px: round6(height_error),
        size_error_ratio: round6((width_ratio + height_ratio) / 2.0),
        aspect_error_ratio: round6(aspect_error),
    }
}

pub(super) fn position_metrics(projected_target: PixelRect, current: PixelRect) -> PositionMetrics {
    let left = (projected_target.left - current.left).unsigned_abs() as f64;
    let top = (projected_target.top - current.top).unsigned_abs() as f64;
    let right = (projected_target.right - current.right).unsigned_abs() as f64;
    let bottom = (projected_target.bottom - current.bottom).unsigned_abs() as f64;
    let target_center = (
        (projected_target.left + projected_target.right) as f64 / 2.0,
        (projected_target.top + projected_target.bottom) as f64 / 2.0,
    );
    let current_center = (
        (current.left + current.right) as f64 / 2.0,
        (current.top + current.bottom) as f64 / 2.0,
    );
    let center = ((target_center.0 - current_center.0).powi(2)
        + (target_center.1 - current_center.1).powi(2))
    .sqrt();
    PositionMetrics {
        left_error_px: round6(left),
        top_error_px: round6(top),
        right_error_px: round6(right),
        bottom_error_px: round6(bottom),
        center_error_px: round6(center),
        max_edge_error_px: round6(left.max(top).max(right).max(bottom)),
    }
}

pub(super) fn pixel_metrics(
    target: &RgbaImage,
    current: &RgbaImage,
    eligible: &[bool],
) -> (
    ColorMetrics,
    EdgeMetrics,
    PerceptualMetrics,
    CoverageMetrics,
) {
    let mut color_errors = Vec::new();
    let mut delta_errors = Vec::new();
    let mut color_sum = 0.0;
    let mut delta_sum = 0.0;
    let mut alpha_sum = 0.0;
    let mut luminance_sum = 0.0;
    let mut compared = 0_u64;
    let mut eligible_pixels = 0_u64;

    for (index, (left, right)) in target.pixels().zip(current.pixels()).enumerate() {
        if !eligible.get(index).copied().unwrap_or(false) {
            continue;
        }
        eligible_pixels += 1;
        if left[3] == 0 || right[3] == 0 {
            continue;
        }
        let pixel_error = (left[0].abs_diff(right[0]) as f64
            + left[1].abs_diff(right[1]) as f64
            + left[2].abs_diff(right[2]) as f64)
            / (3.0 * 255.0);
        color_errors.push(pixel_error);
        color_sum += pixel_error;
        let delta_e = delta_e_76(srgb_to_lab(left), srgb_to_lab(right));
        delta_errors.push(delta_e);
        delta_sum += delta_e;
        alpha_sum += left[3].abs_diff(right[3]) as f64 / 255.0;
        luminance_sum += (luminance(left) - luminance(right)).abs();
        compared += 1;
    }

    color_errors.sort_by(f64::total_cmp);
    delta_errors.sort_by(f64::total_cmp);
    let denominator = compared.max(1) as f64;
    let mean_color = color_sum / denominator;
    let p95_color = percentile(&color_errors, 0.95);
    let mean_delta_e = delta_sum / denominator;
    let p95_delta_e = percentile(&delta_errors, 0.95);
    let alpha_error = alpha_sum / denominator;
    let luminance_error = luminance_sum / denominator;
    let (edge_similarity, edge_error) = edge_metrics(target, current, eligible);
    let structural_error = ((luminance_error + edge_error) / 2.0).clamp(0.0, 1.0);
    let coverage_ratio = compared as f64 / eligible_pixels.max(1) as f64;

    (
        ColorMetrics {
            mean_absolute_error: round6(mean_color),
            p95_absolute_error: round6(p95_color),
            mean_delta_e: round6(mean_delta_e),
            p95_delta_e: round6(p95_delta_e),
            alpha_error: round6(alpha_error),
        },
        EdgeMetrics {
            similarity: round6(edge_similarity),
            error: round6(edge_error),
        },
        PerceptualMetrics {
            luminance_error: round6(luminance_error),
            structural_error: round6(structural_error),
        },
        CoverageMetrics {
            compared_pixels: compared,
            eligible_pixels,
            ratio: round6(coverage_ratio),
        },
    )
}

fn edge_metrics(target: &RgbaImage, current: &RgbaImage, eligible: &[bool]) -> (f64, f64) {
    let left = edge_map(target);
    let right = edge_map(current);
    let mut error_sum = 0.0;
    let mut compared = 0_u64;
    for index in 0..left.len().min(right.len()) {
        if !eligible.get(index).copied().unwrap_or(false) {
            continue;
        }
        error_sum += left[index].abs_diff(right[index]) as f64 / 255.0;
        compared += 1;
    }
    let error = error_sum / compared.max(1) as f64;
    (1.0 - error, error)
}

fn edge_map(image: &RgbaImage) -> Vec<u8> {
    let width = image.width() as usize;
    let height = image.height() as usize;
    let mut gray = vec![0_u8; width * height];
    for (index, pixel) in image.pixels().enumerate() {
        if pixel[3] != 0 {
            gray[index] = (luminance(pixel) * 255.0).round() as u8;
        }
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

fn percentile(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 1.0;
    }
    let index = ((values.len() - 1) as f64 * percentile).round() as usize;
    values[index.min(values.len() - 1)]
}

fn luminance(pixel: &image::Rgba<u8>) -> f64 {
    (pixel[0] as f64 * 0.2126 + pixel[1] as f64 * 0.7152 + pixel[2] as f64 * 0.0722) / 255.0
}

fn srgb_to_lab(pixel: &image::Rgba<u8>) -> [f64; 3] {
    let linear = |value: u8| {
        let value = value as f64 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    let r = linear(pixel[0]);
    let g = linear(pixel[1]);
    let b = linear(pixel[2]);
    let x = (r * 0.4124564 + g * 0.3575761 + b * 0.1804375) / 0.95047;
    let y = r * 0.2126729 + g * 0.7151522 + b * 0.0721750;
    let z = (r * 0.0193339 + g * 0.1191920 + b * 0.9503041) / 1.08883;
    let pivot = |value: f64| {
        if value > 0.008856 {
            value.cbrt()
        } else {
            7.787 * value + 16.0 / 116.0
        }
    };
    let fx = pivot(x);
    let fy = pivot(y);
    let fz = pivot(z);
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn delta_e_76(left: [f64; 3], right: [f64; 3]) -> f64 {
    ((left[0] - right[0]).powi(2) + (left[1] - right[1]).powi(2) + (left[2] - right[2]).powi(2))
        .sqrt()
}

fn rect_width(rect: PixelRect) -> f64 {
    (rect.right - rect.left).max(1) as f64
}

fn rect_height(rect: PixelRect) -> f64 {
    (rect.bottom - rect.top).max(1) as f64
}

pub(super) fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}
