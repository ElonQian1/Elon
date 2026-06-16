use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, io, path::PathBuf};

use super::{normalize_text, now_ms};

const MAX_MEMORY_ITEMS: usize = 40;
const MAX_MEMORY_SUMMARY_CHARS: usize = 4_000;
const MAX_MEMORY_FIX_CHARS: usize = 2_000;
const MAX_MEMORY_RESULT_CHARS: usize = 1_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DoctorMemoryItem {
    pub(super) id: String,
    pub(super) problem: String,
    pub(super) summary: String,
    pub(super) fix: Option<String>,
    pub(super) result: Option<String>,
    pub(super) created_at_ms: u64,
}

pub(super) fn save_memory_item(
    problem: &str,
    summary: &str,
    fix: Option<String>,
    result: Option<String>,
) -> io::Result<DoctorMemoryItem> {
    let mut items = read_memory_items().unwrap_or_default();
    let item = DoctorMemoryItem {
        id: format!("doctor-{}", now_ms()),
        problem: normalize_text(problem, super::MAX_ANALYSIS_PROBLEM_CHARS),
        summary: normalize_text(summary, MAX_MEMORY_SUMMARY_CHARS),
        fix: fix.map(|value| normalize_text(&value, MAX_MEMORY_FIX_CHARS)),
        result: result.map(|value| normalize_text(&value, MAX_MEMORY_RESULT_CHARS)),
        created_at_ms: now_ms(),
    };
    items.insert(0, item.clone());
    items.truncate(MAX_MEMORY_ITEMS);
    write_memory_items(&items)?;
    Ok(item)
}

pub(super) fn memory_path() -> PathBuf {
    if cfg!(windows) {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata)
                .join("elon-node-agent")
                .join("doctor_memory.json");
        }
    }

    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".config")
        .join("elon-node-agent")
        .join("doctor_memory.json")
}

pub(super) fn read_memory_items() -> io::Result<Vec<DoctorMemoryItem>> {
    let path = memory_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

pub(super) fn relevant_memories(
    problem: &str,
    items: &[DoctorMemoryItem],
) -> Vec<DoctorMemoryItem> {
    let tokens = tokens(problem);
    let mut scored = items
        .iter()
        .cloned()
        .map(|item| {
            let haystack = format!(
                "{} {} {}",
                item.problem,
                item.summary,
                item.fix.clone().unwrap_or_default()
            )
            .to_ascii_lowercase();
            let token_score = tokens
                .iter()
                .filter(|token| haystack.contains(token.as_str()))
                .count();
            let direct_score = if item.problem.contains(problem) || problem.contains(&item.problem)
            {
                4
            } else {
                0
            };
            (token_score + direct_score, item)
        })
        .collect::<Vec<_>>();

    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.created_at_ms.cmp(&a.1.created_at_ms))
    });

    let mut selected = scored
        .iter()
        .filter(|(score, _)| *score > 0)
        .take(6)
        .map(|(_, item)| item.clone())
        .collect::<Vec<_>>();
    if selected.is_empty() {
        selected = items.iter().take(4).cloned().collect();
    }
    selected
}

fn write_memory_items(items: &[DoctorMemoryItem]) -> io::Result<()> {
    let path = memory_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(items).unwrap_or_else(|_| "[]".to_string());
    fs::write(path, text)
}

fn tokens(value: &str) -> HashSet<String> {
    value
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(str::trim)
        .filter(|token| token.len() >= 2)
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relevant_memories_prefers_matching_tokens() {
        let items = vec![
            DoctorMemoryItem {
                id: "1".to_string(),
                problem: "DNS 解析失败".to_string(),
                summary: "flush dns fixed github".to_string(),
                fix: None,
                result: None,
                created_at_ms: 1,
            },
            DoctorMemoryItem {
                id: "2".to_string(),
                problem: "显示器亮度".to_string(),
                summary: "unrelated".to_string(),
                fix: None,
                result: None,
                created_at_ms: 2,
            },
        ];

        let selected = relevant_memories("github dns failed", &items);
        assert_eq!(selected[0].id, "1");
    }
}
