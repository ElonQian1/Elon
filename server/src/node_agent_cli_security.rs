// server/src/node_agent_cli_security.rs

use anyhow::{anyhow, bail, Result};
use homecli_proto::CliProjectContext;
use std::path::{Path, PathBuf};

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
    // 当没有项目上下文时（如 AI 聊天模式），自动合成一个 chat 上下文：
    // runtime_permission = "full_access" 允许 copilot/codex 执行任意命令（无沙盒限制）。
    let context = project_context.unwrap_or_else(|| CliProjectContext {
        project_id: "chat".to_string(),
        conversation_id: format!(
            "chat-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos()
        ),
        runtime_permission: Some("full_access".to_string()),
    });

    // cwd 优先使用传入值；chat 模式下无 cwd 时回退到用户主目录
    let effective_cwd = cwd
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .or_else(|| {
            std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .ok()
        });

    let cwd_str = effective_cwd.ok_or_else(|| anyhow!("PC CLI 执行必须携带项目工作目录。"))?;
    let path = PathBuf::from(&cwd_str);
    if !path.is_absolute() {
        bail!("PC CLI 工作目录必须是绝对路径: {cwd_str}");
    }
    let full = canonical_cwd_path(&path)?;
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

#[allow(dead_code)]
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

pub(crate) fn windows_batch_wrapper(program: &str) -> Option<(&'static str, Vec<String>)> {
    if !cfg!(windows) || !is_windows_batch_file(program) {
        return None;
    }
    // 不手动加引号——Rust 的 Command::arg() 会在路径含空格时自动加引号。
    // 手动加引号会被 Rust 再次转义为 \" 导致 cmd.exe 解析失败。
    Some((
        "cmd",
        vec![
            "/D".to_string(),
            "/S".to_string(),
            "/C".to_string(),
            program.to_string(),
        ],
    ))
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

fn is_windows_batch_file(program: &str) -> bool {
    let lower = program.trim().to_ascii_lowercase();
    lower.ends_with(".cmd") || lower.ends_with(".bat")
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
    Ok(strip_windows_verbatim_prefix(full))
}

fn canonical_cwd_path(path: &Path) -> Result<PathBuf> {
    let full = std::fs::canonicalize(path)
        .map_err(|error| anyhow!("PC CLI 工作目录不可用: {} ({error})", path.display()))?;
    Ok(strip_windows_verbatim_prefix(full))
}

fn strip_windows_verbatim_prefix(path: PathBuf) -> PathBuf {
    // Windows canonicalize 返回 \\?\ 长路径前缀，但 cmd.exe 不接受该前缀，必须去掉。
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(stripped);
        }
    }
    path
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

#[allow(dead_code)]
fn stable_hash(value: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let hash = hasher.finalize();
    hex::encode(&hash[..8])
}

#[cfg(test)]
#[path = "node_agent_cli_security_tests.rs"]
mod tests;
