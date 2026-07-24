use anyhow::Result;

use crate::node_agent_android_live::visual_diff::{compare_pngs, PixelRect, VisualDiffResult};

pub(super) fn compare_source_parity(
    live_frame: &[u8],
    source_frame: &[u8],
    live_rect: Option<PixelRect>,
    source_rect: Option<PixelRect>,
) -> Result<(VisualDiffResult, &'static str)> {
    if live_frame == source_frame {
        let mut diff = compare_pngs(live_frame, source_frame, None, None)?;
        // Exact process frames are stronger evidence than the derived
        // foreground coverage heuristic, including transparent/letterboxed UI.
        diff.mean_absolute_color_error = 0.0;
        diff.edge_error = 0.0;
        diff.alpha_error = 0.0;
        diff.geometry_error = 0.0;
        diff.visual_loss = 0.0;
        diff.score_report.optimization_score = 0.0;
        diff.score_report.target_gate.passed = true;
        diff.score_report.target_gate.geometry_passed = true;
        diff.score_report.target_gate.position_passed = true;
        diff.score_report.target_gate.color_passed = true;
        diff.score_report.target_gate.edge_passed = true;
        diff.score_report.target_gate.perceptual_passed = true;
        diff.score_report.target_gate.coverage_passed = true;
        diff.score_report.target_gate.failed_metrics.clear();
        return Ok((diff, "PROCESS_FRAME_EXACT"));
    }
    Ok((
        compare_pngs(live_frame, source_frame, live_rect, source_rect)?,
        "TARGET_NODE_CROP",
    ))
}

pub(super) fn target_comparison_current_rect(
    projected_current_rect: Option<PixelRect>,
    verified_current_rect: Option<PixelRect>,
) -> Option<PixelRect> {
    // The calibrated design region wins over narrower semantic node bounds.
    projected_current_rect.or(verified_current_rect)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PixelRect {
        PixelRect {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn projected_design_region_wins_over_node_semantic_bounds() {
        let projected = rect(488, 134, 592, 266);
        let node_bounds = rect(498, 134, 582, 266);
        assert_eq!(
            target_comparison_current_rect(Some(projected), Some(node_bounds)),
            Some(projected)
        );
    }

    #[test]
    fn node_bounds_remain_fallback_without_calibration() {
        let node_bounds = rect(498, 134, 582, 266);
        assert_eq!(
            target_comparison_current_rect(None, Some(node_bounds)),
            Some(node_bounds)
        );
    }
}
