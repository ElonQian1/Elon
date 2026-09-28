//! Adapter for the production capture-tool response (the manifest has a different shape).
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(crate) struct CapturedWeb {
    pub(crate) image: Vec<u8>,
    pub(crate) tree: Vec<u8>,
    pub(crate) manifest: Vec<u8>,
}

pub(crate) fn read(project_root: &Path, response: &Value) -> Result<CapturedWeb> {
    let capture_root = project_root
        .join(".elon/ui-tuner/pwa-runtime/captures")
        .canonicalize()?;
    Ok(CapturedWeb {
        image: captured_file(&capture_root, &response["artifact"]["path"])?,
        tree: captured_file(&capture_root, &response["uiTree"]["path"])?,
        manifest: captured_file(&capture_root, &response["artifact"]["manifestPath"])?,
    })
}

fn captured_file(capture_root: &Path, value: &Value) -> Result<Vec<u8>> {
    let path =
        PathBuf::from(value.as_str().context("SEMANTIC_WEB_ARTIFACT_MISSING")?).canonicalize()?;
    if !path.starts_with(capture_root)
        || !path.is_file()
        || path.metadata()?.len() > 16 * 1024 * 1024
    {
        bail!("SEMANTIC_WEB_ARTIFACT_PATH_INVALID");
    }
    Ok(std::fs::read(path)?)
}
