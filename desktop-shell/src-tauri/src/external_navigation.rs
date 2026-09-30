use tauri::Url;

#[tauri::command]
pub fn open_wechat_feed_url(
    webview: tauri::Webview,
    source_url: String,
    launch_url: String,
    expires_at_ms: u64,
) -> Result<(), String> {
    if webview.label() != crate::MAIN_WINDOW_LABEL {
        return Err("仅允许一龙主窗口打开微信。".into());
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    if expires_at_ms <= now + 1000 {
        return Err("微信链接已过期，请重新点击。".into());
    }
    let source = Url::parse(&source_url).map_err(|_| "视频号来源无效。".to_string())?;
    let target = Url::parse(&launch_url).map_err(|_| "视频号跳转无效。".to_string())?;
    open_wechat_feed(&source, &target)
}

pub(crate) fn open_in_system_browser(url: &Url) -> Result<(), String> {
    validate_external_url(url)?;
    open_platform_url(url.as_str())
}

pub(crate) fn open_wechat_feed(source: &Url, target: &Url) -> Result<(), String> {
    if !crate::internal_browser::wechat::valid_handoff(source, target) {
        return Err("不支持的视频号跳转。".into());
    }
    open_platform_url(target.as_str())
        .map_err(|_| "未能调用微信，请安装或更新微信，或使用页面二维码。".into())
}

pub(crate) fn validate_external_url(url: &Url) -> Result<(), String> {
    if url.scheme() != "https" {
        return Err("只允许在系统浏览器打开 HTTPS 链接".to_string());
    }
    if url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() {
        return Err("外部链接格式无效".to_string());
    }
    Ok(())
}

pub(crate) fn safe_log_origin(url: &Url) -> String {
    match (url.scheme(), url.host_str(), url.port()) {
        (scheme, Some(host), Some(port)) => format!("{scheme}://{host}:{port}"),
        (scheme, Some(host), None) => format!("{scheme}://{host}"),
        _ => "invalid-url".to_string(),
    }
}

#[cfg(target_os = "windows")]
fn open_platform_url(value: &str) -> Result<(), String> {
    use std::{ffi::OsStr, iter, os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::{UI::Shell::ShellExecuteW, UI::WindowsAndMessaging::SW_SHOWNORMAL};

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value)
            .encode_wide()
            .chain(iter::once(0))
            .collect()
    }

    let operation = wide("open");
    let target = wide(value);
    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize <= 32 {
        return Err(format!(
            "Windows 默认浏览器启动失败（ShellExecute={result:?}）"
        ));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn open_platform_url(value: &str) -> Result<(), String> {
    let status = std::process::Command::new("/usr/bin/open")
        .arg(value)
        .status()
        .map_err(|_| "无法启动 macOS 系统浏览器".to_string())?;
    if !status.success() {
        return Err("macOS 系统浏览器拒绝打开链接".to_string());
    }
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn open_platform_url(_value: &str) -> Result<(), String> {
    Err("当前桌面壳只在 Windows 上提供系统浏览器跳转".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(value: &str) -> Url {
        value.parse().expect("valid test URL")
    }

    #[test]
    fn external_navigation_only_accepts_credential_free_https_urls() {
        assert!(validate_external_url(&url("https://example.com/weather?q=taipei")).is_ok());
        assert!(validate_external_url(&url("http://example.com/")).is_err());
        assert!(validate_external_url(&url("file:///C:/Windows/win.ini")).is_err());
        assert!(validate_external_url(&url("https://user:secret@example.com/")).is_err());
    }

    #[test]
    fn external_navigation_logs_only_the_origin() {
        assert_eq!(
            safe_log_origin(&url("https://example.com/private/path?q=secret#fragment")),
            "https://example.com"
        );
        assert_eq!(
            safe_log_origin(&url("https://example.com:8443/path")),
            "https://example.com:8443"
        );
    }
}
