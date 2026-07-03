pub(crate) fn result_message(message: &str, apk_url: Option<&str>, status: Option<&str>) -> String {
    let mut parts = Vec::new();
    if let Some(status) = status {
        parts.push(format!("AI 开发任务{}。", status));
    }
    if !message.trim().is_empty() {
        parts.push(message.trim().to_string());
    }
    if let Some(apk_url) = apk_url.filter(|value| !value.is_empty()) {
        parts.push(format!("APK 下载：{}", apk_url));
    }
    parts.join("\n")
}
