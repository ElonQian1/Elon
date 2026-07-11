use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use image::{DynamicImage, GenericImageView, ImageFormat};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::node_agent_android_inspector::adb_capture::capture_screen_png;

use super::broker::LiveUiSession;
use super::visual_diff::PixelRect;

const MAX_FRAME_BYTES: usize = 32 * 1024 * 1024;
const MAX_SAVED_FRAMES: usize = 12;
const MAX_IMAGE_SIDE: u32 = 16_384;
const MAX_IMAGE_PIXELS: u64 = 40 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LiveFrameArtifact {
    pub(crate) source: &'static str,
    pub(crate) path: String,
    pub(crate) sha256: String,
    pub(crate) full_frame_sha256: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) full_frame_width: u32,
    pub(crate) full_frame_height: u32,
    pub(crate) rect: PixelRect,
    pub(crate) captured_at: String,
}

pub(crate) async fn capture_latest_frame_artifact(
    session: &LiveUiSession,
    rect: Option<PixelRect>,
) -> Result<LiveFrameArtifact> {
    let png = capture_screen_png(&session.device_id).await?;
    persist_frame_artifact(session, &png, rect)
}

pub(crate) fn persist_frame_artifact(
    session: &LiveUiSession,
    full_png: &[u8],
    rect: Option<PixelRect>,
) -> Result<LiveFrameArtifact> {
    persist_image_artifact(session, full_png, rect, "ADB_LIVE", "current")
}

pub(crate) fn persist_target_crop_artifact(
    session: &LiveUiSession,
    target_path: &str,
    rect: Option<PixelRect>,
) -> Result<LiveFrameArtifact> {
    let bytes = read_bounded_image(target_path)?;
    persist_image_artifact(session, &bytes, rect, "TARGET_DESIGN", "target-crop")
}

fn persist_image_artifact(
    session: &LiveUiSession,
    bytes: &[u8],
    rect: Option<PixelRect>,
    source: &'static str,
    prefix: &str,
) -> Result<LiveFrameArtifact> {
    if bytes.is_empty() || bytes.len() > MAX_FRAME_BYTES {
        bail!("Live Frame 大小必须在 1..32MiB");
    }
    let (header_width, header_height) = validated_image_dimensions(bytes)?;
    let image = image::load_from_memory(bytes).context("无法解码 UI 图片工件")?;
    let (full_width, full_height) = image.dimensions();
    if (full_width, full_height) != (header_width, header_height) {
        bail!("UI 图片解码尺寸与文件头不一致");
    }
    let rect = normalize_rect(rect, full_width, full_height)?;
    let crop = image.crop_imm(
        rect.left as u32,
        rect.top as u32,
        (rect.right - rect.left) as u32,
        (rect.bottom - rect.top) as u32,
    );
    let crop_png = encode_png(&crop)?;
    let sha256 = sha256_bytes(&crop_png);
    let full_frame_sha256 = sha256_bytes(bytes);
    let root = frame_root(session)?;
    fs::create_dir_all(&root)
        .with_context(|| format!("创建 Live Frame 工件目录失败: {}", root.display()))?;
    let path = root.join(format!("{prefix}-{sha256}.png"));
    if !path.exists() {
        fs::write(&path, &crop_png)
            .with_context(|| format!("写入 Live Frame 工件失败: {}", path.display()))?;
    }
    prune_old_frames(&root, &path);
    Ok(LiveFrameArtifact {
        source,
        path: path.display().to_string(),
        sha256,
        full_frame_sha256,
        width: crop.width(),
        height: crop.height(),
        full_frame_width: full_width,
        full_frame_height: full_height,
        rect,
        captured_at: Utc::now().to_rfc3339(),
    })
}

fn normalize_rect(rect: Option<PixelRect>, width: u32, height: u32) -> Result<PixelRect> {
    let rect = rect.unwrap_or(PixelRect {
        left: 0,
        top: 0,
        right: width as i32,
        bottom: height as i32,
    });
    let normalized = PixelRect {
        left: rect.left.max(0).min(width as i32),
        top: rect.top.max(0).min(height as i32),
        right: rect.right.max(0).min(width as i32),
        bottom: rect.bottom.max(0).min(height as i32),
    };
    if normalized.right <= normalized.left || normalized.bottom <= normalized.top {
        bail!("Live Frame 裁剪区域为空或超出屏幕范围");
    }
    Ok(normalized)
}

fn read_bounded_image(path: &str) -> Result<Vec<u8>> {
    let metadata =
        fs::metadata(path).with_context(|| format!("读取目标设计图元数据失败: {path}"))?;
    if metadata.len() == 0 || metadata.len() > MAX_FRAME_BYTES as u64 {
        bail!("目标设计图大小必须在 1..32MiB");
    }
    fs::read(path).with_context(|| format!("读取目标设计图失败: {path}"))
}

fn validated_image_dimensions(bytes: &[u8]) -> Result<(u32, u32)> {
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .context("无法识别 UI 图片工件格式")?;
    let (width, height) = reader
        .into_dimensions()
        .context("无法读取 UI 图片工件尺寸")?;
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if width == 0
        || height == 0
        || width > MAX_IMAGE_SIDE
        || height > MAX_IMAGE_SIDE
        || pixels > MAX_IMAGE_PIXELS
    {
        bail!(
            "UI 图片尺寸超限：{width}x{height}，单边不超过 {MAX_IMAGE_SIDE}，总像素不超过 {MAX_IMAGE_PIXELS}"
        );
    }
    Ok((width, height))
}

fn encode_png(image: &DynamicImage) -> Result<Vec<u8>> {
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, ImageFormat::Png)
        .context("编码 Live Frame 裁剪失败")?;
    Ok(output.into_inner())
}

fn frame_root(session: &LiveUiSession) -> Result<PathBuf> {
    validate_session_id(&session.id)?;
    let base = if let Some(project_root) = session
        .project_root
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        PathBuf::from(project_root)
            .canonicalize()
            .context("Live UI 项目目录不存在")?
            .join(".elon")
            .join("ui-tuner")
            .join("live")
    } else {
        std::env::temp_dir().join("elon-ui-tuner-live")
    };
    Ok(base.join(&session.id).join("frames"))
}

fn validate_session_id(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 96
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
    {
        bail!("Live UI sessionId 非法");
    }
    Ok(())
}

fn prune_old_frames(root: &Path, keep: &Path) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut frames = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let modified = entry.metadata().ok()?.modified().ok()?;
            path.extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("png"))
                .then_some((modified, path))
        })
        .collect::<Vec<_>>();
    frames.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    for (_, path) in frames.into_iter().skip(MAX_SAVED_FRAMES) {
        if path != keep {
            let _ = fs::remove_file(path);
        }
    }
}

fn sha256_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

    use super::*;
    use crate::node_agent_android_live::broker::LiveUiBroker;

    fn test_png(width: u32, height: u32) -> Vec<u8> {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            width,
            height,
            Rgba([12, 34, 56, 255]),
        ));
        let mut output = Cursor::new(Vec::new());
        image.write_to(&mut output, ImageFormat::Png).unwrap();
        output.into_inner()
    }

    #[tokio::test]
    async fn persists_a_real_cropped_png_with_content_hash() {
        let root = std::env::temp_dir().join(format!(
            "elon-live-frame-artifact-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).unwrap();
        let broker = LiveUiBroker::new();
        let session = broker
            .create_session(
                "device-1".to_string(),
                "com.example.debug".to_string(),
                Some(root.display().to_string()),
                38917,
            )
            .await;
        let artifact = persist_frame_artifact(
            &session,
            &test_png(40, 30),
            Some(PixelRect {
                left: 5,
                top: 6,
                right: 25,
                bottom: 18,
            }),
        )
        .unwrap();
        assert_eq!(artifact.width, 20);
        assert_eq!(artifact.height, 12);
        assert_eq!(artifact.full_frame_width, 40);
        assert_eq!(artifact.full_frame_height, 30);
        assert_eq!(artifact.source, "ADB_LIVE");
        assert_eq!(artifact.sha256.len(), 64);
        let persisted = image::open(&artifact.path).unwrap();
        assert_eq!(persisted.width(), 20);
        assert_eq!(persisted.height(), 12);
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn rejects_empty_crop_after_screen_clamping() {
        let root = std::env::temp_dir().join(format!(
            "elon-live-frame-artifact-invalid-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).unwrap();
        let broker = LiveUiBroker::new();
        let session = broker
            .create_session(
                "device-1".to_string(),
                "com.example.debug".to_string(),
                Some(root.display().to_string()),
                38917,
            )
            .await;
        let error = persist_frame_artifact(
            &session,
            &test_png(20, 20),
            Some(PixelRect {
                left: 30,
                top: 30,
                right: 40,
                bottom: 40,
            }),
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("裁剪区域为空"));
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn rejects_oversized_dimensions_before_full_decode() {
        let root = std::env::temp_dir().join(format!(
            "elon-live-frame-artifact-dimensions-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).unwrap();
        let broker = LiveUiBroker::new();
        let session = broker
            .create_session(
                "device-1".to_string(),
                "com.example.debug".to_string(),
                Some(root.display().to_string()),
                38917,
            )
            .await;
        let error =
            persist_frame_artifact(&session, &test_png(MAX_IMAGE_SIDE + 1, 1), None).unwrap_err();
        assert!(format!("{error:#}").contains("尺寸超限"));
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn rejects_oversized_target_file_before_reading_it() {
        let root = std::env::temp_dir().join(format!(
            "elon-live-frame-artifact-bytes-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).unwrap();
        let target = root.join("oversized.png");
        let file = fs::File::create(&target).unwrap();
        file.set_len(MAX_FRAME_BYTES as u64 + 1).unwrap();
        drop(file);
        let broker = LiveUiBroker::new();
        let session = broker
            .create_session(
                "device-1".to_string(),
                "com.example.debug".to_string(),
                Some(root.display().to_string()),
                38917,
            )
            .await;
        let error =
            persist_target_crop_artifact(&session, target.to_str().unwrap(), None).unwrap_err();
        assert!(format!("{error:#}").contains("大小必须"));
        fs::remove_dir_all(root).unwrap();
    }
}
