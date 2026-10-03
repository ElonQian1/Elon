//! Fixed private relay, exposed only to the main workbench, never to an exchange webview.
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Webview;

#[path = "../../../../server/src/grid_device_sources/model.rs"]
mod model;

#[tauri::command]
pub(crate) async fn grid_device_sources_request(
    webview: Webview,
    token: String,
    snapshot: Option<String>,
) -> Result<String, String> {
    super::ensure_main_webview(&webview)?;
    if token.is_empty() || token.len() > 8192 || !token.bytes().all(|b| (33..=126).contains(&b)) {
        return Err("请先登录一龙账号".into());
    }
    let origin = option_env!("ELON_GRID_ACCESS_ORIGIN").unwrap_or("https://43.139.149.158:8443");
    let mut endpoint = reqwest::Url::parse(origin).map_err(|_| "安全同步地址无效")?;
    if endpoint.scheme() != "https"
        || endpoint.host_str().is_none()
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
        || endpoint.path() != "/"
    {
        return Err("安全同步地址无效".into());
    }
    endpoint.set_path("/api/me/grid-device-sources");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "系统时间异常")?
        .as_millis() as u64;
    let body = snapshot
        .map(|raw| {
            let value = model::parse(raw.as_bytes(), now).map_err(|_| "网格同步数据无效")?;
            if value.platform != "windows" {
                return Err("只允许同步本机 Win 来源");
            }
            serde_json::to_string(&value).map_err(|_| "网格同步数据无效")
        })
        .transpose()
        .map_err(String::from)?;
    let limit = if body.is_some() {
        4096
    } else {
        model::MAX_TOTAL + 16384
    };
    let client = reqwest::Client::builder()
        .https_only(true)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| "安全连接未就绪")?;
    let mut request = if body.is_some() {
        client.post(endpoint)
    } else {
        client.get(endpoint)
    };
    request = request
        .bearer_auth(token)
        .header("Accept", "application/json")
        .header("Cache-Control", "no-store");
    if let Some(body) = body {
        request = request
            .header("Content-Type", "application/json")
            .body(body);
    }
    let mut response = request
        .send()
        .await
        .map_err(|_| "跨设备同步暂不可用，请检查网络")?;
    if response.status().as_u16() != 200 {
        return Err(if response.status().as_u16() == 401 {
            "一龙登录已失效，请重新登录"
        } else {
            "同步未确认，请检查来源状态或更新服务端"
        }
        .into());
    }
    let headers = response.headers();
    if headers.contains_key("set-cookie")
        || headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.split(';').next().unwrap_or("").trim())
            != Some("application/json")
        || !headers
            .get("cache-control")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| {
                v.split(',')
                    .any(|p| p.trim().eq_ignore_ascii_case("no-store"))
            })
        || response.content_length().is_some_and(|n| n > limit as u64)
    {
        return Err("无法验证同步响应".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "同步响应未完成")? {
        if bytes.len() + chunk.len() > limit {
            return Err("同步响应超出限制".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| "同步响应编码无效".into())
}
