use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chrono::Utc;
use serde::Serialize;
use serde_json::Value;

use crate::node_agent_android_inspector::{
    adb_capture::capture_screen_png, png_probe::png_dimensions,
};

use super::broker::LiveUiSession;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LiveFrame {
    data_url: String,
    width: u32,
    height: u32,
    bytes: usize,
    captured_at: String,
}

#[derive(Debug)]
pub(crate) struct RuntimeFrameImage {
    pub(crate) bytes: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

pub(crate) async fn capture_frame(session: &LiveUiSession) -> Result<LiveFrame> {
    if let Ok(value) = session.request_frame().await {
        if let Ok(frame) = parse_runtime_frame(&value) {
            return Ok(frame);
        }
    }
    let png = capture_screen_png(&session.device_id).await?;
    let (width, height) = png_dimensions(&png)?;
    Ok(LiveFrame {
        data_url: format!("data:image/png;base64,{}", B64.encode(&png)),
        width,
        height,
        bytes: png.len(),
        captured_at: Utc::now().to_rfc3339(),
    })
}

/// Captures only the pixels rendered by the application process. Build parity
/// must not fall back to an ADB display screenshot because a sleeping device,
/// notification shade, or vendor installer can otherwise become the baseline.
pub(crate) async fn capture_runtime_frame_image(
    session: &LiveUiSession,
) -> Result<RuntimeFrameImage> {
    let value = match session.request_frame().await {
        Ok(value) => value,
        Err(error) => {
            let diagnostics = super::adb_session::runtime_failure_diagnostics(session, None).await;
            return Err(error.context(diagnostics));
        }
    };
    let frame = parse_runtime_frame(&value)?;
    Ok(RuntimeFrameImage {
        bytes: decode_runtime_frame_bytes(&frame)?,
        width: frame.width,
        height: frame.height,
    })
}

fn decode_runtime_frame_bytes(frame: &LiveFrame) -> Result<Vec<u8>> {
    let (_, payload) = frame
        .data_url
        .split_once(',')
        .context("Android 真实帧 dataUrl 无效")?;
    let bytes = B64.decode(payload).context("Android 真实帧 Base64 无效")?;
    if bytes.is_empty() || bytes.len() > 8 * 1024 * 1024 {
        bail!("Android 真实帧图片大小无效");
    }
    Ok(bytes)
}

fn parse_runtime_frame(value: &Value) -> Result<LiveFrame> {
    let data_url = value
        .get("dataUrl")
        .and_then(Value::as_str)
        .context("Android 真实帧缺少 dataUrl")?;
    if !data_url.starts_with("data:image/webp;base64,") || data_url.len() > 12 * 1024 * 1024 {
        bail!("Android 真实帧格式或大小无效");
    }
    let width = value
        .get("width")
        .and_then(Value::as_u64)
        .filter(|value| (1..=16_384).contains(value))
        .context("Android 真实帧宽度无效")? as u32;
    let height = value
        .get("height")
        .and_then(Value::as_u64)
        .filter(|value| (1..=16_384).contains(value))
        .context("Android 真实帧高度无效")? as u32;
    Ok(LiveFrame {
        data_url: data_url.to_string(),
        width,
        height,
        bytes: value
            .get("bytes")
            .and_then(Value::as_u64)
            .unwrap_or_default() as usize,
        captured_at: value
            .get("capturedAt")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_runtime_webp_frame() {
        let frame = parse_runtime_frame(&json!({
            "dataUrl": "data:image/webp;base64,UklGRg==",
            "width": 1080,
            "height": 2400,
            "bytes": 8,
            "capturedAt": "123",
        }))
        .expect("runtime frame");
        assert_eq!((frame.width, frame.height, frame.bytes), (1080, 2400, 8));
    }

    #[test]
    fn decodes_runtime_frame_payload() {
        let payload = B64.encode(b"runtime-frame");
        let frame = parse_runtime_frame(&json!({
            "dataUrl": format!("data:image/webp;base64,{payload}"),
            "width": 10,
            "height": 20,
            "bytes": 13,
            "capturedAt": "123",
        }))
        .expect("runtime frame");
        assert_eq!(
            decode_runtime_frame_bytes(&frame).unwrap(),
            b"runtime-frame"
        );
    }
}
