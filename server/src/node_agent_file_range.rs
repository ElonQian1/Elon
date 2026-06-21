// server/src/node_agent_file_range.rs

use anyhow::{bail, Context, Result};
use std::path::Path;

const MAX_RANGE_LINES: usize = 400;
const MAX_RANGE_CHARS: usize = 24_000;

pub(crate) async fn read_file_range(
    full_path: &Path,
    display_path: &str,
    start_line: usize,
    line_count: usize,
) -> Result<String> {
    let text = tokio::fs::read_to_string(full_path)
        .await
        .with_context(|| format!("read_file_range failed: {display_path}"))?;
    render_file_range(display_path, &text, start_line, line_count)
}

fn render_file_range(
    display_path: &str,
    text: &str,
    start_line: usize,
    line_count: usize,
) -> Result<String> {
    if start_line == 0 {
        bail!("start_line must be >= 1");
    }
    if line_count == 0 {
        bail!("line_count must be >= 1");
    }

    let capped_line_count = line_count.min(MAX_RANGE_LINES);
    let total_lines = text.lines().count();
    let mut selected = Vec::new();
    for (index, line) in text.lines().enumerate().skip(start_line - 1) {
        if selected.len() >= capped_line_count {
            break;
        }
        selected.push((index + 1, line));
    }

    let end_line = selected
        .last()
        .map(|(line_number, _)| *line_number)
        .unwrap_or(start_line);
    let mut out = format!(
        "read_file_range ok: {} lines {}-{} of {}",
        display_path.trim(),
        start_line,
        end_line,
        total_lines
    );
    if line_count > capped_line_count {
        out.push_str(&format!(" (line_count capped at {MAX_RANGE_LINES})"));
    }
    if selected.is_empty() {
        out.push_str("\n[no lines in range]");
    } else {
        for (line_number, line) in selected {
            out.push('\n');
            out.push_str(&format!("{line_number}: {line}"));
        }
    }

    Ok(truncate_chars(&out, MAX_RANGE_CHARS))
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut truncated: String = value.chars().take(max_chars).collect();
    truncated.push_str("\n[truncated]");
    truncated
}

#[cfg(test)]
mod tests {
    use super::render_file_range;

    #[test]
    fn render_file_range_returns_numbered_slice() {
        let rendered = render_file_range("src/main.rs", "alpha\nbeta\ngamma\ndelta\n", 2, 2)
            .expect("range should render");

        assert!(rendered.contains("lines 2-3 of 4"));
        assert!(rendered.contains("2: beta"));
        assert!(rendered.contains("3: gamma"));
        assert!(!rendered.contains("1: alpha"));
    }

    #[test]
    fn render_file_range_reports_empty_out_of_range_slice() {
        let rendered =
            render_file_range("src/main.rs", "alpha\nbeta\n", 10, 5).expect("range should render");

        assert!(rendered.contains("lines 10-10 of 2"));
        assert!(rendered.contains("[no lines in range]"));
    }

    #[test]
    fn render_file_range_rejects_zero_values() {
        assert!(render_file_range("src/main.rs", "alpha\n", 0, 1).is_err());
        assert!(render_file_range("src/main.rs", "alpha\n", 1, 0).is_err());
    }
}
