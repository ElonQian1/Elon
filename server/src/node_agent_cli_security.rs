// server/src/node_agent_cli_security.rs

use anyhow::{anyhow, bail, Result};
use homecli_proto::CliProjectContext;
use std::path::PathBuf;

const ROUTE_A_CLIS: &[&str] = &["codex", "copilot", "claude", "gemini"];
const BUILTIN_CLIS: &[&str] = &["api-runtime", "server-runtime"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolvedCli {
    BuiltIn { name: &'static str },
    External { name: &'static str, path: String },
}

impl ResolvedCli {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::BuiltIn { name } | Self::External { name, .. } => name,
        }
    }

    pub(crate) fn bin(&self) -> &str {
        match self {
            Self::BuiltIn { name } => name,
            Self::External { path, .. } => path,
        }
    }
}

pub(crate) fn resolve_cli_request(
    cli: &str,
    cli_paths: &[(String, String)],
) -> Result<ResolvedCli> {
    let name = normalize_cli_name(cli)?;
    if BUILTIN_CLIS.contains(&name) {
        return Ok(ResolvedCli::BuiltIn { name });
    }
    let Some(path) = cli_paths
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
        .map(|(_, path)| path)
    else {
        bail!("此 PC 节点没有发现可用的 {name} CLI，已拒绝执行。");
    };
    let full = canonical_cli_path(path)?;
    Ok(ResolvedCli::External {
        name,
        path: full.to_string_lossy().to_string(),
    })
}

pub(crate) fn prepare_cli_base_cwd(
    cwd: Option<String>,
    project_context: Option<CliProjectContext>,
) -> Result<(PathBuf, CliProjectContext)> {
    let context = project_context
        .ok_or_else(|| anyhow!("PC CLI 执行必须携带项目上下文，已拒绝裸 cwd/默认目录执行。"))?;
    let cwd = cwd
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("PC CLI 执行必须携带项目工作目录。"))?;
    let path = PathBuf::from(&cwd);
    if !path.is_absolute() {
        bail!("PC CLI 工作目录必须是绝对路径: {cwd}");
    }
    let full = std::fs::canonicalize(&path)
        .map_err(|error| anyhow!("PC CLI 工作目录不可用: {} ({error})", path.display()))?;
    if !full.is_dir() {
        bail!("PC CLI 工作目录不是目录: {}", full.display());
    }
    Ok((full, context))
}

pub(crate) fn validate_cli_extra_args(cli_name: &str, extra_args: &[String]) -> Result<()> {
    let mut index = 0;
    while index < extra_args.len() {
        let arg = extra_args[index].trim();
        reject_dangerous_arg(arg)?;
        match cli_name {
            "codex" => {
                if valid_prefix_arg(arg, "--session-id=")
                    || valid_prefix_arg(arg, "--codex-model=")
                    || valid_prefix_arg(arg, "--codex-effort=")
                {
                    index += 1;
                    continue;
                }
                if arg == "-i" {
                    validate_attachment_arg(extra_args.get(index + 1))?;
                    index += 2;
                    continue;
                }
            }
            "copilot" => {
                if valid_prefix_arg(arg, "--session-id=") {
                    index += 1;
                    continue;
                }
                if arg == "--model" {
                    validate_simple_value("model", extra_args.get(index + 1))?;
                    index += 2;
                    continue;
                }
                if arg == "--attachment" {
                    validate_attachment_arg(extra_args.get(index + 1))?;
                    index += 2;
                    continue;
                }
            }
            "claude" | "gemini" => {
                if arg == "--attachment" {
                    validate_attachment_arg(extra_args.get(index + 1))?;
                    index += 2;
                    continue;
                }
            }
            "api-runtime" | "server-runtime" => {
                if extra_args.is_empty() {
                    return Ok(());
                }
            }
            _ => {}
        }
        bail!("{cli_name} 不允许的 CLI 参数: {arg}");
    }
    Ok(())
}

pub(crate) fn codex_session_scope_key(
    extra_args: &[String],
    runtime_permission: Option<&str>,
    cwd: Option<&str>,
) -> Option<String> {
    let base = extra_args
        .iter()
        .find_map(|arg| arg.strip_prefix("--session-id="))
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let permission = runtime_permission
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("read_only");
    let cwd_hash = cwd.map(stable_hash).unwrap_or_else(|| "no-cwd".to_string());
    Some(format!("{base}|perm={permission}|cwd={cwd_hash}"))
}

fn normalize_cli_name(cli: &str) -> Result<&'static str> {
    let clean = cli.trim().to_ascii_lowercase();
    if clean.is_empty()
        || clean.contains('/')
        || clean.contains('\\')
        || clean.contains(':')
        || clean.contains('\0')
        || clean.contains(char::is_whitespace)
    {
        bail!("不合法的 CLI 名称: {cli}");
    }
    ROUTE_A_CLIS
        .iter()
        .chain(BUILTIN_CLIS.iter())
        .copied()
        .find(|name| *name == clean)
        .ok_or_else(|| anyhow!("不支持的 PC CLI: {cli}"))
}

fn canonical_cli_path(path: &str) -> Result<PathBuf> {
    let raw = PathBuf::from(path.trim());
    if !raw.is_absolute() {
        bail!("CLI 路径不是绝对路径: {}", raw.display());
    }
    let full = std::fs::canonicalize(&raw)
        .map_err(|error| anyhow!("CLI 路径不可用: {} ({error})", raw.display()))?;
    if !full.is_file() {
        bail!("CLI 路径不是文件: {}", full.display());
    }
    Ok(full)
}

fn reject_dangerous_arg(arg: &str) -> Result<()> {
    if arg.is_empty() || arg.contains('\0') || arg.contains('\n') || arg.contains('\r') {
        bail!("CLI 参数为空或包含非法字符。");
    }
    let lower = arg.to_ascii_lowercase();
    let blocked = [
        "--allow-all",
        "--dangerously-bypass-approvals-and-sandbox",
        "--approval-mode",
        "--sandbox",
        "resume",
        "exec",
    ];
    if blocked
        .iter()
        .any(|item| lower == *item || lower.starts_with(&format!("{item}=")))
    {
        bail!("拒绝危险 CLI 参数: {arg}");
    }
    Ok(())
}

fn valid_prefix_arg(arg: &str, prefix: &str) -> bool {
    arg.strip_prefix(prefix)
        .is_some_and(|value| valid_simple_token(value, 160))
}

fn validate_simple_value(label: &str, value: Option<&String>) -> Result<()> {
    let Some(value) = value.map(|value| value.trim()) else {
        bail!("{label} 参数缺少值");
    };
    if valid_simple_token(value, 160) {
        Ok(())
    } else {
        bail!("{label} 参数值不合法");
    }
}

fn valid_simple_token(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':' | '/' | '@'))
}

fn validate_attachment_arg(value: Option<&String>) -> Result<()> {
    let Some(value) = value.map(|value| value.trim()) else {
        bail!("attachment 参数缺少文件路径");
    };
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        bail!("attachment 必须是节点下载生成的临时文件");
    }
    let file_name = path
        .file_name()
        .and_then(|item| item.to_str())
        .unwrap_or_default();
    if !file_name.starts_with("elon_attach_") {
        bail!("attachment 文件不是节点下载生成的临时文件");
    }
    let temp_dir = std::env::temp_dir();
    if !path.starts_with(&temp_dir) {
        bail!("attachment 文件不在系统临时目录");
    }
    Ok(())
}

fn stable_hash(value: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let hash = hasher.finalize();
    hex::encode(&hash[..8])
}

#[cfg(test)]
mod tests {
    use super::{
        codex_session_scope_key, prepare_cli_base_cwd, resolve_cli_request,
        validate_cli_extra_args, ResolvedCli,
    };

    #[test]
    fn rejects_unknown_cli_even_when_cloud_sends_executable_name() {
        assert!(resolve_cli_request("powershell", &[]).is_err());
        assert!(resolve_cli_request("C:\\Windows\\System32\\cmd.exe", &[]).is_err());
    }

    #[test]
    fn built_in_runtime_does_not_need_local_binary_path() {
        assert_eq!(
            resolve_cli_request("api-runtime", &[]).unwrap(),
            ResolvedCli::BuiltIn {
                name: "api-runtime"
            }
        );
    }

    #[test]
    fn cli_prompt_requires_project_context_and_absolute_cwd() {
        assert!(prepare_cli_base_cwd(None, None).is_err());
        assert!(prepare_cli_base_cwd(Some("relative".to_string()), None).is_err());
    }

    #[test]
    fn extra_args_reject_privilege_escalation_flags() {
        assert!(validate_cli_extra_args("copilot", &["--allow-all".to_string()]).is_err());
        assert!(validate_cli_extra_args(
            "codex",
            &["--dangerously-bypass-approvals-and-sandbox".to_string()]
        )
        .is_err());
        assert!(validate_cli_extra_args(
            "codex",
            &["--sandbox".to_string(), "danger-full-access".to_string()]
        )
        .is_err());
    }

    #[test]
    fn extra_args_allow_expected_model_and_session_flags() {
        assert!(validate_cli_extra_args(
            "codex",
            &[
                "--codex-model=gpt-5.4".to_string(),
                "--codex-effort=medium".to_string(),
                "--session-id=abc-123".to_string()
            ]
        )
        .is_ok());
        assert!(validate_cli_extra_args(
            "copilot",
            &[
                "--session-id=abc-123".to_string(),
                "--model".to_string(),
                "gpt-5.4".to_string()
            ]
        )
        .is_ok());
    }

    #[test]
    fn codex_session_key_includes_permission_and_cwd_scope() {
        let args = vec!["--session-id=thread-1".to_string()];
        let project = codex_session_scope_key(&args, Some("project_write"), Some("C:/repo"));
        let full = codex_session_scope_key(&args, Some("full_access"), Some("C:/repo"));
        assert_ne!(project, full);
        assert!(full.unwrap().contains("perm=full_access"));
    }
}
