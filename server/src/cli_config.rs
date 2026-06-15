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
    pub reasoning_effort: Option<String>,
    pub reasoning_summary: Option<String>,
    pub verbosity: Option<String>,
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

    pub fn display_label(&self) -> String {
        if self.provider.eq_ignore_ascii_case("codex") {
            return codex_profile_label(
                self.model.as_deref(),
                self.reasoning_effort.as_deref(),
                self.verbosity.as_deref(),
            );
        }
        self.model
            .as_deref()
            .map(str::trim)
            .filter(|model| !model.is_empty() && !model.eq_ignore_ascii_case("default"))
            .map(friendly_cli_model_name)
            .unwrap_or_else(|| self.label.clone())
    }

    pub fn attribution_label(&self) -> String {
        self.display_label()
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

        let fallback_cli_option = std::env::var("AI_CLI_FALLBACK").ok().filter(|id| {
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

    pub fn find_codex_option(&self, preferred_id: Option<&str>) -> Option<&AiCliOption> {
        if let Some(id) = preferred_id.filter(|value| !value.trim().is_empty()) {
            if let Some(option) = self
                .options
                .iter()
                .find(|opt| opt.id.eq_ignore_ascii_case(id) && is_codex_cli_option(opt))
            {
                return Some(option);
            }
        }

        self.default_option
            .as_deref()
            .and_then(|id| {
                self.options
                    .iter()
                    .find(|opt| opt.id.eq_ignore_ascii_case(id) && is_codex_cli_option(opt))
            })
            .or_else(|| self.options.iter().find(|opt| is_codex_cli_option(opt)))
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
    reasoning_effort: Option<String>,
    reasoning_summary: Option<String>,
    verbosity: Option<String>,
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
                    reasoning_effort: item.reasoning_effort,
                    reasoning_summary: item.reasoning_summary,
                    verbosity: item.verbosity,
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
    if models.is_empty() && prefix.eq_ignore_ascii_case("CODEX") {
        models = default_codex_models();
    }

    if models.is_empty() {
        return vec![AiCliOption {
            id: format!("{}_cli", provider),
            label,
            provider: provider.into(),
            model: None,
            reasoning_effort: None,
            reasoning_summary: None,
            verbosity: None,
            bin,
            args: split_cli_args(&args_raw),
            prompt_mode,
            timeout_secs,
        }];
    }

    let configured_efforts =
        split_list(&std::env::var(format!("{}_CLI_REASONING_EFFORTS", prefix)).unwrap_or_default());
    let default_reasoning_summary = std::env::var(format!("{}_CLI_REASONING_SUMMARY", prefix))
        .ok()
        .and_then(clean_optional);
    let default_verbosity = std::env::var(format!("{}_CLI_VERBOSITY", prefix))
        .ok()
        .and_then(clean_optional);

    models
        .into_iter()
        .flat_map(|model| {
            let efforts = if provider.eq_ignore_ascii_case("codex") {
                if configured_efforts.is_empty() {
                    default_codex_reasoning_efforts(&model)
                } else {
                    configured_efforts.clone()
                }
            } else {
                Vec::new()
            };
            let efforts: Vec<Option<String>> = if efforts.is_empty() {
                vec![None]
            } else {
                efforts.into_iter().map(Some).collect()
            };
            let default_label = label.clone();
            let provider = provider.to_string();
            let bin = bin.clone();
            let args_raw = args_raw.clone();
            let model_arg = model_arg.clone();
            let reasoning_summary = default_reasoning_summary.clone();
            let verbosity = default_verbosity.clone();
            efforts.into_iter().map(move |effort| {
                let id = if let Some(effort) = effort.as_deref() {
                    format!("{}:{}:{}", provider, model, effort)
                } else {
                    format!("{}:{}", provider, model)
                };
                let option_label = if provider.eq_ignore_ascii_case("codex") {
                    codex_profile_label(Some(&model), effort.as_deref(), verbosity.as_deref())
                } else {
                    format!("{} / {}", default_label, model)
                };
                AiCliOption {
                    id,
                    label: option_label,
                    provider: provider.clone(),
                    model: Some(model.clone()),
                    reasoning_effort: effort.clone(),
                    reasoning_summary: reasoning_summary.clone(),
                    verbosity: verbosity.clone(),
                    bin: bin.clone(),
                    args: args_for_model_config(
                        &args_raw,
                        &model,
                        &model_arg,
                        effort.as_deref(),
                        reasoning_summary.as_deref(),
                        verbosity.as_deref(),
                    ),
                    prompt_mode,
                    timeout_secs,
                }
            })
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

fn args_for_model_config(
    args_raw: &str,
    model: &str,
    model_arg: &str,
    reasoning_effort: Option<&str>,
    reasoning_summary: Option<&str>,
    verbosity: Option<&str>,
) -> Vec<String> {
    let args = args_for_model(args_raw, model, model_arg);
    with_codex_config_args(args, reasoning_effort, reasoning_summary, verbosity)
}

fn with_codex_config_args(
    mut args: Vec<String>,
    reasoning_effort: Option<&str>,
    reasoning_summary: Option<&str>,
    verbosity: Option<&str>,
) -> Vec<String> {
    let mut configs = Vec::new();
    if let Some(value) = reasoning_effort.and_then(clean_optional_str) {
        configs.extend([
            "-c".to_string(),
            toml_string_arg("model_reasoning_effort", value),
        ]);
    }
    if let Some(value) = reasoning_summary.and_then(clean_optional_str) {
        configs.extend([
            "-c".to_string(),
            toml_string_arg("model_reasoning_summary", value),
        ]);
    }
    if let Some(value) = verbosity.and_then(clean_optional_str) {
        configs.extend(["-c".to_string(), toml_string_arg("model_verbosity", value)]);
    }
    if configs.is_empty() {
        return args;
    }
    let insert_at = args
        .iter()
        .position(|arg| arg == "exec" || arg == "e")
        .unwrap_or(args.len());
    args.splice(insert_at..insert_at, configs);
    args
}

fn toml_string_arg(key: &str, value: &str) -> String {
    format!("{key}=\"{value}\"")
}

fn default_codex_models() -> Vec<String> {
    ["gpt-5.5", "gpt-5.4", "gpt-5.4-mini"]
        .into_iter()
        .map(ToString::to_string)
        .collect()
}

fn default_codex_reasoning_efforts(model: &str) -> Vec<String> {
    let model = model.trim().to_ascii_lowercase();
    let efforts: &[&str] = if model.contains("mini") || model.contains("spark") {
        &["low", "medium"]
    } else if model == "gpt-5.5" {
        &["high", "xhigh"]
    } else {
        &["medium", "high"]
    };
    efforts.iter().map(|value| (*value).to_string()).collect()
}

fn codex_profile_label(
    model: Option<&str>,
    reasoning_effort: Option<&str>,
    verbosity: Option<&str>,
) -> String {
    let model = model
        .and_then(clean_optional_str)
        .map(friendly_cli_model_name)
        .unwrap_or_else(|| "Codex 默认".to_string());
    let mut parts = vec![model];
    if let Some(effort) = reasoning_effort.and_then(clean_optional_str) {
        parts.push(format!("推理 {}", effort));
    }
    if let Some(verbosity) = verbosity.and_then(clean_optional_str) {
        parts.push(format!("输出 {}", verbosity));
    }
    parts.join(" · ")
}

fn friendly_cli_model_name(model: &str) -> String {
    match model.trim().to_ascii_lowercase().as_str() {
        "gpt-5.5" => "GPT-5.5".to_string(),
        "gpt-5.4" => "GPT-5.4".to_string(),
        "gpt-5.4-mini" => "GPT-5.4 mini".to_string(),
        "gpt-5.3-codex-spark" => "GPT-5.3 Codex Spark".to_string(),
        "gpt-5.3-codex" => "GPT-5.3 Codex".to_string(),
        "gpt-5.2" => "GPT-5.2".to_string(),
        "gpt-5" => "GPT-5".to_string(),
        _ => model.trim().to_string(),
    }
}

fn clean_optional(value: String) -> Option<String> {
    clean_optional_str(&value).map(ToOwned::to_owned)
}

fn clean_optional_str(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        None
    } else {
        Some(value)
    }
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
            usage_mode: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cli_option(id: &str, provider: &str, bin: &str) -> AiCliOption {
        AiCliOption {
            id: id.to_string(),
            label: id.to_string(),
            provider: provider.to_string(),
            model: None,
            reasoning_effort: None,
            reasoning_summary: None,
            verbosity: None,
            bin: bin.to_string(),
            args: Vec::new(),
            prompt_mode: CliPromptMode::Arg,
            timeout_secs: 60,
        }
    }

    fn cli_config(options: Vec<AiCliOption>, default_option: Option<&str>) -> AiCliConfig {
        AiCliConfig {
            enabled: true,
            options,
            default_option: default_option.map(ToString::to_string),
            fallback_to_api: false,
            codex_cli_only: false,
            fallback_cli_option: None,
        }
    }

    #[test]
    fn find_codex_option_keeps_explicit_codex_choice() {
        let config = cli_config(
            vec![
                cli_option("copilot:gpt-4o", "copilot", "copilot"),
                cli_option("codex:gpt-5.4:high", "codex", "codex"),
                cli_option("codex:gpt-5.5:xhigh", "codex", "codex"),
            ],
            Some("copilot:gpt-4o"),
        );

        let option = config
            .find_codex_option(Some("codex:gpt-5.5:xhigh"))
            .unwrap();

        assert_eq!(option.id, "codex:gpt-5.5:xhigh");
    }

    #[test]
    fn find_codex_option_falls_back_from_non_codex_choice() {
        let config = cli_config(
            vec![
                cli_option("copilot:gpt-4o", "copilot", "copilot"),
                cli_option("codex:gpt-5.4:high", "codex", "codex"),
            ],
            Some("copilot:gpt-4o"),
        );

        let option = config.find_codex_option(Some("copilot:gpt-4o")).unwrap();

        assert_eq!(option.id, "codex:gpt-5.4:high");
    }

    #[test]
    fn find_codex_option_prefers_default_codex_when_no_explicit_choice() {
        let config = cli_config(
            vec![
                cli_option("codex:gpt-5.4:medium", "codex", "codex"),
                cli_option("codex:gpt-5.5:xhigh", "codex", "codex"),
            ],
            Some("codex:gpt-5.5:xhigh"),
        );

        let option = config.find_codex_option(None).unwrap();

        assert_eq!(option.id, "codex:gpt-5.5:xhigh");
    }

    #[test]
    fn find_codex_option_returns_none_without_codex() {
        let config = cli_config(
            vec![cli_option("copilot:gpt-4o", "copilot", "copilot")],
            Some("copilot:gpt-4o"),
        );

        assert!(config.find_codex_option(Some("copilot:gpt-4o")).is_none());
    }
}
