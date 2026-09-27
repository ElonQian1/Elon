//! Public Channels metadata and fresh app handoffs; no user cookies or WebView required.
use super::{cover, metadata, policy, Preview};
use anyhow::{bail, Result};
use reqwest::Url;
use serde::Serialize;
use serde_json::{json, Value};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(super) fn short_id(url: &Url) -> Option<String> {
    policy::public_url(url.as_str())?;
    if url.fragment().is_some() {
        return None;
    }
    let id = match (url.host_str()?, url.path()) {
        ("weixin.qq.com", path) => path.strip_prefix("/sph/")?.to_string(),
        ("channels.weixin.qq.com", "/finder-preview/pages/sph") => {
            let ids: Vec<_> = url.query_pairs().filter(|(key, _)| key == "id").collect();
            if ids.len() != 1 {
                return None;
            }
            ids[0].1.to_string()
        }
        _ => return None,
    };
    ((1..=128).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b)))
    .then_some(id)
}

async fn fetch(id: &str) -> Result<Value> {
    // Fixed endpoint, pinned public DNS, no automatic redirect or account transfer.
    let page = format!("https://channels.weixin.qq.com/finder-preview/pages/sph?id={id}");
    let mut endpoint =
        Url::parse("https://channels.weixin.qq.com/finder-preview/api/feed/get_feed_info")?;
    endpoint.query_pairs_mut().append_pair(
        "_pageUrl",
        "https://channels.weixin.qq.com/finder-preview/pages/sph",
    );
    let target = crate::open_commerce_outbound_security::pinned_public_https_target(
        endpoint.as_str(),
        Duration::from_secs(2),
        Duration::from_secs(7),
    )
    .await?;
    let mut response = target
        .client
        .post(&target.url)
        .header("User-Agent", policy::user_agent(&endpoint))
        .header("Origin", "https://channels.weixin.qq.com")
        .header("Referer", page)
        .header("Content-Type", "application/json")
        .body(json!({"baseReq":{"generalToken":""},"shortUri":id}).to_string())
        .send()
        .await?;
    if !response.status().is_success() {
        bail!("channels unavailable");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > 131072 {
            bail!("channels response too large");
        }
        bytes.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&bytes)?;
    if value["errCode"] != 0 {
        bail!("channels request rejected");
    }
    Ok(value)
}

fn project(url: &Url, value: &Value) -> Preview {
    let mut preview = Preview::fallback(url);
    let feed = &value["data"]["feedInfo"];
    preview.site = "视频号".into();
    preview.description = metadata::clean(feed["description"].as_str().unwrap_or(""), 300);
    preview.title = metadata::clean(feed["description"].as_str().unwrap_or(""), 160);
    preview.author = metadata::clean(
        value["data"]["authorInfo"]["nickname"]
            .as_str()
            .unwrap_or(""),
        80,
    );
    preview.image = feed["coverUrl"]
        .as_str()
        .and_then(|s| policy::image_url(s, url));
    preview.status = if preview.title.is_empty() && preview.image.is_none() {
        "unavailable"
    } else {
        "ready"
    };
    preview
}

pub(super) async fn preview(url: Url) -> Result<Preview> {
    let id = short_id(&url).ok_or_else(|| anyhow::anyhow!("not a Channels link"))?;
    let value = fetch(&id).await?;
    let mut preview = project(&url, &value);
    let image = preview.image.as_deref().and_then(policy::public_url);
    let avatar = value["data"]["authorInfo"]["headImgUrl"]
        .as_str()
        .and_then(policy::public_url);
    // Optional images share one timeout window; an unavailable avatar must not hide the card.
    let (poster, avatar) = tokio::join!(
        async {
            if let Some(url) = image {
                cover::fetch(&url).await
            } else {
                None
            }
        },
        async {
            if let Some(url) = avatar {
                cover::fetch_avatar(&url).await
            } else {
                None
            }
        }
    );
    preview.cover_data_url = poster;
    preview.author_avatar_data_url = avatar;
    Ok(preview)
}

#[derive(Debug, Serialize)]
pub(crate) struct Handoff {
    schema: u8,
    source_url: String,
    launch_url: String,
    expires_at_ms: u64,
}

fn scene(id: &str, data: &Value, now: u64) -> Result<Handoff> {
    let export = data["dynamicExportId"]
        .as_str()
        .and_then(|s| s.strip_prefix("export/"))
        .ok_or_else(|| anyhow::anyhow!("invalid scene"))?;
    if !(8..=2048).contains(&export.len())
        || !export
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        bail!("invalid scene");
    }
    let expires = data["expiredTime"]
        .as_u64()
        .and_then(|n| n.checked_mul(1000))
        .ok_or_else(|| anyhow::anyhow!("invalid expiry"))?;
    if expires <= now + 5000 {
        bail!("expired scene");
    }
    let mut pairs = format!("exportId=export/{export}&actionType=0");
    for key in [
        "commentScene",
        "entryScene",
        "entryCardType",
        "requestScene",
    ] {
        if let Some(value) = data.get(key) {
            let n = value
                .as_u64()
                .filter(|n| *n <= 1_000_000)
                .ok_or_else(|| anyhow::anyhow!("invalid scene"))?;
            pairs.push_str(&format!("&{key}={n}"));
        }
    }
    let mut wrapper = Url::parse("https://local.invalid/")?;
    wrapper.query_pairs_mut().append_pair("v", &pairs);
    Ok(Handoff {
        schema: 1,
        source_url: format!("https://channels.weixin.qq.com/finder-preview/pages/sph?id={id}"),
        launch_url: format!(
            "weixin://biz/finder/openFinderFeed/{}",
            wrapper.query().unwrap().trim_start_matches("v=")
        ),
        expires_at_ms: expires,
    })
}

pub(crate) async fn handoff(url: Url) -> Result<Handoff> {
    let _permit = super::FETCHES.try_acquire()?;
    let id = short_id(&url).ok_or_else(|| anyhow::anyhow!("not a Channels link"))?;
    let value = tokio::time::timeout(Duration::from_secs(9), fetch(&id)).await??;
    scene(
        &id,
        &value["data"]["sceneInfo"],
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_identity_and_scene_only() {
        assert_eq!(
            short_id(&Url::parse("https://weixin.qq.com/sph/Example_123").unwrap()).as_deref(),
            Some("Example_123")
        );
        for url in [
            "https://weixin.qq.com.evil.test/sph/a",
            "https://weixin.qq.com/sph/a/b",
            "https://channels.weixin.qq.com/finder-preview/pages/sph?id=a&id=b",
            "https://weixin.qq.com/sph/a#x",
        ] {
            assert!(short_id(&Url::parse(url).unwrap()).is_none());
        }
        let data =
            json!({"dynamicExportId":"export/abcdefgh123", "expiredTime":100, "entryScene":64});
        let target = scene("a", &data, 0).unwrap();
        assert!(target
            .launch_url
            .ends_with("exportId%3Dexport%2Fabcdefgh123%26actionType%3D0%26entryScene%3D64"));
        assert!(scene("a", &data, 99000).is_err());
        assert!(scene(
            "a",
            &json!({"dynamicExportId":"export/a&evil=1", "expiredTime":100}),
            0
        )
        .is_err());
    }
    #[test]
    fn metadata_has_cover_not_fake_player_or_temporary_launch_key() {
        let url = Url::parse("https://weixin.qq.com/sph/example").unwrap();
        let preview = project(
            &url,
            &json!({"data":{"feedInfo":{"description":"Example video","coverUrl":"https://finder.video.qq.com/cover.jpg"},"authorInfo":{"nickname":"Author"},"sceneInfo":{"dynamicExportId":"export/not-for-cache"}}}),
        );
        assert_eq!(preview.author, "Author");
        assert!(preview.image.is_some());
        assert!(preview.embed.is_none());
        assert!(!serde_json::to_string(&preview)
            .unwrap()
            .contains("not-for-cache"));
    }
    #[tokio::test]
    #[ignore = "Explicit public sample network verification"]
    async fn live_public_preview_and_handoff() {
        let url = Url::parse("https://weixin.qq.com/sph/Aur6t4pfk3").unwrap();
        let preview = preview(url.clone()).await.unwrap();
        assert!(!preview.title.is_empty());
        assert!(preview.image.is_some());
        assert!(preview.cover_data_url.is_some());
        assert!(preview
            .author_avatar_data_url
            .as_ref()
            .is_some_and(|s| s.len() <= 12 * 1024));
        let handoff = handoff(url).await.unwrap();
        assert!(handoff
            .launch_url
            .starts_with("weixin://biz/finder/openFinderFeed/"));
        println!("channels live: metadata=true cover=true creator_avatar=true fresh_handoff=true");
    }
}
