use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

use super::chatgpt_rich_preservation::preserve_message_rich_content;

pub(super) fn preserve_private_stream_turn_content(previous: &[Value], incoming: &mut [Value]) {
    let previous_turns = assistant_turns(previous);
    let incoming_turns = assistant_turns(incoming);
    let previous_skip = previous_turns.len().saturating_sub(incoming_turns.len());
    let incoming_skip = incoming_turns.len().saturating_sub(previous_turns.len());
    for (turn_index, previous_assistants) in previous_turns.iter().enumerate().skip(previous_skip) {
        let incoming_turn_index = incoming_skip + turn_index - previous_skip;
        let Some(incoming_assistants) = incoming_turns.get(incoming_turn_index) else {
            continue;
        };
        let Some(&target_index) = incoming_assistants.last() else {
            continue;
        };
        for &previous_index in previous_assistants {
            if !is_private_stream_message(&previous[previous_index]) {
                continue;
            }
            incoming[target_index] = preserve_message_rich_content(
                &previous[previous_index],
                incoming[target_index].clone(),
            );
        }
    }
}

pub(super) fn reconcile_private_stream_shadows(messages: Vec<Value>) -> Vec<Value> {
    let turns = assistant_turns(&messages);
    if turns.is_empty() {
        return messages;
    }
    let mut replacements = BTreeMap::new();
    let mut removed = HashSet::new();
    for assistants in turns {
        let Some(&target_index) = assistants
            .iter()
            .rev()
            .find(|&&index| !is_private_stream_message(&messages[index]))
        else {
            continue;
        };
        let private_indices = assistants
            .iter()
            .copied()
            .filter(|&index| is_private_stream_message(&messages[index]))
            .collect::<Vec<_>>();
        if private_indices.is_empty() {
            continue;
        }
        let mut target = messages[target_index].clone();
        for private_index in private_indices {
            target = preserve_message_rich_content(&messages[private_index], target);
            removed.insert(private_index);
        }
        replacements.insert(target_index, target);
    }
    messages
        .into_iter()
        .enumerate()
        .filter_map(|(index, message)| {
            if removed.contains(&index) {
                None
            } else {
                Some(replacements.remove(&index).unwrap_or(message))
            }
        })
        .collect()
}

fn assistant_turns(messages: &[Value]) -> Vec<Vec<usize>> {
    let mut turns = Vec::new();
    let mut current = None;
    for (index, message) in messages.iter().enumerate() {
        match message.get("role").and_then(Value::as_str) {
            Some("user") => {
                turns.push(Vec::new());
                current = Some(turns.len() - 1);
            }
            Some("assistant") => {
                if let Some(turn_index) = current {
                    turns[turn_index].push(index);
                }
            }
            _ => {}
        }
    }
    turns
}

fn is_private_stream_message(message: &Value) -> bool {
    message
        .get("id")
        .and_then(Value::as_str)
        .is_some_and(|id| id.starts_with("private-stream:"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn removes_private_stream_shadow_after_official_message_arrives() {
        let messages = vec![
            user_message("u1", "比特币走势图"),
            finance_message("private-stream:reply-1", "US$78,805.00"),
            assistant_message("a1", "官方正文"),
        ];

        let reconciled = reconcile_private_stream_shadows(messages);

        assert_eq!(ids(&reconciled), vec!["u1", "a1"]);
        assert_eq!(reconciled[1]["content"][1]["type"], "rich_card");
    }

    #[test]
    fn private_cards_stay_with_their_original_repeated_prompt_turns() {
        let previous = vec![
            user_message("u1", "再说一次"),
            finance_message("private-stream:reply-1", "US$77,000.00"),
            user_message("u2", "再说一次"),
            finance_message("private-stream:reply-2", "US$78,805.00"),
        ];
        let mut incoming = vec![
            user_message("conversation-turn-0", "再说一次"),
            assistant_message("conversation-turn-1", "第一轮正文"),
            user_message("conversation-turn-2", "再说一次"),
            assistant_message("conversation-turn-3", "第二轮正文"),
        ];

        preserve_private_stream_turn_content(&previous, &mut incoming);

        assert_eq!(
            incoming[1]["content"][1]["richContent"]["payload"]["primaryValue"],
            "US$77,000.00"
        );
        assert_eq!(
            incoming[3]["content"][1]["richContent"]["payload"]["primaryValue"],
            "US$78,805.00"
        );
    }

    #[test]
    fn tail_window_private_card_is_not_attached_to_an_older_full_history_turn() {
        let previous = vec![
            user_message("u3", "最新问题"),
            finance_message("private-stream:reply-3", "US$78,805.00"),
        ];
        let mut incoming = vec![
            user_message("conversation-turn-0", "旧问题一"),
            assistant_message("conversation-turn-1", "旧回答一"),
            user_message("conversation-turn-2", "旧问题二"),
            assistant_message("conversation-turn-3", "旧回答二"),
            user_message("conversation-turn-4", "最新问题"),
            assistant_message("conversation-turn-5", "最新回答"),
        ];

        preserve_private_stream_turn_content(&previous, &mut incoming);

        assert_eq!(incoming[1]["content"].as_array().unwrap().len(), 1);
        assert_eq!(incoming[3]["content"].as_array().unwrap().len(), 1);
        assert_eq!(
            incoming[5]["content"][1]["richContent"]["payload"]["primaryValue"],
            "US$78,805.00"
        );
    }

    #[test]
    fn private_stream_answer_without_official_successor_is_retained() {
        let messages = vec![
            user_message("u1", "比特币走势图"),
            finance_message("private-stream:reply-1", "US$78,805.00"),
            user_message("u2", "另一个问题"),
            assistant_message("a2", "另一轮正文"),
        ];

        let reconciled = reconcile_private_stream_shadows(messages);

        assert_eq!(ids(&reconciled), vec!["u1", "private-stream:reply-1", "u2", "a2"]);
    }

    fn user_message(id: &str, text: &str) -> Value {
        json!({
            "id": id,
            "role": "user",
            "state": "completed",
            "content": [{"type":"text","text":text}]
        })
    }

    fn assistant_message(id: &str, text: &str) -> Value {
        json!({
            "id": id,
            "role": "assistant",
            "state": "completed",
            "content": [{"type":"markdown","text":text}]
        })
    }

    fn finance_message(id: &str, price: &str) -> Value {
        json!({
            "id": id,
            "role": "assistant",
            "state": "completed",
            "content": [{
                "type":"rich_card",
                "text":"Bitcoin (BTC)",
                "kind":"finance",
                "richContent":{
                    "schema":"yilong.rich-content.v1",
                    "kind":"finance",
                    "source":"private_response",
                    "payload":{
                        "title":"Bitcoin (BTC)",
                        "primaryValue":price,
                        "trend":"positive",
                        "periods":[],
                        "metrics":[]
                    }
                }
            }]
        })
    }

    fn ids(messages: &[Value]) -> Vec<&str> {
        messages
            .iter()
            .filter_map(|message| message["id"].as_str())
            .collect()
    }
}
