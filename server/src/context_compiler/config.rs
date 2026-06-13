#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContextCompilerMode {
    Shadow,
    Inject,
}

impl ContextCompilerMode {
    pub(crate) fn from_env_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "inject" | "injected" | "on" => Self::Inject,
            _ => Self::Shadow,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Shadow => "shadow",
            Self::Inject => "inject",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ContextCompilerConfig {
    pub(crate) enabled: bool,
    pub(crate) mode: ContextCompilerMode,
    pub(crate) agent_name: String,
    pub(crate) llm_brief_enabled: bool,
    pub(crate) rust_analysis_enabled: bool,
    pub(crate) rust_analyzer_enabled: bool,
    pub(crate) rust_analyzer_probe_enabled: bool,
    pub(crate) rust_analyzer_probe_timeout_ms: usize,
    pub(crate) max_relevant_files: usize,
    pub(crate) max_rust_files: usize,
    pub(crate) max_symbols: usize,
    pub(crate) max_relationships: usize,
    pub(crate) max_rust_analyzer_files: usize,
    pub(crate) max_pack_chars: usize,
    pub(crate) save_pack_enabled: bool,
    pub(crate) artifact_max_bytes: usize,
    pub(crate) rust_probe_enabled: bool,
}

impl ContextCompilerConfig {
    pub(crate) fn from_env() -> Self {
        Self {
            enabled: env_bool("ELON_CONTEXT_COMPILER_ENABLED", false),
            mode: std::env::var("ELON_CONTEXT_COMPILER_MODE")
                .ok()
                .map(|value| ContextCompilerMode::from_env_value(&value))
                .unwrap_or(ContextCompilerMode::Shadow),
            agent_name: std::env::var("ELON_CONTEXT_COMPILER_AGENT")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "hunyuan".to_string()),
            llm_brief_enabled: env_bool("ELON_CONTEXT_COMPILER_LLM_BRIEF", true),
            rust_analysis_enabled: env_bool("ELON_CONTEXT_COMPILER_RUST_ANALYSIS", true),
            rust_analyzer_enabled: env_bool("ELON_CONTEXT_COMPILER_RUST_ANALYZER", true),
            rust_analyzer_probe_enabled: env_bool("ELON_CONTEXT_COMPILER_RA_PROBE", false),
            rust_analyzer_probe_timeout_ms: env_usize(
                "ELON_CONTEXT_COMPILER_RA_PROBE_TIMEOUT_MS",
                4_000,
            ),
            max_relevant_files: env_usize("ELON_CONTEXT_COMPILER_MAX_FILES", 8),
            max_rust_files: env_usize("ELON_CONTEXT_COMPILER_MAX_RUST_FILES", 400),
            max_symbols: env_usize("ELON_CONTEXT_COMPILER_MAX_SYMBOLS", 80),
            max_relationships: env_usize("ELON_CONTEXT_COMPILER_MAX_RELATIONSHIPS", 120),
            max_rust_analyzer_files: env_usize("ELON_CONTEXT_COMPILER_RA_FILES", 6),
            max_pack_chars: env_usize("ELON_CONTEXT_COMPILER_MAX_CHARS", 24_000),
            save_pack_enabled: env_bool("ELON_CONTEXT_COMPILER_SAVE_PACK", true),
            artifact_max_bytes: env_usize("ELON_CONTEXT_COMPILER_ARTIFACT_MAX_BYTES", 200_000),
            rust_probe_enabled: env_bool("ELON_CONTEXT_COMPILER_RUST_PROBE", true),
        }
    }
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mode_values() {
        assert_eq!(
            ContextCompilerMode::from_env_value("inject"),
            ContextCompilerMode::Inject
        );
        assert_eq!(
            ContextCompilerMode::from_env_value("shadow"),
            ContextCompilerMode::Shadow
        );
        assert_eq!(
            ContextCompilerMode::from_env_value("unknown"),
            ContextCompilerMode::Shadow
        );
    }
}
