//! One bounded, silent original-page reader for missing Douyin/XHS card covers.
//! Reuses the APK adapter. No arbitrary script, cookie export or request replay.
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use tauri::{webview::NewWindowResponse, AppHandle, Manager, Webview, WebviewBuilder, WebviewUrl};

static BUSY: AtomicBool = AtomicBool::new(false);
const LABEL: &str = "social-media-preview";
const SILENT: &str = include_str!("social_media_preview_silent.js");
const MOBILE_UA: &str = "Mozilla/5.0 (Linux; Android 15) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Mobile Safari/537.36";

fn source(raw: &str) -> Option<(tauri::Url, bool)> {
    if raw.len() > 4096 {
        return None;
    }
    let url = raw.parse::<tauri::Url>().ok()?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    let path = url.path().trim_end_matches('/');
    let id = path.rsplit('/').next()?;
    let mobile = match url.host_str()? {
        "www.douyin.com"
            if path == format!("/video/{id}")
                && (5..=24).contains(&id.len())
                && id.bytes().all(|b| b.is_ascii_digit()) =>
        {
            false
        }
        "www.xiaohongshu.com" | "xiaohongshu.com"
            if (path == format!("/explore/{id}") || path == format!("/discovery/item/{id}"))
                && id.len() == 24
                && id
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) =>
        {
            true
        }
        _ => return None,
    };
    Some((url, mobile))
}

struct Reader(Option<Webview>);
impl Drop for Reader {
    fn drop(&mut self) {
        if let Some(tab) = self.0.take() {
            let _ = tab.close();
        }
        BUSY.store(false, Ordering::Release);
    }
}

#[tauri::command]
pub async fn get_social_media_read_preview(
    app: AppHandle,
    webview: Webview,
    url: String,
) -> Result<Option<serde_json::Value>, String> {
    crate::group_ai_worker::ensure_caller(&webview)?;
    let (target, mobile) = source(&url).ok_or("invalid_media_source")?;
    if BUSY
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("media_reader_busy".into());
    }
    let mut reader = Reader(None);
    let host = target.host_str().unwrap().to_string();
    let window = app
        .get_window(crate::MAIN_WINDOW_LABEL)
        .ok_or("main_window_unavailable")?;
    let profile = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "profile_unavailable")?
        .join("reading-tabs-profile");
    let mut builder = crate::browser_profile::persistent(
        WebviewBuilder::new(LABEL, WebviewUrl::External(target)),
        &profile,
    )
    .initialization_script(crate::internal_browser::read_preview::ADAPTER)
    .initialization_script(SILENT)
    .on_navigation(move |next| next.scheme() == "https" && next.host_str() == Some(host.as_str()))
    .on_new_window(|_, _| NewWindowResponse::Deny);
    if mobile {
        builder = builder.user_agent(MOBILE_UA);
    }
    let tab = window
        .add_child(
            builder,
            tauri::LogicalPosition::new(-20000.0, -20000.0),
            tauri::LogicalSize::new(if mobile { 393.0 } else { 1280.0 }, 900.0),
        )
        .map_err(|_| "media_reader_unavailable")?;
    reader.0 = Some(tab.clone());
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(35) {
        if let Some(value) = crate::internal_browser::read_preview::read(&tab, &url).await {
            if value["image"].as_str().is_some_and(|v| !v.is_empty()) {
                return Ok(Some(value));
            }
        }
        tauri::async_runtime::spawn_blocking(|| std::thread::sleep(Duration::from_millis(750)))
            .await
            .map_err(|_| "media_reader_interrupted")?;
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_matching_public_media_pages() {
        assert!(
            !source("https://www.douyin.com/video/7678952575260953882")
                .unwrap()
                .1
        );
        assert!(source("https://www.xiaohongshu.com/discovery/item/6a6e8951000000002402c81f?xsec_token=fixture").unwrap().1);
        for invalid in [
            "http://www.douyin.com/video/12345",
            "https://www.douyin.com.evil.test/video/12345",
            "https://user@www.douyin.com/video/12345",
            "https://www.douyin.com/video/12345/extra",
            "https://www.douyin.com/video/12345#script",
            "https://127.0.0.1/video/12345",
        ] {
            assert!(source(invalid).is_none());
        }
    }
}
