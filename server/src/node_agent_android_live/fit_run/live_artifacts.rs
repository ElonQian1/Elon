use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};

use super::model::{FitRect, FitRunDocument};
use crate::node_agent_android_live::build_verify::BuildVerifyRequest;
use crate::node_agent_android_live::visual_diff::PixelRect;

pub(super) fn persist_frame(run: &FitRunDocument, trial_id: &str, png: &[u8]) -> Result<String> {
    let root = PathBuf::from(&run.project_root)
        .canonicalize()
        .context("FitRun 项目目录不存在")?;
    let dir = root
        .join(".elon")
        .join("ui-tuner")
        .join("fit-runs")
        .join(&run.run_id)
        .join("frames");
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{trial_id}.png"));
    fs::write(&path, png)?;
    Ok(path.display().to_string())
}

pub(super) fn pixel_rect(value: FitRect) -> PixelRect {
    PixelRect {
        left: value.left,
        top: value.top,
        right: value.right,
        bottom: value.bottom,
    }
}

pub(super) fn build_verify_request(run: &FitRunDocument) -> Result<BuildVerifyRequest> {
    Ok(BuildVerifyRequest {
        preview: None,
        debug_application_id_suffix: None,
        lkg_enabled: false,
        target_rect: Some(pixel_rect(run.pair.target_rect)),
        current_rect: Some(pixel_rect(run.pair.current_rect)),
        projected_current_rect: Some(pixel_rect(run.pair.projected_target_rect)),
        target_definition_id: Some(run.pair.definition_id.clone()),
        target_instance_key: run.pair.instance_key.clone(),
        visual_mask: run.visual_mask.visual_mask(),
        state_replay: run.environment.validated_state_replay()?,
    })
}

pub(super) fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u64::MAX as u128) as u64
}
