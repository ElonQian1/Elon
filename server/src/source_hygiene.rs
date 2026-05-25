//! Lightweight source-size preflight for APK-triggered project work.
//!
//! This keeps the "avoid giant files" rule cheap: the server scans line counts
//! and passes a compact summary to the CLI instead of asking the model to infer
//! the whole repository shape from broad file reads.

use std::{
    fs,
    io::{BufRead, BufReader},
    path::Path,
};

const MAX_SOURCE_FILES: usize = 3000;
const LARGE_FILE_REPORT_LIMIT: usize = 6;
const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "kt", "kts", "java", "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "go", "swift", "cs",
    "c", "cc", "cpp", "h", "hpp",
];
const SKIP_DIRS: &[&str] = &[
    ".git",
    ".gradle",
    ".idea",
    ".next",
    ".nuxt",
    ".venv",
    "build",
    "dist",
    "node_modules",
    "out",
    "target",
    "vendor",
];

#[derive(Debug, Clone)]
struct SourceStat {
    relative_path: String,
    lines: usize,
    role: &'static str,
}

pub(crate) fn source_size_preflight_note(workspace: &Path) -> Option<String> {
    let mut stats = Vec::new();
    collect_source_stats(workspace, workspace, &mut stats);
    if stats.is_empty() {
        return None;
    }

    let mut large: Vec<_> = stats.into_iter().filter(|stat| stat.lines > 500).collect();
    if large.is_empty() {
        return Some(
            "源文件体量预检：未发现超过 500 行的源码文件；仍按 <=500 / 501-800 / >800 规则做文件计划。"
                .to_string(),
        );
    }

    large.sort_by(|left, right| right.lines.cmp(&left.lines));
    let red_count = large.iter().filter(|stat| stat.lines > 800).count();
    let yellow_count = large.len().saturating_sub(red_count);
    let report = large
        .iter()
        .take(LARGE_FILE_REPORT_LIMIT)
        .map(|stat| {
            format!(
                "- {}: {} 行，{}，{}",
                stat.relative_path,
                stat.lines,
                stat.role,
                size_band(stat.lines)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    Some(format!(
        "源文件体量预检（低成本扫描，避免先写后拆）：\n- 规则：新建源文件目标 <=500 行；501-800 行仅单一职责可容忍；>800 行必须拆分；>1500 行旧文件除小修外不得加功能。\n- 当前超过 500 行的源码文件：黄区 {} 个，红区 {} 个；优先不要继续扩大红区文件。\n{}",
        yellow_count, red_count, report
    ))
}

fn collect_source_stats(base: &Path, dir: &Path, stats: &mut Vec<SourceStat>) {
    if stats.len() >= MAX_SOURCE_FILES {
        return;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if stats.len() >= MAX_SOURCE_FILES {
            return;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            if should_skip_dir(&path) {
                continue;
            }
            collect_source_stats(base, &path, stats);
            continue;
        }
        if !file_type.is_file() || !is_source_file(&path) {
            continue;
        }
        let Some(lines) = count_lines(&path) else {
            continue;
        };
        let relative_path = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        stats.push(SourceStat {
            role: source_role(&relative_path),
            relative_path,
            lines,
        });
    }
}

fn should_skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            SKIP_DIRS.iter().any(|skip| lower == *skip)
        })
        .unwrap_or(false)
}

fn is_source_file(path: &Path) -> bool {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.ends_with(".min.js") || name.ends_with(".generated.ts"))
        .unwrap_or(false)
    {
        return false;
    }

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_ascii_lowercase();
            SOURCE_EXTENSIONS
                .iter()
                .any(|source_ext| lower == *source_ext)
        })
        .unwrap_or(false)
}

fn count_lines(path: &Path) -> Option<usize> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);
    let mut lines = 0usize;
    for line in reader.lines() {
        if line.is_err() {
            return None;
        }
        lines += 1;
    }
    Some(lines)
}

fn source_role(relative_path: &str) -> &'static str {
    let name = relative_path.rsplit('/').next().unwrap_or(relative_path);
    if matches!(
        name,
        "main.rs" | "router.rs" | "App.tsx" | "App.jsx" | "MainActivity.kt"
    ) {
        return "入口/组装文件";
    }
    if name.ends_with("Test.kt")
        || name.ends_with("_test.rs")
        || name.ends_with(".test.ts")
        || name.ends_with(".spec.ts")
        || relative_path.contains("/test/")
        || relative_path.contains("/tests/")
    {
        return "测试文件";
    }
    if name.contains("schema") || name.contains("types") || name.ends_with(".d.ts") {
        return "协议/类型文件";
    }
    "业务源码文件"
}

fn size_band(lines: usize) -> &'static str {
    match lines {
        0..=500 => "绿区",
        501..=800 => "黄区可容忍",
        _ => "红区需拆分",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn reports_large_source_files_compactly() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "elon_source_hygiene_{}_{}",
            std::process::id(),
            nonce
        ));
        let src = dir.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("large.rs"), "fn demo() {}\n".repeat(801)).unwrap();
        fs::write(src.join("small.rs"), "fn demo() {}\n".repeat(12)).unwrap();

        let note = source_size_preflight_note(&dir).unwrap();

        assert!(note.contains("src/large.rs"));
        assert!(note.contains("801 行"));
        assert!(note.contains("红区需拆分"));
        assert!(!note.contains("src/small.rs"));

        fs::remove_dir_all(dir).unwrap();
    }
}
