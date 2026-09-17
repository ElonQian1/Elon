use super::{cover, metadata, policy, Preview};
use anyhow::{bail, Result};
use reqwest::Url;
use std::time::Duration;

const MAX_HEAD: usize = 256 * 1024;

// Every hop is resolved and pinned independently; no cookies, auth, proxy or auto redirects.
async fn response(url: &Url) -> Result<reqwest::Response> {
    let target = crate::open_commerce_outbound_security::pinned_public_https_target(
        url.as_str(),
        Duration::from_secs(3),
        Duration::from_secs(7),
    )
    .await?;
    Ok(target
        .client
        .get(&target.url)
        .header("User-Agent", policy::user_agent(url))
        .header("Accept", "text/html,application/json;q=0.9")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .send()
        .await?)
}

async fn body(mut response: reqwest::Response, head: bool) -> Result<String> {
    if !response.status().is_success() {
        bail!("preview unavailable");
    }
    let mime = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !mime.contains(if head { "text/html" } else { "json" }) {
        bail!("unsupported preview");
    }
    // Bilibili and others gzip regardless of Accept-Encoding; reqwest has no decoder enabled.
    let gzip = response
        .headers()
        .get("content-encoding")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("gzip"));
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        let remaining = MAX_HEAD.saturating_sub(bytes.len());
        bytes.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        if bytes.len() >= MAX_HEAD
            || (head && !gzip && bytes.windows(7).any(|s| s.eq_ignore_ascii_case(b"</head>")))
        {
            break;
        }
    }
    if gzip {
        use std::io::Read;
        let mut out = Vec::new();
        let _ = flate2::read::GzDecoder::new(bytes.as_slice())
            .take(4 * MAX_HEAD as u64)
            .read_to_end(&mut out);
        bytes = out;
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub(super) async fn resolve(mut url: Url) -> Result<Preview> {
    let original = url.clone();
    let mut page = None;
    // X exposes an official oEmbed endpoint. Do not parse its application HTML shell.
    if policy::embed(&url).is_some_and(|e| e.kind == "x") {
        return x_preview(original).await;
    }
    for hop in 0..=4 {
        if policy::embed(&url).is_some_and(|e| e.kind == "douyin") {
            return douyin_preview(&original, &url).await;
        }
        let res = response(&url).await?;
        if res.status().is_redirection() {
            if hop == 4 {
                bail!("too many redirects");
            }
            let location = res
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| anyhow::anyhow!("redirect missing"))?;
            url = policy::public_url(url.join(location)?.as_str())
                .ok_or_else(|| anyhow::anyhow!("unsafe redirect"))?;
            continue;
        }
        page = Some(body(res, true).await?);
        break;
    }
    let mut preview = Preview::fallback(&original);
    preview.site = policy::label(&url);
    preview.embed = policy::embed(&url);
    if let Some(page) = page {
        let metadata = metadata::parse(&page, &url);
        preview.title = metadata.title;
        preview.description = metadata.description;
        preview.author = metadata.author;
        preview.image = metadata.image;
        if let Some(image) = preview.image.as_deref().and_then(policy::public_url) {
            preview.cover_data_url = cover::fetch(&image).await;
            // An http-only cover that cannot be copied over https is not offered to clients.
            if metadata.image_needs_check && preview.cover_data_url.is_none() {
                preview.image = None;
            }
        }
        // Unknown hosts are never hot-linked by clients; only the bounded server copy is shared.
        if policy::generic(&url) {
            preview.image = None;
        }
    }
    preview.status = if preview.title.is_empty() {
        "unavailable"
    } else {
        "ready"
    };
    Ok(preview)
}

async fn douyin_preview(original: &Url, resolved: &Url) -> Result<Preview> {
    let mut preview = Preview::fallback(original);
    preview.embed = policy::embed(resolved);
    if let Some(embed) = &preview.embed {
        let mut endpoint =
            Url::parse("https://open.douyin.com/api/douyin/v1/video/get_iframe_by_video")?;
        endpoint
            .query_pairs_mut()
            .append_pair("video_id", &embed.id);
        if let Ok(res) = response(&endpoint).await {
            if let Ok(text) = body(res, false).await {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                    preview.title = metadata::clean(
                        value
                            .pointer("/data/video_title")
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                        160,
                    );
                }
            }
        }
    }
    preview.status = if preview.title.is_empty() {
        "unavailable"
    } else {
        "ready"
    };
    Ok(preview)
}

async fn x_preview(url: Url) -> Result<Preview> {
    let mut endpoint = Url::parse("https://publish.x.com/oembed")?;
    endpoint
        .query_pairs_mut()
        .append_pair("url", url.as_str())
        .append_pair("omit_script", "1")
        .append_pair("dnt", "true")
        .append_pair("hide_thread", "true");
    let value: serde_json::Value =
        serde_json::from_str(&body(response(&endpoint).await?, false).await?)?;
    let mut preview = Preview::fallback(&url);
    preview.title = metadata::plain_snippet(value["html"].as_str().unwrap_or(""));
    preview.author = metadata::clean(value["author_name"].as_str().unwrap_or(""), 80);
    preview.status = if preview.title.is_empty() {
        "unavailable"
    } else {
        "ready"
    };
    Ok(preview)
}
