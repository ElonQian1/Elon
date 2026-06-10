use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;

use super::config::ContextCompilerConfig;

#[derive(Debug, Clone)]
pub(crate) struct ContextPackArtifact {
    pub(crate) path: PathBuf,
    pub(crate) bytes: usize,
}

pub(crate) fn save_context_pack(
    data_dir: &Path,
    config: &ContextCompilerConfig,
    trace_id: Option<&str>,
    user_id: &str,
    pack: &str,
) -> Option<ContextPackArtifact> {
    if !config.save_pack_enabled {
        return None;
    }

    let now = Utc::now();
    let day = now.format("%Y%m%d").to_string();
    let stamp = now.format("%H%M%S%.3f").to_string().replace('.', "");
    let trace = trace_id
        .map(safe_component)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "no-trace".to_string());
    let user = safe_component(user_id);
    let dir = data_dir.join("context-compiler").join(day);
    let path = dir.join(format!("{stamp}-{trace}-{user}.md"));
    fs::create_dir_all(&dir).ok()?;

    let content = clamp_artifact(pack, config.artifact_max_bytes);
    fs::write(&path, content.as_bytes()).ok()?;
    Some(ContextPackArtifact {
        path,
        bytes: content.len(),
    })
}

fn clamp_artifact(pack: &str, max_bytes: usize) -> String {
    if pack.len() <= max_bytes {
        return pack.to_string();
    }
    let mut out = String::new();
    for ch in pack.chars() {
        if out.len() + ch.len_utf8() > max_bytes {
            break;
        }
        out.push(ch);
    }
    out.push_str(
        "\n\n<!-- context pack artifact truncated by ELON_CONTEXT_COMPILER_ARTIFACT_MAX_BYTES -->",
    );
    out
}

fn safe_component(value: &str) -> String {
    let mut out = value
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                Some(ch)
            } else {
                None
            }
        })
        .take(64)
        .collect::<String>();
    if out.is_empty() {
        out.push_str("unknown");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_compiler::config::ContextCompilerMode;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn saves_pack_under_data_dir() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "elon_context_artifact_{}_{}",
            std::process::id(),
            nonce
        ));
        let config = ContextCompilerConfig {
            enabled: true,
            mode: ContextCompilerMode::Shadow,
            agent_name: "hunyuan".to_string(),
            llm_brief_enabled: false,
            rust_analysis_enabled: true,
            rust_analyzer_enabled: true,
            max_relevant_files: 4,
            max_rust_files: 40,
            max_symbols: 20,
            max_relationships: 20,
            max_rust_analyzer_files: 2,
            max_pack_chars: 20_000,
            save_pack_enabled: true,
            artifact_max_bytes: 100_000,
            rust_probe_enabled: true,
        };

        let artifact =
            save_context_pack(&dir, &config, Some("trace/1"), "user@example", "hello").unwrap();

        assert!(artifact.path.starts_with(&dir));
        assert_eq!(fs::read_to_string(&artifact.path).unwrap(), "hello");
        assert!(artifact.path.to_string_lossy().contains("trace1"));

        fs::remove_dir_all(dir).unwrap();
    }
}
