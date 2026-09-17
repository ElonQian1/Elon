//! Provider cover images are copied once into a small inline thumbnail so that clients never
//! hot-link CDNs that reject third-party referrers, and never receive oversized payloads.
use super::policy;
use anyhow::{bail, Result};
use base64::Engine;
use image::ImageReader;
use reqwest::Url;
use std::{io::Cursor, time::Duration};

const MAX_BYTES: usize = 1024 * 1024;
const MAX_SIDE: u32 = 6000;
const THUMB_SIDE: u32 = 256;
const JPEG_QUALITY: u8 = 72;
pub(super) const MAX_DATA_URL: usize = 96 * 1024;

pub(super) async fn fetch(url: &Url) -> Option<String> {
    tokio::time::timeout(Duration::from_secs(4), download(url))
        .await
        .ok()?
        .ok()
}

async fn download(url: &Url) -> Result<String> {
    let target = crate::open_commerce_outbound_security::pinned_public_https_target(
        url.as_str(),
        Duration::from_secs(2),
        Duration::from_secs(4),
    )
    .await?;
    let mut response = target
        .client
        .get(&target.url)
        .header("User-Agent", policy::user_agent(url))
        .header("Accept", "image/*")
        .send()
        .await?;
    let mime_ok = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("image/"));
    if !response.status().is_success() || !mime_ok {
        bail!("cover unavailable");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > MAX_BYTES {
            bail!("cover too large");
        }
        bytes.extend_from_slice(&chunk);
    }
    tokio::task::spawn_blocking(move || thumbnail(&bytes)).await?
}

pub(super) fn thumbnail(bytes: &[u8]) -> Result<String> {
    let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let (width, height) = reader.into_dimensions()?;
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        bail!("cover dimensions rejected");
    }
    let decoded = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()?
        .decode()?
        .thumbnail(THUMB_SIDE, THUMB_SIDE)
        .to_rgb8();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode_image(&decoded)?;
    let data = format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(out)
    );
    if data.len() > MAX_DATA_URL {
        bail!("thumbnail too large");
    }
    Ok(data)
}
