//! Fixed public metadata endpoint, without account credentials or arbitrary outbound URLs.
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

static CACHE: LazyLock<Mutex<HashMap<String, (Instant, Option<Value>)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
const LIMIT: usize = 256 * 1024;

fn valid_id(id: &str) -> bool {
    id.len() == 12 && id.starts_with("BV") && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

fn parse(id: &str, value: &Value) -> Option<Value> {
    if !valid_id(id) || value["code"].as_i64()? != 0 {
        return None;
    }
    let data = &value["data"];
    if data["bvid"].as_str()? != id {
        return None;
    }
    let title: String = data["title"].as_str()?.trim().chars().take(160).collect();
    if title.is_empty() {
        return None;
    }
    let mut image = reqwest::Url::parse(data["pic"].as_str()?).ok()?;
    if !matches!(image.scheme(), "http" | "https")
        || !image.username().is_empty()
        || image.password().is_some()
        || image.port().is_some()
    {
        return None;
    }
    let host = image.host_str()?;
    if !(host == "hdslb.com" || host.ends_with(".hdslb.com"))
        || !image.path().starts_with("/bfs/archive/")
        || image.as_str().len() > 4096
    {
        return None;
    }
    image.set_scheme("https").ok()?;
    let author: String = data["owner"]["name"]
        .as_str()
        .unwrap_or("")
        .trim()
        .chars()
        .take(80)
        .collect();
    Some(json!({"title": title, "author": author, "image": image.as_str()}))
}

async fn fetch(id: &str) -> Option<Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .ok()?;
    let mut response = client
        .get("https://api.bilibili.com/x/web-interface/view")
        .query(&[("bvid", id)])
        .send()
        .await
        .ok()?;
    if !response.status().is_success()
        || !response
            .headers()
            .get("content-type")?
            .to_str()
            .ok()?
            .starts_with("application/json")
        || response.content_length().is_some_and(|n| n > LIMIT as u64)
    {
        return None;
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if body.len() + chunk.len() > LIMIT {
            return None;
        }
        body.extend_from_slice(&chunk);
    }
    parse(id, &serde_json::from_slice(&body).ok()?)
}

#[tauri::command]
pub async fn get_bilibili_public_preview(
    webview: tauri::Webview,
    bvid: String,
) -> Result<Option<Value>, String> {
    crate::group_ai_worker::ensure_caller(&webview)?;
    if !valid_id(&bvid) {
        return Err("invalid_video_id".into());
    }
    if let Some((at, result)) = CACHE.lock().unwrap().get(&bvid) {
        if at.elapsed() < Duration::from_secs(if result.is_some() { 3600 } else { 30 }) {
            return Ok(result.clone());
        }
    }
    let result = fetch(&bvid).await;
    let mut cache = CACHE.lock().unwrap();
    if cache.len() >= 128 {
        if let Some(oldest) = cache
            .iter()
            .min_by_key(|(_, (at, _))| *at)
            .map(|(id, _)| id.clone())
        {
            cache.remove(&oldest);
        }
    }
    cache.insert(bvid, (Instant::now(), result.clone()));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requires_matching_video_and_archive_cdn() {
        let id = "BV19eYH6NEsC";
        let mut value = json!({"code":0,"data":{"bvid":id,"title":"Public video","pic":"http://i0.hdslb.com/bfs/archive/cover.jpg","owner":{"name":"Author"}}});
        assert_eq!(
            parse(id, &value).unwrap()["image"],
            "https://i0.hdslb.com/bfs/archive/cover.jpg"
        );
        for bad in [
            "https://hdslb.com.evil.test/bfs/archive/a.jpg",
            "https://i0.hdslb.com/bfs/face/a.jpg",
            "https://user@i0.hdslb.com/bfs/archive/a.jpg",
        ] {
            value["data"]["pic"] = json!(bad);
            assert!(parse(id, &value).is_none());
        }
        assert!(!valid_id("BV123/../../"));
        value["data"]["pic"] = json!("https://i0.hdslb.com/bfs/archive/a.jpg");
        value["data"]["bvid"] = json!("BV1BEY96vEjJ");
        assert!(parse(id, &value).is_none());
        value["data"]["bvid"] = json!(id);
        value["code"] = json!(-404);
        assert!(parse(id, &value).is_none());
    }
}
