//! 本地 AI CLI（Codex / Copilot 等）多选项配置与环境解析。
//!
//! 历史原因这块代码原本住在 `types.rs`，把环境变量解析、选项构造、
//! Copilot API 代理 agent 生成等无关 AppState 的逻辑全部塞在一起。
//! 本模块抽出来后 `types.rs` 只保留共享数据结构骨架。

use serde::Deserialize;

use crate::types::AgentConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliPromptMode {
    Arg,
    Stdin,
}

impl CliPromptMode {
    fn from_env_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "stdin" => Self::Stdin,
            _ => Self::Arg,
        }
    }
}

/// 一个可在 APK 中选择的本地 AI CLI 模型/后端选项。
#[derive(Debug, Clone)]
pub struct AiCliOption {
    pub id: String,
    pub label: String,
    pub provider: String,
    pub model: Option<String>,
    pub bin: String,
    pub args: Vec<String>,
    pub prompt_mode: CliPromptMode,
    pub timeout_secs: u64,
}

impl AiCliOption {
    pub fn command_preview(&self) -> String {
        let mut parts = vec![self.bin.clone()];
        parts.extend(self.args.clone());
        parts.push("<prompt>".into());
        parts.join(" ")
    }
}

/// 本地 AI CLI 配置，可同时挂 Codex CLI、Copilot CLI 等多个可选模型。
#[derive(Debug, Clone)]
pub struct AiCliConfig {
    pub enabled: bool,
    pub options: Vec<AiCliOption>,
    pub default_option: Option<String>,
    pub fallback_to_api: bool,
    /// 临时单通道模式：用户侧聊天、意图分析后的执行、代码协作都只走 Codex CLI。
    pub codex_cli_only: bool,
    /// 主 CLI 失败时自动切换的备用 CLI option_id（如 "codex_cli"）。
    /// 由环境变量 AI_CLI_FALLBACK 设置。
    pub fallback_cli_option: Option<String>,
}

impl AiCliConfig {
    pub(crate) fn from_env() -> Self {
        let codex_cli_only = env_bool("AI_CODEX_CLI_ONLY", true);
        let mut options = cli_options_from_json_env();
        options.extend(provider_cli_options(
            "CODEX",
            "codex",
            "Codex CLI",
            "codex",
            "exec --sandbox workspace-write --skip-git-repo-check",
            true,
            "-m",
        ));
        options.extend(provider_cli_options(
            "COPILOT",
            "copilot",
            "Copilot CLI",
            "copilot",
            "",
            true,
            "--model",
        ));
        if codex_cli_only {
            options.retain(is_codex_cli_option);
        }

        let enabled = env_bool("AI_CLI_ENABLED", true) && !options.is_empty();
        let default_option = std::env::var("AI_CLI_DEFAULT")
            .ok()
            .filter(|id| options.iter().any(|opt| opt.id == *id))
            .or_else(|| options.first().map(|opt| opt.id.clone()));

        let allow_api_fallback = env_bool("AI_ALLOW_API_FALLBACK", false);

        let fallback_cli_option = std::env::var("AI_CLI_FALLBACK")
            .ok()
            .filter(|id| {
                let id = id.trim().to_ascii_lowercase();
                !id.is_empty() && options.iter().any(|opt| opt.id.eq_ignore_ascii_case(&id))
            });

        Self {
            enabled,
            options,
            default_option,
            fallback_to_api: !codex_cli_only && allow_api_fallback,
            codex_cli_only,
            fallback_cli_option,
        }
    }

    pub fn find_option(&self, id: Option<&str>) -> Option<&AiCliOption> {
        let id = id
            .filter(|value| !value.trim().is_empty())
            .or(self.default_option.as_deref())?;
        self.options
            .iter()
            .find(|opt| opt.id.eq_ignore_ascii_case(id))
            .or_else(|| self.options.first())
    }

    pub fn has_option(&self, id: &str) -> bool {
        self.options
            .iter()
            .any(|opt| opt.id.eq_ignore_ascii_case(id))
    }
}

fn is_codex_cli_option(option: &AiCliOption) -> bool {
    let id = option.id.to_ascii_lowercase();
    let provider = option.provider.to_ascii_lowercase();
    let bin = option.bin.to_ascii_lowercase();
    id.contains("codex") || provider.contains("codex") || bin.contains("codex")
}

#[derive(Debug, Deserialize)]
struct AiCliOptionInput {
    id: String,
    label: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    bin: String,
    #[serde(default)]
    args: Vec<String>,
    args_text: Option<String>,
    prompt_mode: Option<String>,
    timeout_secs: Option<u64>,
}

fn cli_options_from_json_env() -> Vec<AiCliOption> {
    let raw = match std::env::var("AI_CLI_OPTIONS_JSON") {
        Ok(raw) if !raw.trim().is_empty() => raw,
        _ => return Vec::new(),
    };

    match serde_json::from_str::<Vec<AiCliOptionInput>>(&raw) {
        Ok(items) => items
            .into_iter()
            .filter_map(|item| {
                let args = if let Some(args_text) = item.args_text {
                    split_cli_args(&args_text)
                } else {
                    item.args
                };
                let provider = item.provider.unwrap_or_else(|| item.id.clone());
                let label = item.label.unwrap_or_else(|| {
                    item.model
                        .as_ref()
                        .map(|model| format!("{} / {}", provider, model))
                        .unwrap_or_else(|| provider.clone())
                });
                Some(AiCliOption {
                    id: item.id,
                    label,
                    provider,
                    model: item.model,
                    bin: item.bin,
                    args,
                    prompt_mode: CliPromptMode::from_env_value(
                        item.prompt_mode.as_deref().unwrap_or("arg"),
                    ),
                    timeout_secs: item.timeout_secs.unwrap_or(1800),
                })
            })
            .collect(),
        Err(e) => {
            tracing::warn!("AI_CLI_OPTIONS_JSON 解析失败: {}", e);
            Vec::new()
        }
    }
}

fn provider_cli_options(
    prefix: &str,
    provider: &str,
    default_label: &str,
    default_bin: &str,
    default_args: &str,
    default_enabled: bool,
    default_model_arg: &str,
) -> Vec<AiCliOption> {
    let enabled = env_bool(&format!("{}_CLI_ENABLED", prefix), default_enabled);
    if !enabled {
        return Vec::new();
    }

    let bin = std::env::var(format!("{}_CLI_BIN", prefix)).unwrap_or_else(|_| default_bin.into());
    let label =
        std::env::var(format!("{}_CLI_LABEL", prefix)).unwrap_or_else(|_| default_label.into());
    let args_raw =
        std::env::var(format!("{}_CLI_ARGS", prefix)).unwrap_or_else(|_| default_args.into());
    let prompt_mode = CliPromptMode::from_env_value(
        &std::env::var(format!("{}_CLI_PROMPT_MODE", prefix)).unwrap_or_else(|_| "arg".into()),
    );
    let timeout_secs = std::env::var(format!("{}_CLI_TIMEOUT_SECS", prefix))
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1800);
    let model_arg = std::env::var(format!("{}_CLI_MODEL_ARG", prefix))
        .unwrap_or_else(|_| default_model_arg.into());

    let mut models =
        split_list(&std::env::var(format!("{}_CLI_MODELS", prefix)).unwrap_or_default());
    if models.is_empty() {
        if let Ok(model) = std::env::var(format!("{}_CLI_MODEL", prefix)) {
            if !model.trim().is_empty() {
                models.push(model.trim().to_string());
            }
        }
    }

    if models.is_empty() {
        return vec![AiCliOption {
            id: format!("{}_cli", provider),
            label,
            provider: provider.into(),
            model: None,
            bin,
            args: split_cli_args(&args_raw),
            prompt_mode,
            timeout_secs,
        }];
    }

    models
        .into_iter()
        .map(|model| AiCliOption {
            id: format!("{}:{}", provider, model),
            label: format!("{} / {}", label, model),
            provider: provider.into(),
            model: Some(model.clone()),
            bin: bin.clone(),
            args: args_for_model(&args_raw, &model, &model_arg),
            prompt_mode,
            timeout_secs,
        })
        .collect()
}

fn args_for_model(args_raw: &str, model: &str, model_arg: &str) -> Vec<String> {
    if args_raw.contains("{model}") {
        return split_cli_args(&args_raw.replace("{model}", model));
    }

    let mut args = Vec::new();
    if !model_arg.trim().is_empty() {
        args.extend(split_cli_args(model_arg));
        args.push(model.to_string());
    }
    args.extend(split_cli_args(args_raw));
    args
}

fn split_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            !matches!(value.as_str(), "0" | "false" | "no" | "off")
        })
        .unwrap_or(default)
}

/// 从 COPILOT_GITHUB_TOKEN（或 GITHUB_TOKEN）+ COPILOT_MODELS 自动生成
/// GitHub Copilot / GitHub Models 多模型 API 代理配置。
///
/// 默认 API 地址：https://models.inference.ai.azure.com（OpenAI 兼容，无需特殊 header）
/// 若要用 GitHub Copilot 直连 API，设置 COPILOT_API_BASE=https://api.githubcopilot.com
/// 并配合 COPILOT_INTEGRATION_ID=vscode-chat（由 agent.rs 中的 call_llm 自动识别并添加 header）
pub(crate) fn copilot_api_agents() -> Vec<AgentConfig> {
    let token = std::env::var("COPILOT_GITHUB_TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .ok()
        .filter(|t| !t.trim().is_empty());
    let token = match token {
        Some(t) => t,
        None => return Vec::new(),
    };

    let api_base = std::env::var("COPILOT_API_BASE")
        .unwrap_or_else(|_| "https://models.inference.ai.azure.com".into());

    // 默认模型集合：与 VS Code Copilot 模型选择器一致
    let default_models = "gpt-4o,gpt-4o-mini,claude-3.5-sonnet,o1-mini";
    let models_raw = std::env::var("COPILOT_MODELS").unwrap_or_else(|_| default_models.into());

    split_list(&models_raw)
        .into_iter()
        .map(|model| AgentConfig {
            name: format!("copilot:{}", model),
            api_base: api_base.clone(),
            api_key: token.clone(),
            model,
        })
        .collect()
}

fn split_cli_args(raw: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in raw.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '"' | '\'' if quote == Some(ch) => quote = None,
            '"' | '\'' if quote.is_none() => quote = Some(ch),
            c if c.is_whitespace() && quote.is_none() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }

    if escaped {
        current.push('\\');
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}
