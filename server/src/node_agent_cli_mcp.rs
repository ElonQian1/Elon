mod intent;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

const CONFIG_MARKER: &str = "\"mcpConfigPath\"";
const PROJECT_DOCS_TASK_MARKER: &str = "<elon-project-docs-task version=\"1\">";
// ui_prepare_debug_runtime may legitimately spend 15 minutes in Gradle and
// another 6 minutes in an OEM installer. Keep the MCP request alive across
// that bounded operation so Codex does not report a false tool timeout while
// the node continues installing and connecting in the background.
const UI_TUNER_MCP_TOOL_TIMEOUT_SECS: u64 = 1_500;

pub(crate) struct ProjectDocsMcpLaunchConfig {
    pub(crate) args: Vec<String>,
    pub(crate) env: Vec<(String, String)>,
}

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
                format!(
                    "mcp_servers.yilong_ui_live.tool_timeout_sec={UI_TUNER_MCP_TOOL_TIMEOUT_SECS}"
                ),
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
    let mut args = codex_mcp_config_args(prompt).unwrap_or_default();
    let cwd = cwd.map(str::trim).filter(|value| !value.is_empty());
    if prompt.contains("<elon-ui-design-task version=\"1\">")
        && !args
            .iter()
            .any(|arg| arg.contains("mcp_servers.yilong_ui_live.url"))
    {
        if let Some(cwd) = cwd {
            match crate::node_agent_android_live::mcp_descriptor_for_project(
                runtime.live_ui.as_ref(),
                cwd,
                crate::node_agent_admin_open::admin_port_from_env(),
            )
            .await
            {
                Ok(Some(descriptor)) => {
                    if let Some(config_path) = descriptor
                        .get("configPath")
                        .and_then(serde_json::Value::as_str)
                    {
                        let synthetic =
                            serde_json::json!({ "mcpConfigPath": config_path }).to_string();
                        if let Some(ui_args) = codex_mcp_config_args(&synthetic) {
                            args.extend(ui_args);
                        }
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::warn!(error = %error, "无法为 UI 设计任务自动准备 Live MCP");
                }
            }
        }
    }
    (!args.is_empty()).then_some(args)
}

pub(crate) fn project_docs_mcp_launch_config(
    prompt: &str,
    cwd: Option<&str>,
    cli_name: &str,
    host_port: u16,
) -> Option<ProjectDocsMcpLaunchConfig> {
    let full_governance = prompt.contains(PROJECT_DOCS_TASK_MARKER);
    let cli_name = cli_name.trim().to_ascii_lowercase();
    if !full_governance && (cli_name != "codex" || intent::skip_context_profile(prompt)) {
        return None;
    }
    let cwd = cwd?.trim();
    if cwd.is_empty() {
        return None;
    }
    let descriptor = if full_governance {
        crate::node_agent_project_docs_mcp::descriptor_for_project(cwd, host_port)
    } else {
        crate::node_agent_project_docs_mcp::descriptor_for_project_context(cwd, host_port)
    };
    match descriptor {
        Ok(descriptor) => {
            let receipt_descriptor = if full_governance {
                None
            } else {
                match crate::node_agent_project_docs_mcp::descriptor_for_project_receipt(
                    cwd, host_port,
                ) {
                    Ok(descriptor) => Some(descriptor),
                    Err(error) => {
                        tracing::warn!(error = %error, "无法为普通 Codex 任务准备项目理解回执 MCP");
                        return None;
                    }
                }
            };
            let feature_descriptor = if full_governance {
                None
            } else {
                match crate::node_agent_project_docs_mcp::descriptor_for_project_feature(
                    cwd, host_port,
                ) {
                    Ok(descriptor) => Some(descriptor),
                    Err(error) => {
                        tracing::warn!(error = %error, "无法为普通 Codex 任务准备功能需求 MCP");
                        return None;
                    }
                }
            };
            let win_control_descriptor = if cli_name == "codex" {
                match crate::node_agent_project_docs_mcp::descriptor_for_project_win_control(
                    cwd, host_port,
                ) {
                    Ok(descriptor) => Some(descriptor),
                    Err(error) => {
                        tracing::warn!(error = %error, "无法为 Codex 任务准备 Win 语义控制 MCP");
                        return None;
                    }
                }
            } else {
                None
            };
            let config_path = |provider: &str| {
                descriptor
                    .pointer(&format!("/configPaths/{provider}"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            };
            let mut config = ProjectDocsMcpLaunchConfig {
                args: Vec::new(),
                env: Vec::new(),
            };
            match cli_name.as_str() {
                "codex" => {
                    append_http_mcp_args(
                        &mut config.args,
                        if full_governance {
                            "yilong_project_docs"
                        } else {
                            "yilong_project_context"
                        },
                        descriptor.get("url")?.as_str()?,
                    );
                    if let Some(receipt_descriptor) = receipt_descriptor.as_ref() {
                        append_http_mcp_args(
                            &mut config.args,
                            "yilong_project_receipt",
                            receipt_descriptor.get("url")?.as_str()?,
                        );
                    }
                    if let Some(feature_descriptor) = feature_descriptor.as_ref() {
                        append_http_mcp_args(
                            &mut config.args,
                            "yilong_project_features",
                            feature_descriptor.get("url")?.as_str()?,
                        );
                    }
                    if let Some(win_control_descriptor) = win_control_descriptor.as_ref() {
                        append_http_mcp_args(
                            &mut config.args,
                            "yilong_win_control",
                            win_control_descriptor.get("url")?.as_str()?,
                        );
                    }
                    if !full_governance
                        && crate::node_agent_project_memory_hook_config::enabled(prompt)
                    {
                        match crate::node_agent_project_memory_hook_config::codex_config_args() {
                            Ok(args) => config.args.extend(args),
                            Err(error) => {
                                tracing::warn!(error = %error, "无法为普通 Codex 任务准备项目记忆 Hook");
                            }
                        }
                    }
                }
                "copilot" => config.args.push(format!(
                    "--additional-mcp-config=@{}",
                    config_path("copilot")?
                )),
                "claude" => {
                    config.args.push("--mcp-config".to_string());
                    config.args.push(config_path("claude")?);
                }
                "gemini" => config.env.push((
                    "GEMINI_CLI_SYSTEM_SETTINGS_PATH".to_string(),
                    config_path("gemini")?,
                )),
                _ => return None,
            }
            Some(config)
        }
        Err(error) => {
            tracing::warn!(
                error = %error,
                profile = if full_governance { "governance" } else { "context" },
                "无法为项目任务自动准备文档 MCP"
            );
            None
        }
    }
}

fn append_http_mcp_args(args: &mut Vec<String>, name: &str, url: &str) {
    let Ok(quoted_url) = serde_json::to_string(url) else {
        return;
    };
    args.extend([
        "-c".to_string(),
        format!("mcp_servers.{name}.url={quoted_url}"),
        "-c".to_string(),
        format!("mcp_servers.{name}.required=false"),
        "-c".to_string(),
        format!("mcp_servers.{name}.tool_timeout_sec=60"),
    ]);
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
    use super::{codex_mcp_config_args, project_docs_mcp_launch_config};
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
        assert!(args
            .iter()
            .any(|arg| arg == "mcp_servers.yilong_ui_live.tool_timeout_sec=1500"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn ignores_uncontrolled_path() {
        let prompt = r#"{"mcpConfigPath":"C:\\Windows\\win.ini"}"#;
        assert!(codex_mcp_config_args(prompt).is_none());
    }

    #[test]
    fn injects_minimal_profiles_and_hooks_for_plain_codex_but_full_mcp_for_document_tasks() {
        let root = std::env::temp_dir().join(format!(
            "elon_project_docs_cli_mcp_{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(root.join(".git")).unwrap();
        let plain = project_docs_mcp_launch_config("普通代码任务", root.to_str(), "codex", 7799);
        let plain = plain.unwrap();
        assert!(plain
            .args
            .iter()
            .any(|arg| arg.contains("mcp_servers.yilong_project_context.url")));
        assert!(plain.args.iter().any(|arg| arg.contains("profile=context")));
        assert!(plain
            .args
            .iter()
            .any(|arg| arg.contains("mcp_servers.yilong_project_receipt.url")));
        assert!(plain.args.iter().any(|arg| arg.contains("profile=receipt")));
        assert!(plain
            .args
            .iter()
            .any(|arg| arg.contains("mcp_servers.yilong_project_features.url")));
        assert!(plain.args.iter().any(|arg| arg.contains("profile=feature")));
        assert!(plain
            .args
            .iter()
            .any(|arg| arg.contains("mcp_servers.yilong_win_control.url")));
        assert!(plain
            .args
            .iter()
            .any(|arg| arg.contains("profile=win_control")));
        assert!(plain
            .args
            .iter()
            .any(|arg| arg.starts_with("hooks.PostToolUse=")));
        assert!(plain.args.iter().any(|arg| arg.starts_with("hooks.Stop=")));
        assert!(project_docs_mcp_launch_config(
            "请只修改 server/src/exact.rs:42 不要读取其他文件",
            root.to_str(),
            "codex",
            7799,
        )
        .is_none());
        assert!(
            project_docs_mcp_launch_config("普通代码任务", root.to_str(), "claude", 7799).is_none()
        );
        let config = project_docs_mcp_launch_config(
            "<elon-project-docs-task version=\"1\">",
            root.to_str(),
            "codex",
            7799,
        )
        .unwrap();
        assert!(config
            .args
            .iter()
            .any(|arg| arg.contains("mcp_servers.yilong_project_docs.url")));
        assert!(!config
            .args
            .iter()
            .any(|arg| arg.contains("yilong_project_receipt")));
        assert!(!config
            .args
            .iter()
            .any(|arg| arg.contains("yilong_project_features")));
        assert!(config
            .args
            .iter()
            .any(|arg| arg.contains("mcp_servers.yilong_win_control.url")));
        let copilot = project_docs_mcp_launch_config(
            "<elon-project-docs-task version=\"1\">",
            root.to_str(),
            "copilot",
            7799,
        )
        .unwrap();
        assert!(copilot.args[0].starts_with("--additional-mcp-config=@"));
        let claude = project_docs_mcp_launch_config(
            "<elon-project-docs-task version=\"1\">",
            root.to_str(),
            "claude",
            7799,
        )
        .unwrap();
        assert_eq!(claude.args[0], "--mcp-config");
        let gemini = project_docs_mcp_launch_config(
            "<elon-project-docs-task version=\"1\">",
            root.to_str(),
            "gemini",
            7799,
        )
        .unwrap();
        assert_eq!(gemini.env[0].0, "GEMINI_CLI_SYSTEM_SETTINGS_PATH");
        fs::remove_dir_all(root).unwrap();
    }
}
