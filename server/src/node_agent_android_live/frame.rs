use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chrono::Utc;
use serde::Serialize;

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

pub(crate) async fn capture_frame(session: &LiveUiSession) -> Result<LiveFrame> {
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
