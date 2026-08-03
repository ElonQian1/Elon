//! Conservative launch-time intent gate for the lightweight context profile.

const PROJECT_CONTEXT_SKIP_MARKER: &str = "<elon-project-context-skip version=\"1\">";

pub(super) fn skip_context_profile(prompt: &str) -> bool {
    if prompt.contains(PROJECT_CONTEXT_SKIP_MARKER) {
        return true;
    }
    let task = user_task_text(prompt);
    let normalized = task.to_lowercase();
    let broad = [
        "架构",
        "跨文件",
        "跨模块",
        "整个项目",
        "项目现状",
        "当前状态",
        "陌生项目",
        "多文件",
        "文档治理",
        "architecture",
        "cross-file",
        "cross module",
        "project status",
        "multiple files",
    ]
    .iter()
    .any(|marker| normalized.contains(marker));
    if broad {
        return false;
    }
    let explicit_single = [
        "只修改",
        "只改",
        "仅修改",
        "仅修复",
        "这个文件",
        "此文件",
        "single file",
        "only edit",
        "only change",
    ]
    .iter()
    .any(|marker| normalized.contains(marker));
    let paths = task
        .split_whitespace()
        .filter(|token| looks_like_source_path(token))
        .take(2)
        .collect::<Vec<_>>();
    let line_anchored = paths.first().is_some_and(|path| {
        path.rsplit_once(':')
            .is_some_and(|(_, line)| line.parse::<u32>().is_ok())
    });
    paths.len() == 1 && (explicit_single || line_anchored)
}

fn user_task_text(prompt: &str) -> &str {
    let Some((_, tail)) = prompt.split_once("<user-request>") else {
        return prompt;
    };
    tail.split_once("</user-request>")
        .map(|(task, _)| task)
        .unwrap_or(tail)
}

fn looks_like_source_path(token: &str) -> bool {
    let candidate = token.trim_matches(|ch: char| {
        matches!(
            ch,
            '`' | '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | '，'
        )
    });
    let path = candidate
        .rsplit_once(':')
        .filter(|(_, line)| line.parse::<u32>().is_ok())
        .map(|(path, _)| path)
        .unwrap_or(candidate)
        .to_ascii_lowercase();
    (path.contains('/') || path.contains('\\'))
        && [
            ".rs", ".kt", ".java", ".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".cs", ".cpp", ".c",
            ".h", ".html", ".css",
        ]
        .iter()
        .any(|extension| path.ends_with(extension))
}

#[cfg(test)]
mod tests {
    use super::skip_context_profile;

    #[test]
    fn skips_only_explicit_precise_tasks() {
        assert!(skip_context_profile("只修改 server/src/main.rs:42"));
        assert!(!skip_context_profile(
            "分析 server/src/main.rs 涉及的整体架构"
        ));
        assert!(!skip_context_profile("修复普通代码任务"));
    }

    #[test]
    fn extracts_user_request_from_executor_envelope() {
        let prompt = "<elon-pc-executor>AGENTS.md</elon-pc-executor>\n<user-request>只改 src/app.ts:9</user-request>";
        assert!(skip_context_profile(prompt));
    }
}
