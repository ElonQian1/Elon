// server/src/node_agent_api_runtime_config.rs

use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::path::Path;

const DEFAULT_OPENAI_API_BASE: &str = "https://api.openai.com/v1";

const ROUTE_B_SUPPORTED_TOOLS: &[&str] = &[
    "list_dir",
    "search_files",
    "file_info",
    "read_file",
    "read_file_range",
    "git_status",
    "git_diff",
    "git_log",
    "write_file",
    "apply_patch",
    "run_command",
];
const ROUTE_B_APPROVAL_REQUIRED_TOOLS: &[&str] = &["write_file", "apply_patch", "run_command"];
const ROUTE_B_READ_ONLY_TOOLS: &[&str] = &[
    "list_dir",
    "search_files",
    "file_info",
    "read_file",
    "read_file_range",
    "git_status",
    "git_diff",
    "git_log",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ApiRuntimeEnvStatus {
    pub key_configured: bool,
    pub model_configured: bool,
    pub model: Option<String>,
    pub api_base: String,
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ApiRuntimeToolContract {
    pub route: String,
    pub label: String,
    pub mode: String,
    pub supported_tools: Vec<String>,
    pub read_only_tools: Vec<String>,
    pub approval_required_tools: Vec<String>,
    pub path_policy: String,
    pub command_policy: String,
    pub approval_policy: String,
    pub audit_policy: String,
    pub recovery_policy: String,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApiRuntimeSave {
    pub api_key: String,
    pub model: Option<String>,
    pub api_base: Option<String>,
}

pub(crate) fn status_from_env() -> ApiRuntimeEnvStatus {
    status_from_lookup(|name| std::env::var(name).ok())
}

pub(crate) fn status_from_lookup(lookup: impl Fn(&str) -> Option<String>) -> ApiRuntimeEnvStatus {
    let key = first_value(
        &lookup,
        &["ELON_AGENT_API_KEY", "OPENAI_API_KEY", "HUNYUAN_API_KEY"],
    );
    let model = first_value(
        &lookup,
        &["ELON_AGENT_MODEL", "OPENAI_MODEL", "HUNYUAN_MODEL"],
    );
    let api_base = first_value(
        &lookup,
        &[
            "ELON_AGENT_API_BASE",
            "OPENAI_API_BASE",
            "OPENAI_BASE_URL",
            "HUNYUAN_API_BASE",
        ],
    )
    .unwrap_or_else(|| DEFAULT_OPENAI_API_BASE.to_string());
    let key_configured = key.is_some();
    let model_configured = model.is_some();

    ApiRuntimeEnvStatus {
        key_configured,
        model_configured,
        model,
        api_base: normalize_api_base(&api_base),
        ready: key_configured && model_configured,
    }
}

pub(crate) fn tool_contract() -> ApiRuntimeToolContract {
    ApiRuntimeToolContract {
        route: "route_b_api_runtime".to_string(),
        label: "Route B · 本机 API runtime".to_string(),
        mode: "direct_provider_api".to_string(),
        supported_tools: strings(ROUTE_B_SUPPORTED_TOOLS),
        read_only_tools: strings(ROUTE_B_READ_ONLY_TOOLS),
        approval_required_tools: strings(ROUTE_B_APPROVAL_REQUIRED_TOOLS),
        path_policy: "workspace_relative_no_git_no_symlink_escape".to_string(),
        command_policy: "structured_project_command_allowlist".to_string(),
        approval_policy: "write_file_apply_patch_run_command_require_user_approval".to_string(),
        audit_policy: "tool_events_redact_content_and_secrets".to_string(),
        recovery_policy: "task_journal_replay_without_original_tty_reattach".to_string(),
        limitations: vec![
            "不能重新接管原 CLI TTY；任务恢复依赖 journal replay".to_string(),
            "文件访问默认限制在项目工作区内，不允许 .git 或符号链接逃逸".to_string(),
            "命令执行走结构化白名单，不是任意 shell".to_string(),
            "Route B 仍不是完整 Codex Desktop 级别 IDE runtime".to_string(),
        ],
    }
}

pub(crate) fn validate_save(
    api_key: &str,
    model: Option<&str>,
    api_base: Option<&str>,
) -> Result<ApiRuntimeSave> {
    let api_key = clean_required_env_value("API Key", api_key, 4096)?;
    let model = clean_optional_env_value("模型", model, 200)?;
    let api_base = clean_optional_env_value("API Base URL", api_base, 512)?
        .map(|value| validate_api_base(&value))
        .transpose()?;

    Ok(ApiRuntimeSave {
        api_key,
        model,
        api_base,
    })
}

pub(crate) fn apply_to_process(save: &ApiRuntimeSave) {
    // 同时写 ELON_AGENT_* 和 OPENAI_*：前者给内置 Route B 使用，后者兼容 Codex CLI。
    std::env::set_var("ELON_AGENT_API_KEY", &save.api_key);
    std::env::set_var("OPENAI_API_KEY", &save.api_key);
    if let Some(model) = &save.model {
        std::env::set_var("ELON_AGENT_MODEL", model);
        std::env::set_var("OPENAI_MODEL", model);
    }
    if let Some(api_base) = &save.api_base {
        std::env::set_var("ELON_AGENT_API_BASE", api_base);
        std::env::set_var("OPENAI_API_BASE", api_base);
        std::env::set_var("OPENAI_BASE_URL", api_base);
    }
}

pub(crate) fn persist_to_env_file(path: &Path, save: &ApiRuntimeSave) -> Result<()> {
    upsert_env_file(path, "ELON_AGENT_API_KEY", &save.api_key)?;
    upsert_env_file(path, "OPENAI_API_KEY", &save.api_key)?;
    if let Some(model) = &save.model {
        upsert_env_file(path, "ELON_AGENT_MODEL", model)?;
        upsert_env_file(path, "OPENAI_MODEL", model)?;
    }
    if let Some(api_base) = &save.api_base {
        upsert_env_file(path, "ELON_AGENT_API_BASE", api_base)?;
        upsert_env_file(path, "OPENAI_API_BASE", api_base)?;
        upsert_env_file(path, "OPENAI_BASE_URL", api_base)?;
    }
    Ok(())
}

/// 更新或追加 key=value 到 .env 文件（注释行也会被激活）。
pub(crate) fn upsert_env_file(path: &Path, key: &str, value: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("无法创建环境变量目录 {}", parent.display()))?;
    }
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let prefix = format!("{}=", key);
    let mut found = false;
    let new_lines: Vec<String> = existing
        .lines()
        .map(|line| {
            let stripped = line.trim_start_matches('#').trim_start();
            if stripped.starts_with(&prefix) {
                found = true;
                format!("{}={}", key, value)
            } else {
                line.to_string()
            }
        })
        .collect();

    let mut content = new_lines.join("\n");
    if !found {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!("{}={}\n", key, value));
    } else if !content.ends_with('\n') {
        content.push('\n');
    }
    std::fs::write(path, content)
        .with_context(|| format!("无法写入环境变量文件 {}", path.display()))
}

fn first_value(lookup: &impl Fn(&str) -> Option<String>, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        lookup(name)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn clean_required_env_value(label: &str, value: &str, max_len: usize) -> Result<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        bail!("{label} 不能为空");
    }
    validate_env_scalar(label, &value, max_len)?;
    Ok(value)
}

fn clean_optional_env_value(
    label: &str,
    value: Option<&str>,
    max_len: usize,
) -> Result<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    validate_env_scalar(label, value, max_len)?;
    Ok(Some(value.to_string()))
}

fn validate_env_scalar(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.len() > max_len {
        bail!("{label} 过长");
    }
    if value.contains('\n') || value.contains('\r') || value.contains('\0') {
        bail!("{label} 不能包含换行或 NUL 字符");
    }
    Ok(())
}

fn validate_api_base(value: &str) -> Result<String> {
    let clean = normalize_api_base(value);
    if !(clean.starts_with("https://") || clean.starts_with("http://")) {
        bail!("API Base URL 必须以 http:// 或 https:// 开头");
    }
    if clean
        .split("://")
        .nth(1)
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        bail!("API Base URL 不完整");
    }
    Ok(clean)
}

fn normalize_api_base(value: &str) -> String {
    value.trim().trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_requires_key_and_model_for_route_b_ready() {
        let missing_model = status_from_lookup(|name| match name {
            "OPENAI_API_KEY" => Some("sk-test".to_string()),
            _ => None,
        });
        assert!(missing_model.key_configured);
        assert!(!missing_model.model_configured);
        assert!(!missing_model.ready);

        let ready = status_from_lookup(|name| match name {
            "OPENAI_API_KEY" => Some("sk-test".to_string()),
            "OPENAI_MODEL" => Some("gpt-test".to_string()),
            _ => None,
        });
        assert!(ready.ready);
        assert_eq!(ready.api_base, DEFAULT_OPENAI_API_BASE);
    }

    #[test]
    fn validate_save_rejects_env_file_injection() {
        assert!(validate_save("sk-test\nOPENAI_MODEL=x", Some("gpt-test"), None).is_err());
        assert!(validate_save("sk-test", Some("gpt-test\rbad"), None).is_err());
    }

    #[test]
    fn validate_save_normalizes_api_base() {
        let save = validate_save(
            "sk-test",
            Some(" gpt-test "),
            Some(" https://example.test/v1/ "),
        )
        .expect("valid route b config");
        assert_eq!(save.model.as_deref(), Some("gpt-test"));
        assert_eq!(save.api_base.as_deref(), Some("https://example.test/v1"));
    }

    #[test]
    fn upsert_env_file_updates_commented_lines() {
        let path =
            std::env::temp_dir().join(format!("elon-api-runtime-env-{}.env", uuid::Uuid::new_v4()));
        std::fs::write(&path, "#OPENAI_MODEL=old\nOTHER=1\n").expect("seed env");
        upsert_env_file(&path, "OPENAI_MODEL", "new").expect("upsert env");
        let text = std::fs::read_to_string(&path).expect("read env");
        let _ = std::fs::remove_file(&path);
        assert!(text.contains("OPENAI_MODEL=new\n"));
        assert!(text.contains("OTHER=1"));
    }

    #[test]
    fn tool_contract_exposes_route_b_capabilities_and_guardrails() {
        let contract = tool_contract();
        assert_eq!(contract.route, "route_b_api_runtime");
        assert!(contract
            .supported_tools
            .contains(&"search_files".to_string()));
        assert!(contract.supported_tools.contains(&"file_info".to_string()));
        assert!(contract.read_only_tools.contains(&"file_info".to_string()));
        assert!(contract
            .supported_tools
            .contains(&"read_file_range".to_string()));
        assert!(contract.supported_tools.contains(&"git_status".to_string()));
        assert!(contract.supported_tools.contains(&"git_diff".to_string()));
        assert!(contract.supported_tools.contains(&"git_log".to_string()));
        assert!(contract.read_only_tools.contains(&"git_status".to_string()));
        assert!(contract.read_only_tools.contains(&"git_diff".to_string()));
        assert!(contract.read_only_tools.contains(&"git_log".to_string()));
        assert!(contract
            .supported_tools
            .contains(&"apply_patch".to_string()));
        assert!(contract
            .supported_tools
            .contains(&"run_command".to_string()));
        assert!(contract
            .approval_required_tools
            .contains(&"write_file".to_string()));
        assert!(contract
            .approval_required_tools
            .contains(&"apply_patch".to_string()));
        assert_eq!(
            contract.command_policy,
            "structured_project_command_allowlist"
        );
        assert!(contract
            .recovery_policy
            .contains("without_original_tty_reattach"));
    }
}
