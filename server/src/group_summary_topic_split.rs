//! Topic splitting for automatic group summary post creation.

use anyhow::Context;
use chrono::{DateTime, Duration, Timelike, Utc};
use serde::Serialize;
use serde_json::json;
use std::{collections::BTreeSet, sync::Arc};

use crate::{
    agent_llm_call::call_chat_llm_with_options,
    social_ai::resolve_social_agent,
    store::{GroupAiDocument, GroupSummarySourceMessage},
    types::AppState,
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct GroupSummaryTopicCandidate {
    pub title: String,
    pub topic: String,
    pub message_ids: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct GroupSummaryTopicSplit {
    pub mode: String,
    pub requested_message_count: usize,
    pub topics: Vec<GroupSummaryTopicCandidate>,
}

pub(crate) async fn split_group_summary_topics(
    state: &Arc<AppState>,
    user_id: &str,
    messages: &[GroupSummarySourceMessage],
    documents: &[GroupAiDocument],
    max_topics: usize,
) -> GroupSummaryTopicSplit {
    let max_topics = max_topics.clamp(1, 6);
    match ai_split_topics(state, user_id, messages, documents, max_topics).await {
        Ok(topics) if !topics.is_empty() => GroupSummaryTopicSplit {
            mode: "ai".to_string(),
            requested_message_count: messages.len(),
            topics,
        },
        _ => GroupSummaryTopicSplit {
            mode: "time_gap_fallback".to_string(),
            requested_message_count: messages.len(),
            topics: fallback_topic_candidates(messages, max_topics),
        },
    }
}

pub(crate) fn fallback_topic_candidates(
    messages: &[GroupSummarySourceMessage],
    max_topics: usize,
) -> Vec<GroupSummaryTopicCandidate> {
    if messages.is_empty() {
        return Vec::new();
    }
    let max_topics = max_topics.clamp(1, 6);
    let mut chunks: Vec<Vec<&GroupSummarySourceMessage>> = Vec::new();
    let mut current = Vec::new();
    let mut previous_at: Option<DateTime<Utc>> = None;
    for message in messages {
        let at = parse_time(&message.created_at);
        let should_split = chunks.len() + 1 < max_topics
            && current.len() >= 2
            && time_gap_minutes(previous_at, at).unwrap_or(0) >= 90;
        if should_split {
            chunks.push(std::mem::take(&mut current));
        }
        current.push(message);
        previous_at = at;
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    if chunks.len() == 1 && messages.len() > 80 && max_topics > 1 {
        return size_based_chunks(messages, max_topics);
    }
    chunks
        .into_iter()
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| candidate_from_chunk(&chunk, "按聊天时间间隔自动拆分"))
        .collect()
}

async fn ai_split_topics(
    state: &Arc<AppState>,
    user_id: &str,
    messages: &[GroupSummarySourceMessage],
    documents: &[GroupAiDocument],
    max_topics: usize,
) -> anyhow::Result<Vec<GroupSummaryTopicCandidate>> {
    let agent = resolve_social_agent(state).await?;
    let response = call_chat_llm_with_options(
        state,
        &agent,
        &[
            json!({
                "role": "system",
                "content": "你是群聊议题拆分 AI。你只能根据给定消息和群聊 AI 文档拆分议题，不得编造消息 ID。只输出 JSON，不要输出 Markdown。"
            }),
            json!({
                "role": "user",
                "content": build_split_prompt(messages, documents, max_topics)
            }),
        ],
        user_id,
        "group_summary_topic_split",
        0.1,
        1400,
    )
    .await?;
    let content = response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim();
    let raw_json = extract_json_object(content).context("AI 没有返回 JSON")?;
    let value: serde_json::Value = serde_json::from_str(raw_json).context("议题 JSON 无法解析")?;
    Ok(validate_ai_topics(&value, messages, max_topics))
}

fn build_split_prompt(
    messages: &[GroupSummarySourceMessage],
    documents: &[GroupAiDocument],
    max_topics: usize,
) -> String {
    let docs = documents
        .iter()
        .filter(|doc| {
            doc.path.contains("TOPIC")
                || doc.path.contains("SUMMARY")
                || doc.path.contains("GROUP_CHAT")
                || doc.path.contains("RAG")
        })
        .map(|doc| {
            format!(
                "### {}\n{}",
                doc.path,
                truncate_chars(&doc.content, 1200).replace('\r', "")
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let message_lines = messages
        .iter()
        .map(|message| {
            format!(
                "- id={} time={} sender={} content={}",
                message.id,
                message.created_at,
                message.sender_name,
                truncate_chars(&message.content.replace('\n', " "), 260)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"请把下面群聊消息按真实议题拆分成 1 到 {max_topics} 个总结帖候选。

拆分规则：
- 同一个议题连续讨论时不要拆。
- 上午讨论一件事、下午讨论另一件事，或目标/结论明显不同，应拆成不同 topic。
- 每个 topic 至少包含 2 条消息；如果无法可靠拆分，就只返回 1 个 topic。
- message_ids 只能使用下方出现的 id，按时间顺序排列。
- title 使用 8 到 24 个中文字符，适合做群聊置顶总结帖标题。

输出 JSON schema：
{{"topics":[{{"title":"标题","topic":"议题说明","message_ids":["消息ID"],"reason":"为什么这样拆"}}]}}

<group_ai_docs>
{docs}
</group_ai_docs>

<messages>
{message_lines}
</messages>"#
    )
}

fn validate_ai_topics(
    value: &serde_json::Value,
    messages: &[GroupSummarySourceMessage],
    max_topics: usize,
) -> Vec<GroupSummaryTopicCandidate> {
    let valid_ids = messages
        .iter()
        .map(|message| message.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut used = BTreeSet::<String>::new();
    value["topics"]
        .as_array()
        .into_iter()
        .flatten()
        .take(max_topics)
        .filter_map(|topic| {
            let mut ids = Vec::<String>::new();
            for id in topic["message_ids"].as_array().into_iter().flatten() {
                let id = id.as_str()?.trim();
                if valid_ids.contains(id) && used.insert(id.to_string()) {
                    ids.push(id.to_string());
                }
            }
            if ids.len() < 2 {
                return None;
            }
            let title = clean_label(topic["title"].as_str(), &ids, messages);
            let topic_text = topic["topic"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(&title)
                .chars()
                .take(160)
                .collect::<String>();
            Some(GroupSummaryTopicCandidate {
                title,
                topic: topic_text,
                message_ids: ids,
                reason: topic["reason"]
                    .as_str()
                    .unwrap_or("AI 根据语义和时间线拆分")
                    .chars()
                    .take(180)
                    .collect(),
            })
        })
        .collect()
}

fn size_based_chunks(
    messages: &[GroupSummarySourceMessage],
    max_topics: usize,
) -> Vec<GroupSummaryTopicCandidate> {
    let chunk_size = ((messages.len() + max_topics - 1) / max_topics).max(2);
    messages
        .chunks(chunk_size)
        .map(|chunk| {
            let chunk = chunk.iter().collect::<Vec<_>>();
            candidate_from_chunk(&chunk, "消息较多，按时间顺序分段")
        })
        .collect()
}

fn candidate_from_chunk(
    chunk: &[&GroupSummarySourceMessage],
    reason: &str,
) -> GroupSummaryTopicCandidate {
    let ids = chunk
        .iter()
        .map(|message| message.id.clone())
        .collect::<Vec<_>>();
    let owned_messages = chunk
        .iter()
        .map(|message| (*message).clone())
        .collect::<Vec<_>>();
    let title = clean_label(None, &ids, &owned_messages);
    let start = chunk
        .first()
        .and_then(|message| parse_time(&message.created_at));
    GroupSummaryTopicCandidate {
        title: time_prefixed_title(start, title),
        topic: chunk
            .first()
            .map(|message| truncate_chars(&message.content.replace('\n', " "), 120))
            .unwrap_or_else(|| "群聊议题".to_string()),
        message_ids: ids,
        reason: reason.to_string(),
    }
}

fn clean_label(
    label: Option<&str>,
    ids: &[String],
    messages: &[GroupSummarySourceMessage],
) -> String {
    let fallback = ids
        .first()
        .and_then(|id| messages.iter().find(|message| &message.id == id))
        .map(|message| truncate_chars(&message.content.replace('\n', " "), 18))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "群聊总结".to_string());
    let value = label
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&fallback);
    let title = value
        .chars()
        .filter(|ch| !matches!(ch, '"' | '\'' | '`' | '\n' | '\r'))
        .take(24)
        .collect::<String>();
    if title.is_empty() {
        "群聊总结".to_string()
    } else {
        title
    }
}

fn time_prefixed_title(at: Option<DateTime<Utc>>, title: String) -> String {
    let prefix = match at.map(|value| (value + Duration::hours(8)).hour()) {
        Some(0..=11) => "上午讨论",
        Some(12..=17) => "下午讨论",
        Some(_) => "晚上讨论",
        None => "群聊讨论",
    };
    if title.starts_with(prefix) || title == "群聊总结" {
        format!("{}：{}", prefix, title.trim_start_matches("群聊总结"))
            .trim_end_matches('：')
            .to_string()
    } else {
        format!("{}：{}", prefix, title)
    }
}

fn parse_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn time_gap_minutes(
    previous: Option<DateTime<Utc>>,
    current: Option<DateTime<Utc>>,
) -> Option<i64> {
    Some((current? - previous?).num_minutes())
}

fn extract_json_object(content: &str) -> Option<&str> {
    let trimmed = content.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed);
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    (end > start).then_some(&trimmed[start..=end])
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_splits_morning_and_afternoon_discussions() {
        let messages = vec![
            msg("m1", "2026-06-17T01:00:00Z", "上午第一件事"),
            msg("m2", "2026-06-17T01:10:00Z", "继续上午议题"),
            msg("m3", "2026-06-17T06:00:00Z", "下午另一件事"),
            msg("m4", "2026-06-17T06:05:00Z", "继续下午议题"),
        ];
        let topics = fallback_topic_candidates(&messages, 4);
        assert_eq!(topics.len(), 2);
        assert_eq!(topics[0].message_ids, vec!["m1", "m2"]);
        assert_eq!(topics[1].message_ids, vec!["m3", "m4"]);
        assert!(topics[0].title.starts_with("上午讨论"));
        assert!(topics[1].title.starts_with("下午讨论"));
    }

    #[test]
    fn fallback_keeps_short_single_topic_together() {
        let messages = vec![
            msg("m1", "2026-06-17T01:00:00Z", "第一句"),
            msg("m2", "2026-06-17T01:10:00Z", "第二句"),
            msg("m3", "2026-06-17T01:20:00Z", "第三句"),
        ];
        let topics = fallback_topic_candidates(&messages, 4);
        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0].message_ids, vec!["m1", "m2", "m3"]);
    }

    fn msg(id: &str, created_at: &str, content: &str) -> GroupSummarySourceMessage {
        GroupSummarySourceMessage {
            id: id.to_string(),
            group_id: "g1".to_string(),
            sender_user_id: "u1".to_string(),
            sender_name: "成员".to_string(),
            content: content.to_string(),
            created_at: created_at.to_string(),
        }
    }
}
