use super::{metadata, policy, Preview};
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
        .header("User-Agent", "YilongLinkPreview/1.0")
        .header("Accept", "text/html,application/json;q=0.9")
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
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        let remaining = MAX_HEAD.saturating_sub(bytes.len());
        bytes.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        if bytes.len() >= MAX_HEAD
            || (head && bytes.windows(7).any(|s| s.eq_ignore_ascii_case(b"</head>")))
        {
            break;
        }
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
        if !policy::fetchable(&url) {
            bail!("unsupported redirect");
        }
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
    preview.site = policy::site(&url).into();
    preview.embed = policy::embed(&url);
    if let Some(page) = page {
        let metadata = metadata::parse(&page, &url);
        preview.title = metadata.title;
        preview.author = metadata.author;
        preview.image = metadata.image;
        if metadata.image_needs_check {
            if let Some(image) = &preview.image {
                let available = tokio::time::timeout(Duration::from_secs(2), check_image(image))
                    .await
                    .unwrap_or(false);
                if !available {
                    preview.image = None;
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

async fn check_image(value: &str) -> bool {
    let Some(url) = policy::public_url(value) else {
        return false;
    };
    match response(&url).await {
        Ok(res) => {
            res.status().is_success()
                && res
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v.starts_with("image/"))
        }
        Err(_) => false,
    }
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
