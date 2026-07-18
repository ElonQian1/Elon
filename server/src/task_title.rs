//! Deterministic, no-model titles for PC-local tasks and their conversations.

pub(crate) const LOCAL_TASK_PLACEHOLDER_TITLE: &str = "本机离线任务";
pub(crate) const LOCAL_TASK_FALLBACK_TITLE: &str = "本机 Codex 任务";

const MAX_TITLE_CHARS: usize = 34;
const ORIGINAL_REQUIREMENT_MARKER: &str = "用户原始需求";
const SECTION_STOPS: [&str; 4] = ["桌面监督分析结论", "用户可见目标", "实施要求", "非目标"];

pub(crate) fn readable_task_title(prompt: &str) -> String {
    let user_request = user_request_body(prompt);
    let original_requirement = original_requirement_body(user_request);
    let cleaned = clean_lines(original_requirement);
    let preferred = preferred_goal_clause(&cleaned).unwrap_or(cleaned.as_str());
    let candidate = normalize_candidate(preferred);
    if candidate.is_empty() || is_machine_line(&candidate) {
        return LOCAL_TASK_FALLBACK_TITLE.to_string();
    }
    truncate_title(&candidate)
}

pub(crate) fn local_task_conversation_title(
    stored_title: Option<&str>,
    prompt: Option<&str>,
) -> Option<String> {
    match stored_title {
        Some(LOCAL_TASK_PLACEHOLDER_TITLE) => Some(readable_task_title(prompt.unwrap_or_default())),
        Some(title) => Some(title.to_string()),
        None => None,
    }
}

fn user_request_body(prompt: &str) -> &str {
    if let Some(start) = prompt.find("<user-request>") {
        let body = &prompt[start + "<user-request>".len()..];
        return body
            .find("</user-request>")
            .map(|end| &body[..end])
            .unwrap_or(body);
    }
    if let Some(end) = prompt.find("</elon-pc-executor>") {
        return &prompt[end + "</elon-pc-executor>".len()..];
    }
    prompt
}

fn original_requirement_body(prompt: &str) -> &str {
    let Some(marker) = prompt.find(ORIGINAL_REQUIREMENT_MARKER) else {
        return prompt;
    };
    let body = prompt[marker + ORIGINAL_REQUIREMENT_MARKER.len()..]
        .trim_start_matches(|ch: char| ch.is_whitespace() || matches!(ch, ':' | '：'));
    let end = SECTION_STOPS
        .iter()
        .filter_map(|stop| body.find(stop))
        .min()
        .unwrap_or(body.len());
    &body[..end]
}

fn clean_lines(input: &str) -> String {
    input
        .lines()
        .map(remove_codex_uri)
        .map(|line| strip_title_edges(&line))
        .filter(|line| !line.is_empty() && !is_machine_line(line))
        .collect::<Vec<_>>()
        .join(" ")
}

fn remove_codex_uri(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("codex://") {
        output.push_str(&rest[..start]);
        let uri = &rest[start..];
        let end = uri
            .char_indices()
            .skip("codex://".chars().count())
            .find(|(_, ch)| {
                !(ch.is_ascii_alphanumeric()
                    || matches!(
                        ch,
                        ':' | '/' | '-' | '_' | '.' | '?' | '=' | '&' | '%' | '#'
                    ))
            })
            .map(|(index, _)| index)
            .unwrap_or(uri.len());
        rest = &uri[end..];
    }
    output.push_str(rest);
    output
}

fn strip_title_edges(input: &str) -> String {
    let mut value = input.trim().trim_matches(title_edge_char).trim();
    if let Some(stripped) = strip_list_prefix(value) {
        value = stripped;
    }
    value.trim_matches(title_edge_char).trim().to_string()
}

fn strip_list_prefix(input: &str) -> Option<&str> {
    let digit_bytes = input
        .char_indices()
        .take_while(|(_, ch)| ch.is_ascii_digit())
        .map(|(index, ch)| index + ch.len_utf8())
        .last()?;
    let rest = &input[digit_bytes..];
    let rest = rest
        .strip_prefix('.')
        .or_else(|| rest.strip_prefix('、'))
        .or_else(|| rest.strip_prefix(')'))?;
    Some(rest.trim_start())
}

fn title_edge_char(ch: char) -> bool {
    ch.is_whitespace()
        || matches!(
            ch,
            '“' | '”' | '‘' | '’' | '"' | '\'' | '`' | '#' | '*' | '-' | '•' | '<' | '>'
        )
}

fn is_machine_line(input: &str) -> bool {
    let value = input.trim();
    let lower = value.to_ascii_lowercase();
    lower.starts_with("<elon-pc-executor")
        || lower.starts_with("</elon-pc-executor")
        || lower.starts_with("<user-request")
        || lower.starts_with("</user-request")
        || lower.starts_with("supervision_contract=")
        || value.starts_with("你是由一龙 PC 本机节点启动的执行者")
        || value.starts_with("直接在当前项目完成任务")
        || value.starts_with("读取并遵守项目 AGENTS.md")
        || value.starts_with("桌面监督者会独立检查")
        || value.starts_with("非阻塞的平台改进先记录")
        || value.starts_with("最终回复分别说明")
        || value.starts_with("桌面监督分析结论")
        || value.starts_with("用户可见目标")
        || value.starts_with("实施要求")
        || value.starts_with("非目标")
        || value.starts_with("节点更新后自动恢复原任务")
        || value.starts_with("请恢复原任务")
        || value.starts_with("请继续完成上述任务")
        || value.starts_with("继续完成原任务并运行统一收尾")
}

fn preferred_goal_clause(input: &str) -> Option<&str> {
    for cue in ["用户希望的是", "用户希望的就是", "希望的是"] {
        if let Some(start) = input.find(cue) {
            let clause = input[start + cue.len()..].trim_start_matches(|ch: char| {
                ch.is_whitespace() || matches!(ch, ',' | '，' | ':' | '：')
            });
            return Some(first_sentence(clause));
        }
    }
    None
}

fn normalize_candidate(input: &str) -> String {
    let sentence = first_sentence(input);
    let collapsed = sentence.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut value = strip_title_edges(&collapsed);
    for prefix in ["请你", "请", "麻烦你", "麻烦", "帮我", "希望你", "需要你"] {
        if let Some(rest) = value.strip_prefix(prefix) {
            value = strip_title_edges(rest);
            break;
        }
    }
    if value.starts_with('有') && value.chars().count() > 2 {
        value = value['有'.len_utf8()..].to_string();
    }
    value
        .trim_matches(|ch: char| {
            ch.is_whitespace() || matches!(ch, ',' | '，' | ';' | '；' | ':' | '：')
        })
        .to_string()
}

fn first_sentence(input: &str) -> &str {
    input
        .find(|ch: char| matches!(ch, '。' | '！' | '？' | '!' | '?'))
        .map(|end| &input[..end])
        .unwrap_or(input)
}

fn truncate_title(input: &str) -> String {
    if input.chars().count() <= MAX_TITLE_CHARS {
        return input.to_string();
    }
    input
        .chars()
        .take(MAX_TITLE_CHARS - 1)
        .chain(['…'])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_readable_chinese_title_without_model_call() {
        assert_eq!(
            readable_task_title("  请修复登录页面按钮错位，并补充回归测试。  "),
            "修复登录页面按钮错位，并补充回归测试"
        );
    }

    #[test]
    fn filters_codex_uri_and_supervision_wrapper() {
        assert_eq!(
            readable_task_title("codex://threads/019-test\n请完善本机任务标题"),
            "完善本机任务标题"
        );
        let wrapped = r#"<elon-pc-executor version="1">
supervision_contract={"protocol":"elon.desktop_pc_supervision.v1"}
</elon-pc-executor>
<user-request>
用户原始需求：
“现在的标题不友好。用户希望的是，有适合人阅读且可区分的任务标题。”
桌面监督分析结论：不需要 Goal 模式。
</user-request>"#;
        assert_eq!(readable_task_title(wrapped), "适合人阅读且可区分的任务标题");
    }

    #[test]
    fn metadata_only_input_has_stable_fallback_and_long_titles_are_bounded() {
        assert_eq!(
            readable_task_title("codex://threads/019-test\n请继续完成上述任务并运行统一收尾"),
            LOCAL_TASK_FALLBACK_TITLE
        );
        let title = readable_task_title(&"修复".repeat(40));
        assert_eq!(title.chars().count(), MAX_TITLE_CHARS);
        assert!(title.ends_with('…'));
    }

    #[test]
    fn only_exact_placeholder_uses_prompt_fallback() {
        assert_eq!(
            local_task_conversation_title(Some(LOCAL_TASK_PLACEHOLDER_TITLE), Some("修复会话标题")),
            Some("修复会话标题".to_string())
        );
        assert_eq!(
            local_task_conversation_title(Some("用户手工标题"), Some("不应覆盖")),
            Some("用户手工标题".to_string())
        );
        assert_eq!(
            local_task_conversation_title(Some(" 本机离线任务 "), Some("不应覆盖")),
            Some(" 本机离线任务 ".to_string())
        );
    }
}
