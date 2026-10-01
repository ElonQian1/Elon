//! Character budget for history text, independent of UI pagination and provider token accounting.
use crate::store::SocialAiHistoryMessage;

const HISTORY_CHARS: usize = 32_000;
const NOTICE: &str = "[较早上下文已省略；以下不是完整聊天记录]\n";

pub(super) fn format(history: &[SocialAiHistoryMessage]) -> String {
    let mut remaining = HISTORY_CHARS - NOTICE.chars().count();
    let mut lines = vec![];
    let mut omitted = false;
    for message in history
        .iter()
        .rev()
        .filter(|m| !m.content.trim().is_empty())
    {
        let line = format!("{}：{}", message.speaker, message.content.trim());
        let size = line.chars().count() + 1;
        if size > remaining {
            omitted = true;
            if lines.is_empty() {
                // Keep the newest request prefix; mark the exceptional oversized message explicitly.
                let marker = "[本条过长，尾部已省略]";
                let take = remaining.saturating_sub(marker.chars().count() + 1);
                lines.push(format!(
                    "{}{}",
                    line.chars().take(take).collect::<String>(),
                    marker
                ));
            }
            break;
        }
        remaining -= size;
        lines.push(line);
    }
    lines.reverse();
    format!("{}{}", if omitted { NOTICE } else { "" }, lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn message_timeline_context_budget_retains_complete_recent_messages() {
        let history: Vec<_> = (0..50)
            .map(|i| SocialAiHistoryMessage {
                speaker: format!("sender{i}"),
                content: "字🙂".repeat(2000),
                from_request_user: true,
            })
            .collect();
        let result = format(&history);
        assert!(result.chars().count() <= HISTORY_CHARS);
        assert!(result.contains("sender49："));
        assert!(!result.contains("sender0："));
        assert!(result.ends_with(&"字🙂".repeat(2000)));
        assert_eq!(history.len(), 50, "budget never changes stored history");
    }
    #[test]
    fn message_timeline_context_budget_marks_oversize_without_splitting_unicode() {
        let history = [SocialAiHistoryMessage {
            speaker: "我".into(),
            content: "🙂".repeat(50000),
            from_request_user: true,
        }];
        let result = format(&history);
        assert!(result.chars().count() <= HISTORY_CHARS);
        assert!(result.ends_with("[本条过长，尾部已省略]"));
        assert_eq!(format(&[]), "");
    }
}
