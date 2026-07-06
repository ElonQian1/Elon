use std::{fs, io::Read, path::Path};

use super::{
    repo_snapshot::{count_lines, is_source_file, relative_path, source_role},
    repo_walk,
};

const MAX_SCAN_FILES: usize = 1200;
const MAX_FILE_BYTES: u64 = 256 * 1024;
const MATCHES_PER_FILE: usize = 3;
const SEARCH_EXTENSIONS: &[&str] = &[
    "rs", "kt", "java", "ts", "tsx", "js", "jsx", "py", "go", "md", "toml", "gradle", "kts",
    "yaml", "yml", "json",
];

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RelevantFile {
    pub(crate) path: String,
    pub(crate) score: usize,
    pub(crate) lines: usize,
    pub(crate) role: &'static str,
    pub(crate) reasons: Vec<String>,
    pub(crate) matches: Vec<LineMatch>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct LineMatch {
    pub(crate) line: usize,
    pub(crate) text: String,
}

pub(crate) fn find_relevant_files(
    workspace: &Path,
    user_message: &str,
    limit: usize,
) -> Vec<RelevantFile> {
    let terms = extract_terms(user_message);
    if terms.is_empty() {
        return Vec::new();
    }
    let mut results = Vec::new();
    for path in repo_walk::collect_matching_files(workspace, MAX_SCAN_FILES, is_searchable_file) {
        let Some(result) = score_file(workspace, &path, &terms) else {
            continue;
        };
        if result.score > 0 {
            results.push(result);
        }
    }
    results.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.path.cmp(&right.path))
    });
    results.truncate(limit);
    results
}

fn score_file(base: &Path, path: &Path, terms: &[String]) -> Option<RelevantFile> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.len() > MAX_FILE_BYTES {
        return None;
    }
    let mut file = fs::File::open(path).ok()?;
    let mut content = String::new();
    file.read_to_string(&mut content).ok()?;

    let relative = relative_path(base, path);
    let path_lower = relative.to_ascii_lowercase();
    let mut score = 0usize;
    let mut reasons = Vec::new();
    let mut matches = Vec::new();

    for term in terms {
        if path_lower.contains(term) {
            score += 8;
            reasons.push(format!("path contains `{term}`"));
        }
    }

    for (idx, line) in content.lines().enumerate() {
        let lower = line.to_ascii_lowercase();
        let mut line_hits = 0usize;
        for term in terms {
            if lower.contains(term) {
                line_hits += 1;
            }
        }
        if line_hits == 0 {
            continue;
        }
        score += line_hits;
        if matches.len() < MATCHES_PER_FILE {
            matches.push(LineMatch {
                line: idx + 1,
                text: compact_line(line),
            });
        }
    }

    if matches.is_empty() && reasons.is_empty() {
        return None;
    }
    let lines = count_lines(path).unwrap_or_else(|| content.lines().count());
    let role = source_role(&relative);
    if is_source_file(path) {
        score += 2;
    }
    Some(RelevantFile {
        path: relative,
        score,
        lines,
        role,
        reasons,
        matches,
    })
}

fn is_searchable_file(path: &Path) -> bool {
    if is_source_file(path) {
        return true;
    }
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_ascii_lowercase();
            SEARCH_EXTENSIONS.iter().any(|item| lower == *item)
        })
        .unwrap_or(false)
}

fn extract_terms(user_message: &str) -> Vec<String> {
    let mut terms = Vec::new();
    for raw in user_message
        .split(|ch: char| !(ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '/'))
    {
        let term = raw.trim().trim_matches('/').to_ascii_lowercase();
        if term.len() < 3 || is_stop_word(&term) || terms.contains(&term) {
            continue;
        }
        terms.push(term);
        if terms.len() >= 16 {
            break;
        }
    }
    terms
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
            | "你们"
            | "我们"
            | "这个"
            | "那个"
            | "功能"
            | "项目"
            | "实现"
            | "修改"
            | "代码"
            | "怎么"
    )
}

fn compact_line(line: &str) -> String {
    let compact = line.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = compact.chars().take(180).collect::<String>();
    if compact.chars().count() > 180 {
        out.push_str("...");
    }
    out
}


#[cfg(test)]
#[path = "relevance_tests.rs"]
mod tests;
