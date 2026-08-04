use anyhow::{bail, Result};

pub(super) fn infer_platforms(
    intent: &str,
    explicit: Option<&str>,
    session: Option<&str>,
) -> Result<Vec<String>> {
    if let Some(platform) = explicit {
        if !matches!(platform, "web" | "pwa" | "tauri" | "android") {
            bail!("platform 无效");
        }
        return Ok(vec![platform.to_string()]);
    }
    let lower = intent.to_ascii_lowercase();
    let mut values = Vec::new();
    for (platform, markers) in [
        ("tauri", &["tauri", "桌面客户端", "桌面端应用"] as &[&str]),
        ("android", &["android", "安卓", "apk"]),
        ("pwa", &["pwa", "移动网页", "手机网页"]),
        ("web", &["web", "网页端", "浏览器端"]),
    ] {
        if markers.iter().any(|marker| lower.contains(marker)) {
            values.push(platform.to_string());
        }
    }
    if values.is_empty() {
        if let Some(platform) = session {
            values.push(platform.to_string());
        }
    }
    Ok(values)
}

pub(super) fn infer_route(intent: &str) -> Option<String> {
    intent.split_whitespace().find_map(|token| {
        let token =
            token.trim_matches(|ch: char| matches!(ch, '，' | '。' | ',' | ';' | ')' | ']' | '}'));
        (token.starts_with('/') && token.len() <= 2_048).then(|| token.to_string())
    })
}

pub(super) fn infer_states(intent: &str) -> Vec<String> {
    let lower = intent.to_ascii_lowercase();
    let mut values = Vec::new();
    for (state, markers) in [
        (
            "AUTHENTICATED",
            &["已登录", "authenticated", "signed in"] as &[&str],
        ),
        ("ANONYMOUS", &["未登录", "anonymous", "signed out"]),
        ("LOADING", &["加载状态", "loading"]),
        ("EMPTY", &["空状态", "empty state"]),
        ("ERROR", &["错误状态", "error state"]),
        ("DARK_THEME", &["暗色", "深色", "dark mode"]),
    ] {
        if markers.iter().any(|marker| lower.contains(marker)) {
            values.push(state.to_string());
        }
    }
    values
}

pub(super) fn normalize_route(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 2_048
        || value.chars().any(|ch| matches!(ch, '\0' | '\r' | '\n'))
        || !value.starts_with('/')
    {
        bail!("route 必须是以 / 开头的安全路径");
    }
    Ok(value.to_string())
}

pub(super) fn clean_summary(value: &str, max: usize) -> String {
    value.trim().chars().take(max).collect()
}

pub(super) fn push_unique(values: &mut Vec<String>, value: String) {
    if !value.is_empty() && !values.contains(&value) {
        values.push(value);
    }
}
