mod image_pair;
mod metrics;
pub(crate) mod types;

use anyhow::Result;
use image::{DynamicImage, GenericImageView};

use self::image_pair::{crop, prepare_pair};
use self::metrics::{geometry_metrics, pixel_metrics, position_metrics, round6};
use self::types::{MetricGate, VisualMask, VisualScoreProfile, VisualScoreReport};
use super::PixelRect;

pub(crate) struct ScoreInput<'a> {
    pub(crate) target: &'a DynamicImage,
    pub(crate) current: &'a DynamicImage,
    pub(crate) target_rect: Option<PixelRect>,
    pub(crate) current_rect: Option<PixelRect>,
    pub(crate) projected_target_rect: Option<PixelRect>,
    pub(crate) mask: &'a VisualMask,
    pub(crate) profile: VisualScoreProfile,
}

pub(crate) struct ScoredImages {
    pub(crate) target_width: u32,
    pub(crate) target_height: u32,
    pub(crate) current_width: u32,
    pub(crate) current_height: u32,
    pub(crate) report: VisualScoreReport,
}

pub(crate) fn score_images(input: ScoreInput<'_>) -> Result<ScoredImages> {
    let target_crop = crop(input.target, input.target_rect)?;
    let current_crop = crop(input.current, input.current_rect)?;
    let (target_width, target_height) = target_crop.dimensions();
    let (current_width, current_height) = current_crop.dimensions();
    let current_rect = input.current_rect.unwrap_or(PixelRect {
        left: 0,
        top: 0,
        right: current_width as i32,
        bottom: current_height as i32,
    });
    let projected_target_rect = input.projected_target_rect.unwrap_or(PixelRect {
        left: current_rect.left,
        top: current_rect.top,
        right: current_rect.left + target_width as i32,
        bottom: current_rect.top + target_height as i32,
    });
    let prepared = prepare_pair(
        &target_crop,
        &current_crop,
        projected_target_rect,
        current_rect,
        input.mask,
    )?;
    let geometry = geometry_metrics(projected_target_rect, current_rect);
    let position = position_metrics(projected_target_rect, current_rect);
    let (color, edge, perceptual, coverage) =
        pixel_metrics(&prepared.target, &prepared.current, &prepared.eligible);
    let gate = evaluate_gate(
        &input.profile,
        &geometry,
        &position,
        &color,
        &edge,
        &perceptual,
        &coverage,
    );
    let optimization_score = round6(
        geometry.size_error_ratio * 0.20
            + geometry.aspect_error_ratio * 0.10
            + normalized_position_error(position.max_edge_error_px, projected_target_rect) * 0.15
            + color.mean_absolute_error * 0.25
            + edge.error * 0.15
            + perceptual.structural_error * 0.10
            + (1.0 - coverage.ratio) * 0.05,
    );
    Ok(ScoredImages {
        target_width,
        target_height,
        current_width,
        current_height,
        report: VisualScoreReport {
            schema_version: 1,
            geometry,
            position,
            color,
            edge,
            perceptual,
            coverage,
            optimization_score,
            target_gate: gate,
            comparison_width: prepared.target.width(),
            comparison_height: prepared.target.height(),
        },
    })
}

fn evaluate_gate(
    profile: &VisualScoreProfile,
    geometry: &types::GeometryMetrics,
    position: &types::PositionMetrics,
    color: &types::ColorMetrics,
    edge: &types::EdgeMetrics,
    perceptual: &types::PerceptualMetrics,
    coverage: &types::CoverageMetrics,
) -> MetricGate {
    let geometry_passed = geometry.size_error_ratio <= profile.max_size_error_ratio
        && geometry.aspect_error_ratio <= profile.max_aspect_error_ratio;
    let position_passed = position.max_edge_error_px <= profile.max_position_error_px;
    let color_passed = color.mean_absolute_error <= profile.max_mean_color_error
        && color.mean_delta_e <= profile.max_mean_delta_e;
    let edge_passed = edge.similarity >= profile.min_edge_similarity;
    let perceptual_passed = perceptual.structural_error <= profile.max_perceptual_error;
    let coverage_passed = coverage.ratio >= profile.min_coverage;
    let mut failed_metrics = Vec::new();
    if !geometry_passed {
        failed_metrics.push("geometry");
    }
    if !position_passed {
        failed_metrics.push("position");
    }
    if !color_passed {
        failed_metrics.push("color");
    }
    if !edge_passed {
        failed_metrics.push("edge");
    }
    if !perceptual_passed {
        failed_metrics.push("perceptual");
    }
    if !coverage_passed {
        failed_metrics.push("coverage");
    }
    MetricGate {
        passed: failed_metrics.is_empty(),
        geometry_passed,
        position_passed,
        color_passed,
        edge_passed,
        perceptual_passed,
        coverage_passed,
        failed_metrics,
    }
}

fn normalized_position_error(error: f64, rect: PixelRect) -> f64 {
    let scale = ((rect.right - rect.left).unsigned_abs())
        .max((rect.bottom - rect.top).unsigned_abs())
        .max(1) as f64;
    (error / scale).min(1.0)
}
