use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

const CONFIG_MARKER: &str = "\"mcpConfigPath\"";

pub(crate) fn codex_mcp_config_args(prompt: &str) -> Option<Vec<String>> {
    match read_ui_tuner_mcp_url(prompt) {
        Ok(Some(url)) => {
            let quoted_url = serde_json::to_string(&url).ok()?;
            Some(vec![
                "-c".to_string(),
                format!("mcp_servers.yilong_ui_live.url={quoted_url}"),
                "-c".to_string(),
                "mcp_servers.yilong_ui_live.required=false".to_string(),
                "-c".to_string(),
                "mcp_servers.yilong_ui_live.tool_timeout_sec=60".to_string(),
            ])
        }
        Ok(None) => None,
        Err(error) => {
            tracing::warn!(error = %error, "忽略不安全的 ui-tuner MCP 配置");
            None
        }
    }
}

pub(crate) async fn codex_mcp_config_args_for_runtime(
    prompt: &str,
    cwd: Option<&str>,
    runtime: &crate::node_agent_runtime::NodeRuntime,
) -> Option<Vec<String>> {
    if let Some(args) = codex_mcp_config_args(prompt) {
        return Some(args);
    }
    if !prompt.contains("<elon-ui-design-task version=\"1\">") {
        return None;
    }
    let cwd = cwd?.trim();
    if cwd.is_empty() {
        return None;
    }
    let descriptor = match crate::node_agent_android_live::mcp_descriptor_for_project(
        runtime.live_ui.as_ref(),
        cwd,
        crate::node_agent_admin_open::admin_port_from_env(),
    )
    .await
    {
        Ok(Some(descriptor)) => descriptor,
        Ok(None) => return None,
        Err(error) => {
            tracing::warn!(error = %error, "无法为 UI 设计任务自动准备 Live MCP");
            return None;
        }
    };
    let config_path = descriptor.get("configPath")?.as_str()?;
    let synthetic = serde_json::json!({ "mcpConfigPath": config_path }).to_string();
    codex_mcp_config_args(&synthetic)
}

fn read_ui_tuner_mcp_url(prompt: &str) -> Result<Option<String>> {
    let Some(marker) = prompt.find(CONFIG_MARKER) else {
        return Ok(None);
    };
    let tail = &prompt[marker + CONFIG_MARKER.len()..];
    let Some(value_start) = tail.find(':') else {
        return Ok(None);
    };
    let value = tail[value_start + 1..].trim_start();
    if value.starts_with("null") {
        return Ok(None);
    }
    let mut deserializer = serde_json::Deserializer::from_str(value);
    let path = String::deserialize(&mut deserializer).context("mcpConfigPath JSON 值无效")?;
    let path = validate_config_path(&path)?;
    let bytes = fs::read(&path)
        .with_context(|| format!("读取 ui-tuner MCP 配置失败: {}", path.display()))?;
    if bytes.len() > 32 * 1024 {
        bail!("ui-tuner MCP 配置文件过大");
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    let url = value
        .pointer("/mcpServers/yilong_ui_live/url")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("ui-tuner MCP 配置缺少 URL"))?;
    validate_loopback_url(url)?;
    Ok(Some(url.to_string()))
}

fn validate_config_path(value: &str) -> Result<PathBuf> {
    let expected_root = std::env::temp_dir().join("elon-ui-tuner-live");
    let root = expected_root
        .canonicalize()
        .context("ui-tuner MCP 临时根目录不存在")?;
    let path = PathBuf::from(value)
        .canonicalize()
        .context("ui-tuner MCP 配置文件不存在")?;
    if !path.starts_with(&root)
        || path.file_name().and_then(|name| name.to_str()) != Some("mcp.json")
        || !path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("live_"))
            .unwrap_or(false)
    {
        bail!("ui-tuner MCP 配置路径不在受控会话目录");
    }
    Ok(path)
}

fn validate_loopback_url(value: &str) -> Result<()> {
    let url = reqwest::Url::parse(value).context("ui-tuner MCP URL 无效")?;
    let host = url.host_str().unwrap_or_default();
    if url.scheme() != "http"
        || !matches!(host, "127.0.0.1" | "localhost")
        || !url.path().starts_with("/api/android-live/mcp/live_")
        || !url
            .query_pairs()
            .any(|(key, value)| key == "token" && !value.is_empty())
    {
        bail!("ui-tuner MCP 只允许带会话令牌的本机 loopback URL");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::codex_mcp_config_args;
    use std::fs;

    #[test]
    fn injects_only_controlled_loopback_mcp() {
        let session = format!("live_{}", uuid::Uuid::new_v4().simple());
        let dir = std::env::temp_dir()
            .join("elon-ui-tuner-live")
            .join(&session);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mcp.json");
        let url = format!("http://127.0.0.1:7799/api/android-live/mcp/{session}?token=secret");
        fs::write(
            &path,
            serde_json::to_vec(&serde_json::json!({
                "mcpServers": { "yilong_ui_live": { "url": url } }
            }))
            .unwrap(),
        )
        .unwrap();
        let prompt = format!(
            "{{\"liveRuntime\":{{\"mcpConfigPath\":{}}}}}",
            serde_json::to_string(&path.display().to_string()).unwrap()
        );
        let args = codex_mcp_config_args(&prompt).unwrap();
        assert!(args
            .iter()
            .any(|arg| arg.contains("mcp_servers.yilong_ui_live.url")));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn ignores_uncontrolled_path() {
        let prompt = r#"{"mcpConfigPath":"C:\\Windows\\win.ini"}"#;
        assert!(codex_mcp_config_args(prompt).is_none());
    }
}
