//! Explicit user-requested handoff. HTTPS-only general navigation stays unchanged.
use std::{collections::HashSet, sync::mpsc, time::Duration};
use tauri::{Url, Webview};

const ADAPTER: &str =
    include_str!("../../../android/app/src/main/assets/wechat_channels_handoff.js");

fn trusted_source(url: &Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && url.host_str() == Some("channels.weixin.qq.com")
        && url.path() == "/finder-preview/pages/sph"
        && url.fragment().is_none()
        && url.query_pairs().filter(|(k, _)| k == "id").count() == 1
        && url.query_pairs().any(|(k, v)| {
            k == "id"
                && !v.is_empty()
                && v.len() <= 128
                && v.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        })
}

pub(crate) fn valid_handoff(source: &Url, target: &Url) -> bool {
    if !trusted_source(source)
        || target.as_str().len() > 4096
        || target.scheme() != "weixin"
        || target.host_str() != Some("biz")
        || !target.username().is_empty()
        || target.password().is_some()
        || target.port().is_some()
        || target.query().is_some()
        || target.fragment().is_some()
    {
        return false;
    }
    let Some(encoded) = target.path().strip_prefix("/finder/openFinderFeed/") else {
        return false;
    };
    // Decode exactly once, then validate the official, fixed feed parameter contract.
    let Ok(wrapper) = Url::parse(&format!("https://local.invalid/?v={encoded}")) else {
        return false;
    };
    let pairs: Vec<_> = wrapper.query_pairs().collect();
    if pairs.len() != 1 {
        return false;
    }
    let decoded = &pairs[0].1;
    let mut seen = HashSet::new();
    for pair in decoded.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            return false;
        };
        if !seen.insert(key) {
            return false;
        }
        let valid = match key {
            "exportId" => value.strip_prefix("export/").is_some_and(|id| {
                (8..=2048).contains(&id.len())
                    && id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            }),
            "actionType" => value == "0",
            "commentScene" | "entryScene" | "entryCardType" | "requestScene" => {
                !value.is_empty()
                    && value.bytes().all(|b| b.is_ascii_digit())
                    && value.parse::<u32>().is_ok_and(|n| n <= 1_000_000)
            }
            _ => false,
        };
        if !valid {
            return false;
        }
    }
    seen.contains("exportId") && seen.contains("actionType")
}

async fn evaluate(tab: &Webview, script: String) -> Result<serde_json::Value, String> {
    let (tx, rx) = mpsc::sync_channel(1);
    tab.eval_with_callback(script, move |value| {
        let _ = tx.try_send(value);
    })
    .map_err(|_| "网页读取失败，请刷新后重试。".to_string())?;
    let encoded =
        tauri::async_runtime::spawn_blocking(move || rx.recv_timeout(Duration::from_secs(2)))
            .await
            .ok()
            .and_then(Result::ok)
            .filter(|s| s.len() <= 8192)
            .ok_or_else(|| "视频号页面尚未就绪，请稍后重试。".to_string())?;
    serde_json::from_str(&encoded).map_err(|_| "视频号页面读取失败。".to_string())
}

fn current(runtime: &super::InternalBrowserRuntime, id: &str, source: &Url) -> bool {
    runtime
        .snapshot(id)
        .is_ok_and(|state| state.visible && state.current_url == source.as_str())
}

pub(super) async fn open(
    tab: &Webview,
    runtime: &super::InternalBrowserRuntime,
    id: &str,
) -> Result<(), String> {
    let source = tab.url().map_err(|_| "阅读页面已关闭。".to_string())?;
    if !trusted_source(&source) || !current(runtime, id, &source) {
        return Err("请等待视频号预览加载完成后重试。".into());
    }
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = format!(
        "handoff-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let args = format!(
        "{},{}",
        serde_json::to_string(source.as_str()).unwrap(),
        serde_json::to_string(&nonce).unwrap()
    );
    let mut result = evaluate(tab, format!("{ADAPTER}.start({args})")).await?;
    for _ in 0..35 {
        if result["status"] != "pending" {
            break;
        }
        let _ =
            tauri::async_runtime::spawn_blocking(|| std::thread::sleep(Duration::from_millis(200)))
                .await;
        if tab.url().ok().as_ref() != Some(&source)
            || !current(runtime, id, &source)
            || !tab.window().is_focused().unwrap_or(false)
        {
            return Err("网页已切换，请重新打开。".into());
        }
        result = evaluate(
            tab,
            format!(
                "window.ElonWechatChannelsHandoff.read({})",
                serde_json::to_string(&nonce).unwrap()
            ),
        )
        .await?;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let target = result["url"].as_str().and_then(|s| Url::parse(s).ok());
    if result["status"] != "ready"
        || result["nonce"] != nonce
        || result["source"] != source.as_str()
        || result["expiresAt"].as_u64().unwrap_or(0) <= now + 1000
        || tab.url().ok().as_ref() != Some(&source)
        || !current(runtime, id, &source)
        || !tab.window().is_focused().unwrap_or(false)
    {
        return Err("暂未取得有效的微信跳转链接，请重试或使用页面二维码。".into());
    }
    let target = target
        .filter(|url| valid_handoff(&source, url))
        .ok_or_else(|| "视频号跳转格式不受支持，请使用页面二维码。".to_string())?;
    crate::external_navigation::open_wechat_feed(&source, &target)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parked_closed_or_navigated_tabs_cancel_handoff() {
        let runtime = crate::internal_browser::InternalBrowserRuntime::default();
        let url = Url::parse("https://channels.weixin.qq.com/finder-preview/pages/sph?id=example")
            .unwrap();
        let mut state = crate::internal_browser::state_for("test", &url, "Video".into(), "main");
        state.visible = true;
        runtime.replace(state);
        assert!(current(&runtime, "test", &url));
        runtime.update("test", |state| state.visible = false);
        assert!(!current(&runtime, "test", &url));
        runtime.update("test", |state| {
            state.visible = true;
            state.current_url = "https://example.com/".into();
        });
        assert!(!current(&runtime, "test", &url));
        runtime.remove("test");
        assert!(!current(&runtime, "test", &url));
    }
    #[test]
    fn validates_only_observed_feed_protocol() {
        let source =
            Url::parse("https://channels.weixin.qq.com/finder-preview/pages/sph?id=example")
                .unwrap();
        let good = "weixin://biz/finder/openFinderFeed/exportId%3Dexport%2Fabcdefgh12345678%26actionType%3D0%26entryScene%3D64";
        assert!(valid_handoff(&source, &Url::parse(good).unwrap()));
        for bad in [
            good.replace("actionType%3D0", "actionType%3D1"),
            good.replace("biz/", "evil/"),
            format!("{good}%26actionType%3D0"),
            format!("{good}%26unknown%3D1"),
            format!("{good}#fragment"),
            good.replace("exportId%3D", "exportId%253D"),
        ] {
            assert!(!valid_handoff(&source, &Url::parse(&bad).unwrap()));
        }
        assert!(!valid_handoff(
            &Url::parse("https://channels.weixin.qq.com.evil.test/finder-preview/pages/sph?id=a")
                .unwrap(),
            &Url::parse(good).unwrap()
        ));
        assert!(!valid_handoff(
            &Url::parse("https://channels.weixin.qq.com/finder-preview/pages/sph?id=a&id=b")
                .unwrap(),
            &Url::parse(good).unwrap()
        ));
    }
}
