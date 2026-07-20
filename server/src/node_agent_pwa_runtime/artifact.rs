use super::{
    browser::{BrowserIdentity, ProcessCleanup, RenderedCapture},
    security::{CaptureEvidence, PreparedCapture, SanitizedRoute},
    CaptureDiagnostic,
};
use chrono::Utc;
use image::GenericImageView;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const MAX_PNG_BYTES: usize = 32 * 1024 * 1024;
const MAX_IMAGE_SIDE: u32 = 16_384;
const MAX_IMAGE_PIXELS: u64 = 40_000_000;

pub(super) struct PersistedCapture {
    pub(super) artifact: PwaPngArtifact,
    pub(super) route: SanitizedRoute,
    pub(super) browser: BrowserIdentity,
    pub(super) viewport: CaptureViewportMetadata,
    pub(super) network_policy: NetworkPolicyMetadata,
    pub(super) process_cleanup: ProcessCleanup,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PwaPngArtifact {
    pub(super) path: String,
    pub(super) manifest_path: String,
    pub(super) sha256: String,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) bytes: usize,
    pub(super) media_type: &'static str,
    pub(super) captured_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CaptureViewportMetadata {
    requested_width: u32,
    requested_height: u32,
    device_scale_factor: f64,
    captured_css_width: f64,
    captured_css_height: f64,
    actual_pixel_width: u32,
    actual_pixel_height: u32,
    full_page: bool,
    selector_capture: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NetworkPolicyMetadata {
    allowed_origin_count: usize,
    blocked_request_count: u32,
    open_world: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureManifest<'a> {
    schema: &'static str,
    artifact: &'a PwaPngArtifact,
    route: &'a SanitizedRoute,
    revision: &'a CaptureEvidence,
    browser: &'a BrowserIdentity,
    viewport: &'a CaptureViewportMetadata,
    network_policy: &'a NetworkPolicyMetadata,
    process_cleanup: &'a ProcessCleanup,
    authentication_mode: &'static str,
    base64_embedded: bool,
}

pub(super) fn persist(
    prepared: &PreparedCapture,
    rendered: RenderedCapture,
) -> Result<PersistedCapture, CaptureDiagnostic> {
    if rendered.png.is_empty() || rendered.png.len() > MAX_PNG_BYTES {
        return Err(artifact_error(
            "ARTIFACT_SIZE_INVALID",
            "PNG 工件必须在 1..32MiB",
        ));
    }
    let image = image::load_from_memory(&rendered.png)
        .map_err(|_| artifact_error("PNG_DECODE_FAILED", "浏览器工件不是可解码 PNG"))?;
    let (width, height) = image.dimensions();
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if width == 0
        || height == 0
        || width > MAX_IMAGE_SIDE
        || height > MAX_IMAGE_SIDE
        || pixels > MAX_IMAGE_PIXELS
    {
        return Err(artifact_error(
            "ARTIFACT_DIMENSION_LIMIT",
            "PNG 实际尺寸超过单边 16384/总计 4000 万像素上限",
        ));
    }
    if !prepared.capture.full_page && prepared.capture.selector.is_none() {
        let expected_width = (f64::from(prepared.viewport.width)
            * prepared.viewport.device_scale_factor)
            .round() as u32;
        let expected_height = (f64::from(prepared.viewport.height)
            * prepared.viewport.device_scale_factor)
            .round() as u32;
        if (width, height) != (expected_width, expected_height) {
            return Err(artifact_error(
                "VIEWPORT_PIXEL_MISMATCH",
                format!(
                    "PNG 实际尺寸 {width}x{height} 与固定 viewport 像素 {expected_width}x{expected_height} 不一致"
                ),
            ));
        }
    }
    let sha256 = hex::encode(Sha256::digest(&rendered.png));
    let captured_at = Utc::now().to_rfc3339();
    let root = artifact_root(prepared)?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let png_path = root.join(format!("capture-{id}-{}.png", &sha256[..16]));
    let manifest_path = png_path.with_extension("json");
    if fs::write(&png_path, &rendered.png).is_err() {
        let _ = fs::remove_file(&png_path);
        return Err(artifact_error(
            "ARTIFACT_WRITE_FAILED",
            "无法写入 PWA PNG 工件",
        ));
    }
    let artifact = PwaPngArtifact {
        path: png_path.display().to_string(),
        manifest_path: manifest_path.display().to_string(),
        sha256,
        width,
        height,
        bytes: rendered.png.len(),
        media_type: "image/png",
        captured_at,
    };
    let viewport = CaptureViewportMetadata {
        requested_width: prepared.viewport.width,
        requested_height: prepared.viewport.height,
        device_scale_factor: prepared.viewport.device_scale_factor,
        captured_css_width: rendered.css_width,
        captured_css_height: rendered.css_height,
        actual_pixel_width: width,
        actual_pixel_height: height,
        full_page: prepared.capture.full_page,
        selector_capture: prepared.capture.selector.is_some(),
    };
    let network_policy = NetworkPolicyMetadata {
        allowed_origin_count: prepared.allowed_origins.len(),
        blocked_request_count: rendered.blocked_request_count,
        open_world: false,
    };
    let manifest = CaptureManifest {
        schema: "elon.pwa.runtime-capture.v1",
        artifact: &artifact,
        route: &rendered.route,
        revision: &prepared.evidence,
        browser: &rendered.browser,
        viewport: &viewport,
        network_policy: &network_policy,
        process_cleanup: &rendered.process_cleanup,
        authentication_mode: prepared.auth.mode,
        base64_embedded: false,
    };
    if fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest)
            .map_err(|_| artifact_error("ARTIFACT_WRITE_FAILED", "无法序列化 PWA PNG manifest"))?,
    )
    .is_err()
    {
        let _ = fs::remove_file(&png_path);
        let _ = fs::remove_file(&manifest_path);
        return Err(artifact_error(
            "ARTIFACT_WRITE_FAILED",
            "无法写入 PWA PNG manifest",
        ));
    }
    Ok(PersistedCapture {
        artifact,
        route: rendered.route,
        browser: rendered.browser,
        viewport,
        network_policy,
        process_cleanup: rendered.process_cleanup,
    })
}

fn artifact_root(prepared: &PreparedCapture) -> Result<PathBuf, CaptureDiagnostic> {
    let requested = prepared
        .project_root
        .join(".elon")
        .join("ui-tuner")
        .join("pwa-runtime")
        .join("captures");
    let mut existing = requested.as_path();
    while !existing.exists() {
        existing = existing.parent().ok_or_else(|| {
            artifact_error("ARTIFACT_PATH_REJECTED", "PWA PNG 工件目录没有可信项目祖先")
        })?;
    }
    let canonical_existing = existing.canonicalize().map_err(|_| {
        artifact_error(
            "ARTIFACT_PATH_REJECTED",
            "无法规范化 PWA PNG 工件目录的现有祖先",
        )
    })?;
    if !canonical_existing.starts_with(&prepared.project_root) || canonical_existing != existing {
        return Err(artifact_error(
            "ARTIFACT_PATH_REJECTED",
            "PWA PNG 工件目录的现有祖先经过链接/重解析点",
        ));
    }
    fs::create_dir_all(&requested)
        .map_err(|_| artifact_error("ARTIFACT_WRITE_FAILED", "无法创建 PWA PNG 工件目录"))?;
    let root = requested
        .canonicalize()
        .map_err(|_| artifact_error("ARTIFACT_PATH_REJECTED", "无法规范化 PWA PNG 工件目录"))?;
    if !root.starts_with(&prepared.project_root) || root != requested {
        return Err(artifact_error(
            "ARTIFACT_PATH_REJECTED",
            "PWA PNG 工件目录越出项目或经过链接/重解析点",
        ));
    }
    Ok(root)
}

fn artifact_error(code: &'static str, message: impl Into<String>) -> CaptureDiagnostic {
    CaptureDiagnostic::new(
        code,
        message,
        false,
        "检查项目 .elon/ui-tuner/pwa-runtime 目录权限和输出上限后重试",
    )
}
