use serde::{Deserialize, Serialize};

use super::super::PixelRect;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VisualScoreProfile {
    pub(crate) max_size_error_ratio: f64,
    pub(crate) max_position_error_px: f64,
    pub(crate) max_aspect_error_ratio: f64,
    pub(crate) max_mean_color_error: f64,
    pub(crate) max_mean_delta_e: f64,
    pub(crate) min_edge_similarity: f64,
    pub(crate) max_perceptual_error: f64,
    pub(crate) min_coverage: f64,
}

impl Default for VisualScoreProfile {
    fn default() -> Self {
        Self {
            max_size_error_ratio: 0.02,
            max_position_error_px: 2.0,
            max_aspect_error_ratio: 0.02,
            max_mean_color_error: 0.04,
            max_mean_delta_e: 3.0,
            min_edge_similarity: 0.90,
            max_perceptual_error: 0.05,
            min_coverage: 0.75,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VisualMask {
    #[serde(default)]
    pub(crate) exclude_rects: Vec<PixelRect>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GeometryMetrics {
    pub(crate) width_error_px: f64,
    pub(crate) height_error_px: f64,
    pub(crate) size_error_ratio: f64,
    pub(crate) aspect_error_ratio: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PositionMetrics {
    pub(crate) left_error_px: f64,
    pub(crate) top_error_px: f64,
    pub(crate) right_error_px: f64,
    pub(crate) bottom_error_px: f64,
    pub(crate) center_error_px: f64,
    pub(crate) max_edge_error_px: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ColorMetrics {
    pub(crate) mean_absolute_error: f64,
    pub(crate) p95_absolute_error: f64,
    pub(crate) mean_delta_e: f64,
    pub(crate) p95_delta_e: f64,
    pub(crate) alpha_error: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EdgeMetrics {
    pub(crate) similarity: f64,
    pub(crate) error: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PerceptualMetrics {
    pub(crate) luminance_error: f64,
    pub(crate) structural_error: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoverageMetrics {
    pub(crate) compared_pixels: u64,
    pub(crate) eligible_pixels: u64,
    pub(crate) ratio: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MetricGate {
    pub(crate) passed: bool,
    pub(crate) geometry_passed: bool,
    pub(crate) position_passed: bool,
    pub(crate) color_passed: bool,
    pub(crate) edge_passed: bool,
    pub(crate) perceptual_passed: bool,
    pub(crate) coverage_passed: bool,
    pub(crate) failed_metrics: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VisualScoreReport {
    pub(crate) schema_version: u32,
    pub(crate) geometry: GeometryMetrics,
    pub(crate) position: PositionMetrics,
    pub(crate) color: ColorMetrics,
    pub(crate) edge: EdgeMetrics,
    pub(crate) perceptual: PerceptualMetrics,
    pub(crate) coverage: CoverageMetrics,
    pub(crate) optimization_score: f64,
    pub(crate) target_gate: MetricGate,
    pub(crate) comparison_width: u32,
    pub(crate) comparison_height: u32,
}
