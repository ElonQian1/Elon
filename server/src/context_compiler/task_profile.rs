use super::model::TaskProfile;

const MAX_KEYWORDS: usize = 24;
const MAX_SUSPECTED: usize = 12;

pub(crate) fn analyze_task(user_message: &str) -> TaskProfile {
    let keywords = extract_keywords(user_message);
    let suspected_files = extract_suspected_files(user_message);
    let suspected_symbols = extract_suspected_symbols(user_message);
    let likely_domains = infer_domains(user_message, &keywords, &suspected_files);
    let action_hints = infer_action_hints(user_message, &likely_domains);

    TaskProfile {
        keywords,
        likely_domains,
        suspected_symbols,
        suspected_files,
        action_hints,
    }
}

fn extract_keywords(user_message: &str) -> Vec<String> {
    let mut keywords = Vec::new();
    for raw in user_message
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '/'))
    {
        let term = raw.trim().trim_matches('/').to_ascii_lowercase();
        if term.len() < 3 || is_stop_word(&term) || keywords.contains(&term) {
            continue;
        }
        keywords.push(term);
        if keywords.len() >= MAX_KEYWORDS {
            break;
        }
    }
    keywords
}

fn extract_suspected_files(user_message: &str) -> Vec<String> {
    let mut files = Vec::new();
    for raw in user_message.split_whitespace() {
        let cleaned = raw.trim_matches(|ch: char| {
            matches!(
                ch,
                ',' | '，' | '.' | '。' | ':' | '：' | ';' | '；' | '"' | '\'' | '`'
            )
        });
        if !(cleaned.contains('/') || cleaned.contains('\\')) {
            continue;
        }
        let normalized = cleaned.replace('\\', "/");
        if normalized.len() < 5 || files.contains(&normalized) {
            continue;
        }
        files.push(normalized);
        if files.len() >= MAX_SUSPECTED {
            break;
        }
    }
    files
}

fn extract_suspected_symbols(user_message: &str) -> Vec<String> {
    let mut symbols = Vec::new();
    for raw in
        user_message.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == ':'))
    {
        let token = raw.trim_matches(':');
        if token.len() < 3 || symbols.contains(&token.to_string()) {
            continue;
        }
        if token.contains("::") || looks_like_symbol(token) {
            symbols.push(token.to_string());
        }
        if symbols.len() >= MAX_SUSPECTED {
            break;
        }
    }
    symbols
}

fn looks_like_symbol(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_uppercase() && chars.any(|ch| ch.is_ascii_lowercase())
}

fn infer_domains(
    user_message: &str,
    keywords: &[String],
    suspected_files: &[String],
) -> Vec<String> {
    let lower = user_message.to_ascii_lowercase();
    let mut domains = Vec::new();
    for (domain, terms) in [
        (
            "rust_context_compiler",
            &["repo", "map", "context", "symbol", "rust-analyzer"][..],
        ),
        ("server_backend", &["server", "api", "backend", "axum"]),
        ("android_client", &["android", "apk", "kotlin", "gradle"]),
        (
            "git_release",
            &["git", "worktree", "publish", "release", "deploy"],
        ),
        ("billing", &["billing", "quota", "ledger", "settle"]),
        ("voice", &["voice", "tts", "asr", "audio"]),
    ] {
        if terms.iter().any(|term| lower.contains(term))
            || keywords
                .iter()
                .any(|keyword| terms.contains(&keyword.as_str()))
            || suspected_files.iter().any(|path| path.contains(domain))
        {
            domains.push(domain.to_string());
        }
    }
    domains
}

fn infer_action_hints(user_message: &str, domains: &[String]) -> Vec<String> {
    let lower = user_message.to_ascii_lowercase();
    let mut hints = Vec::new();
    if lower.contains("重构") || lower.contains("refactor") {
        hints.push(
            "treat as refactor: include callers, public API, tests, and invariants".to_string(),
        );
    }
    if lower.contains("完善") || lower.contains("继续") || lower.contains("实现") {
        hints.push(
            "treat as implementation: prefer edit-target snippets and verification commands"
                .to_string(),
        );
    }
    if domains
        .iter()
        .any(|domain| domain == "rust_context_compiler")
    {
        hints.push(
            "include repo map, symbol graph, rust safety context, and missing-context policy"
                .to_string(),
        );
    }
    hints
}

fn is_stop_word(term: &str) -> bool {
    matches!(
        term,
        "the"
            | "and"
            | "for"
            | "with"
            | "this"
            | "that"
            | "功能"
            | "项目"
            | "实现"
            | "完善"
            | "继续"
            | "还有"
            | "哪些"
            | "没有"
            | "根据"
            | "文档"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_repo_map_task_profile() {
        let profile = analyze_task("继续完善 RepoMap 和 rust-analyzer，查看 server/src/context.rs");

        assert!(profile.keywords.contains(&"repomap".to_string()));
        assert!(profile
            .likely_domains
            .contains(&"rust_context_compiler".to_string()));
        assert!(profile
            .suspected_files
            .contains(&"server/src/context.rs".to_string()));
    }
}
