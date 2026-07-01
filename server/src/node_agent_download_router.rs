use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const ROUTER_VERSION: &str = "0.1";
const DEFAULT_CACHE_MINUTES: u64 = 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadRouterProfile {
    enabled: bool,
    mode: String,
    fail_open: bool,
    cache_minutes: u64,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileUpdateRequest {
    enabled: Option<bool>,
    mode: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiConfigureRequest {
    confirm: Option<bool>,
    mode: Option<String>,
}

struct ProbeTarget {
    group: &'static str,
    name: &'static str,
    url: &'static str,
}

pub(crate) fn routes() -> Router<Arc<crate::NodeRuntime>> {
    Router::new()
        .route("/api/download-router/status", get(status_handler))
        .route("/api/download-router/profile", post(profile_handler))
        .route("/api/download-router/doctor", post(doctor_handler))
        .route(
            "/api/download-router/ai-configure",
            post(ai_configure_handler),
        )
}

pub(crate) fn status_payload() -> Value {
    let profile = read_profile();
    json!({
        "ok": true,
        "routerVersion": ROUTER_VERSION,
        "profile": profile,
        "profilePath": profile_path().display().to_string(),
        "traceScope": "project .elon/tool-router/traces",
        "wrapperPolicy": "PATH wrapper + fail-open fallback + ELON_ROUTER_BYPASS=1 emergency bypass",
        "availableModes": ["auto", "direct", "system_proxy", "off"],
    })
}

async fn status_handler(State(_rt): State<Arc<crate::NodeRuntime>>) -> Json<Value> {
    Json(status_payload())
}

async fn profile_handler(
    State(_rt): State<Arc<crate::NodeRuntime>>,
    Json(req): Json<ProfileUpdateRequest>,
) -> (StatusCode, Json<Value>) {
    let mut profile = read_profile();
    if let Some(enabled) = req.enabled {
        profile.enabled = enabled;
        if !enabled {
            profile.mode = "off".to_string();
        } else if profile.mode == "off" {
            profile.mode = "auto".to_string();
        }
    }
    if let Some(mode) = req
        .mode
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let Some(mode) = normalize_mode(mode) else {
            return json_status(
                StatusCode::BAD_REQUEST,
                json!({"ok": false, "error": "mode must be auto, direct, system_proxy, or off"}),
            );
        };
        profile.mode = mode.to_string();
        profile.enabled = mode != "off";
    }
    match write_profile(&profile) {
        Ok(()) => json_status(StatusCode::OK, status_payload()),
        Err(error) => json_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"ok": false, "error": format!("保存下载路由配置失败: {error}")}),
        ),
    }
}

async fn doctor_handler(State(_rt): State<Arc<crate::NodeRuntime>>) -> Json<Value> {
    let report = tokio::task::spawn_blocking(doctor_payload)
        .await
        .unwrap_or_else(
            |error| json!({"ok": false, "error": format!("下载路由诊断任务失败: {error}")}),
        );
    Json(report)
}

async fn ai_configure_handler(
    State(_rt): State<Arc<crate::NodeRuntime>>,
    Json(req): Json<AiConfigureRequest>,
) -> (StatusCode, Json<Value>) {
    let mode = req
        .mode
        .as_deref()
        .and_then(normalize_mode)
        .unwrap_or("auto");
    let recommendation = json!({
        "mode": mode,
        "enabled": mode != "off",
        "scope": "仅影响一龙启动的 AI/子项目进程，不修改系统 PATH 或系统代理",
        "reason": "AI 可通过此安全接口应用下载路由配置；真正写配置、校验和回滚由程序负责。",
    });
    if req.confirm != Some(true) {
        return json_status(
            StatusCode::CONFLICT,
            json!({
                "ok": false,
                "requiresConfirm": true,
                "recommendation": recommendation,
            }),
        );
    }
    let mut profile = read_profile();
    profile.mode = mode.to_string();
    profile.enabled = mode != "off";
    match write_profile(&profile) {
        Ok(()) => json_status(
            StatusCode::OK,
            json!({
                "ok": true,
                "message": "智能下载路由配置已由 AI 助手应用。",
                "recommendation": recommendation,
                "status": status_payload(),
            }),
        ),
        Err(error) => json_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"ok": false, "error": format!("AI 配置下载路由失败: {error}")}),
        ),
    }
}

fn doctor_payload() -> Value {
    let started = Instant::now();
    let profile = read_profile();
    let proxy = system_proxy_hint();
    let probes = probe_targets().into_iter().map(probe).collect::<Vec<_>>();
    let fastest_rust = fastest_name(&probes, "rust");
    let fastest_npm = fastest_name(&probes, "npm");
    json!({
        "ok": true,
        "schema": "elon.download-router.doctor.v1",
        "routerVersion": ROUTER_VERSION,
        "profile": profile,
        "profilePath": profile_path().display().to_string(),
        "systemProxy": proxy,
        "probes": probes,
        "recommendation": {
            "mode": "auto",
            "enabled": true,
            "rust": fastest_rust,
            "npm": fastest_npm,
            "summary": "建议保持自动模式；wrapper 会按项目进程注入最快源并记录失败诊断。"
        },
        "bypass": "ELON_ROUTER_BYPASS=1",
        "failOpen": true,
        "elapsedMs": started.elapsed().as_millis() as u64,
    })
}

fn probe(target: ProbeTarget) -> Value {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(6))
        .build()
        .unwrap_or_default();
    let started = Instant::now();
    match client.get(target.url).send() {
        Ok(response) => json!({
            "group": target.group,
            "name": target.name,
            "url": target.url,
            "ok": response.status().is_success(),
            "status": response.status().as_u16(),
            "elapsedMs": started.elapsed().as_millis() as u64,
        }),
        Err(error) => json!({
            "group": target.group,
            "name": target.name,
            "url": target.url,
            "ok": false,
            "error": error.to_string(),
            "elapsedMs": started.elapsed().as_millis() as u64,
        }),
    }
}

fn probe_targets() -> Vec<ProbeTarget> {
    vec![
        ProbeTarget {
            group: "rust",
            name: "rsproxy",
            url: "https://rsproxy.cn/dist/channel-rust-stable.toml",
        },
        ProbeTarget {
            group: "rust",
            name: "tuna",
            url: "https://mirrors.tuna.tsinghua.edu.cn/rustup/dist/channel-rust-stable.toml",
        },
        ProbeTarget {
            group: "rust",
            name: "official",
            url: "https://static.rust-lang.org/dist/channel-rust-stable.toml",
        },
        ProbeTarget {
            group: "npm",
            name: "npmmirror",
            url: "https://registry.npmmirror.com/npm",
        },
        ProbeTarget {
            group: "npm",
            name: "official",
            url: "https://registry.npmjs.org/npm",
        },
    ]
}

fn fastest_name(probes: &[Value], group: &str) -> Option<String> {
    probes
        .iter()
        .filter(|probe| probe.get("group").and_then(Value::as_str) == Some(group))
        .filter(|probe| probe.get("ok").and_then(Value::as_bool).unwrap_or(false))
        .min_by_key(|probe| {
            probe
                .get("elapsedMs")
                .and_then(Value::as_u64)
                .unwrap_or(u64::MAX)
        })
        .and_then(|probe| probe.get("name").and_then(Value::as_str))
        .map(ToOwned::to_owned)
}

fn default_profile() -> DownloadRouterProfile {
    DownloadRouterProfile {
        enabled: true,
        mode: "auto".to_string(),
        fail_open: true,
        cache_minutes: DEFAULT_CACHE_MINUTES,
        updated_at: now_isoish(),
    }
}

fn read_profile() -> DownloadRouterProfile {
    let path = profile_path();
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<DownloadRouterProfile>(&text).ok())
        .map(normalize_profile)
        .unwrap_or_else(default_profile)
}

fn write_profile(profile: &DownloadRouterProfile) -> std::io::Result<()> {
    let mut profile = normalize_profile(profile.clone());
    profile.updated_at = now_isoish();
    let path = profile_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&profile).unwrap_or_else(|_| "{}".to_string());
    std::fs::write(path, format!("{json}\n"))
}

fn normalize_profile(mut profile: DownloadRouterProfile) -> DownloadRouterProfile {
    let mode = normalize_mode(&profile.mode).unwrap_or("auto");
    profile.mode = mode.to_string();
    profile.enabled = profile.enabled && mode != "off";
    profile.fail_open = true;
    if profile.cache_minutes == 0 || profile.cache_minutes > 24 * 60 {
        profile.cache_minutes = DEFAULT_CACHE_MINUTES;
    }
    profile
}

fn normalize_mode(mode: &str) -> Option<&'static str> {
    match mode.trim().to_ascii_lowercase().as_str() {
        "auto" => Some("auto"),
        "direct" => Some("direct"),
        "system_proxy" | "system-proxy" | "proxy" => Some("system_proxy"),
        "off" | "disabled" | "disable" => Some("off"),
        _ => None,
    }
}

fn profile_path() -> PathBuf {
    config_root().join("download-router.json")
}

fn config_root() -> PathBuf {
    if cfg!(windows) {
        std::env::var("APPDATA")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("elon-node-agent")
    } else {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|home| PathBuf::from(home).join(".config"))
            })
            .unwrap_or_else(|| PathBuf::from("."))
            .join("elon-node-agent")
    }
}

fn system_proxy_hint() -> Value {
    json!({
        "httpProxyEnv": std::env::var("HTTP_PROXY").ok(),
        "httpsProxyEnv": std::env::var("HTTPS_PROXY").ok(),
        "allProxyEnv": std::env::var("ALL_PROXY").ok(),
        "noProxyEnv": std::env::var("NO_PROXY").ok(),
    })
}

fn now_isoish() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);
    format!("{millis}")
}

fn json_status(status: StatusCode, value: Value) -> (StatusCode, Json<Value>) {
    (status, Json(value))
}

#[cfg(test)]
mod tests {
    use super::{normalize_mode, normalize_profile, DownloadRouterProfile};

    #[test]
    fn normalize_mode_accepts_proxy_aliases() {
        assert_eq!(normalize_mode("proxy"), Some("system_proxy"));
        assert_eq!(normalize_mode("system-proxy"), Some("system_proxy"));
        assert_eq!(normalize_mode("off"), Some("off"));
        assert_eq!(normalize_mode("bad"), None);
    }

    #[test]
    fn profile_is_fail_open_and_bounded() {
        let profile = normalize_profile(DownloadRouterProfile {
            enabled: true,
            mode: "bad".to_string(),
            fail_open: false,
            cache_minutes: 99_999,
            updated_at: String::new(),
        });
        assert_eq!(profile.mode, "auto");
        assert!(profile.fail_open);
        assert_eq!(profile.cache_minutes, super::DEFAULT_CACHE_MINUTES);
    }
}
