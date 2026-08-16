use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

const HISTORY_LIMIT: usize = 160;

pub(super) fn merge(
    previous: Option<&Value>,
    mut incoming: Value,
    same_conversation: bool,
) -> Value {
    if !same_conversation {
        return incoming;
    }
    let Some(previous) = previous else {
        return incoming;
    };
    let previous_messages = messages(previous);
    let incoming_messages = messages(&incoming);
    if previous_messages.is_empty() {
        return incoming;
    }

    let previous_start = window_start(previous);
    let incoming_start = window_start(&incoming);
    let previous_observed = observed_count(previous, previous_start, previous_messages.len());
    let incoming_observed = observed_count(&incoming, incoming_start, incoming_messages.len());
    if incoming_start == 0
        && incoming_observed >= previous_observed
        && incoming_observed as usize <= incoming_messages.len()
    {
        return incoming;
    }

    let (merged, merged_start) = if incoming_messages.is_empty() {
        (previous_messages, previous_start)
    } else if messages_have_stable_ids(&previous_messages)
        && messages_have_stable_ids(&incoming_messages)
    {
        merge_by_stable_id(
            previous_messages,
            incoming_messages,
            previous_start,
            incoming_start,
        )
    } else if incoming_start > 0 {
        merge_by_position(
            previous_messages,
            incoming_messages,
            previous_start,
            incoming_start,
        )
    } else {
        // Position-only IDs from a short DOM window cannot prove whether a turn is old or new.
        // Keep the known conversation and still adopt the incoming live page metadata.
        (previous_messages, previous_start)
    };

    replace_messages(
        &mut incoming,
        merged,
        merged_start,
        previous_observed.max(incoming_observed),
    );
    incoming
}

fn merge_by_stable_id(
    previous: Vec<Value>,
    incoming: Vec<Value>,
    previous_start: usize,
    incoming_start: usize,
) -> (Vec<Value>, usize) {
    let previous_ids = previous
        .iter()
        .filter_map(stable_id)
        .collect::<HashSet<_>>();
    let incoming_ids = incoming
        .iter()
        .filter_map(stable_id)
        .collect::<HashSet<_>>();
    let overlaps = incoming_ids.iter().any(|id| previous_ids.contains(id));

    if !overlaps {
        if incoming_start > 0 && incoming_start.saturating_add(incoming.len()) <= previous_start {
            let mut merged = incoming;
            merged.extend(previous);
            return (merged, incoming_start);
        }
        let mut merged = previous;
        merged.extend(incoming);
        return (merged, previous_start);
    }

    let mut merged = previous;
    let mut cursor = None;
    for (incoming_index, message) in incoming.iter().enumerate() {
        let id = stable_id(message).expect("stable IDs were validated");
        if let Some(index) = position_of(&merged, id) {
            merged[index] = message.clone();
            cursor = Some(index + 1);
            continue;
        }
        let next_anchor = incoming[incoming_index + 1..]
            .iter()
            .filter_map(stable_id)
            .find_map(|next_id| position_of(&merged, next_id));
        let insertion = match (cursor, next_anchor) {
            (Some(after), Some(before)) if after <= before => after,
            (None, Some(before)) => before,
            _ => merged.len(),
        };
        merged.insert(insertion, message.clone());
        cursor = Some(insertion + 1);
    }
    let merged_start = if incoming_start > 0 {
        previous_start.min(incoming_start)
    } else {
        previous_start
    };
    (merged, merged_start)
}

fn merge_by_position(
    previous: Vec<Value>,
    incoming: Vec<Value>,
    previous_start: usize,
    incoming_start: usize,
) -> (Vec<Value>, usize) {
    let mut positioned = BTreeMap::new();
    for (offset, message) in previous.into_iter().enumerate() {
        positioned.insert(previous_start + offset, message);
    }
    for (offset, message) in incoming.into_iter().enumerate() {
        positioned.insert(incoming_start + offset, message);
    }
    let start = positioned
        .first_key_value()
        .map(|(index, _)| *index)
        .unwrap_or(incoming_start);
    (positioned.into_values().collect(), start)
}

fn replace_messages(
    incoming: &mut Value,
    messages: Vec<Value>,
    window_start: usize,
    observed: u64,
) {
    let dropped = messages.len().saturating_sub(HISTORY_LIMIT);
    let bounded = messages.into_iter().skip(dropped).collect::<Vec<_>>();
    let bounded_start = window_start.saturating_add(dropped);
    let bounded_observed = observed.max((bounded_start + bounded.len()) as u64);
    if let Some(snapshot) = incoming.as_object_mut() {
        snapshot.insert("messages".to_string(), Value::Array(bounded));
        snapshot.insert(
            "messageWindowStart".to_string(),
            Value::from(bounded_start as u64),
        );
        snapshot.insert(
            "observedMessageCount".to_string(),
            Value::from(bounded_observed),
        );
    }
}

fn messages(snapshot: &Value) -> Vec<Value> {
    snapshot
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn window_start(snapshot: &Value) -> usize {
    snapshot
        .get("messageWindowStart")
        .and_then(Value::as_u64)
        .unwrap_or_default() as usize
}

fn observed_count(snapshot: &Value, start: usize, message_count: usize) -> u64 {
    snapshot
        .get("observedMessageCount")
        .and_then(Value::as_u64)
        .unwrap_or((start + message_count) as u64)
        .max((start + message_count) as u64)
}

fn messages_have_stable_ids(messages: &[Value]) -> bool {
    !messages.is_empty() && messages.iter().all(|message| stable_id(message).is_some())
}

fn stable_id(message: &Value) -> Option<&str> {
    let id = message.get("id").and_then(Value::as_str)?.trim();
    if id.is_empty() || is_position_only_id(id) {
        None
    } else {
        Some(id)
    }
}

fn is_position_only_id(id: &str) -> bool {
    ["user-", "assistant-", "message-", "conversation-turn-"]
        .iter()
        .any(|prefix| {
            id.strip_prefix(prefix).is_some_and(|suffix| {
                !suffix.is_empty()
                    && suffix
                        .split('-')
                        .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
            })
        })
}

fn position_of(messages: &[Value], expected: &str) -> Option<usize> {
    messages
        .iter()
        .position(|message| stable_id(message) == Some(expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stable_short_window_appends_new_turns_when_dom_start_resets() {
        let previous = snapshot(
            0,
            4,
            vec![message("u1"), message("a1"), message("u2"), message("a2")],
        );
        let incoming = snapshot(0, 2, vec![message("u3"), message("a3")]);

        let merged = merge(Some(&previous), incoming, true);

        assert_eq!(ids(&merged), vec!["u1", "a1", "u2", "a2", "u3", "a3"]);
        assert_eq!(merged["observedMessageCount"], 6);
    }

    #[test]
    fn stable_old_subset_updates_metadata_without_erasing_known_turns() {
        let mut previous = snapshot(
            0,
            4,
            vec![message("u1"), message("a1"), message("u2"), message("a2")],
        );
        previous["messages"][1]["state"] = Value::String("streaming".to_string());
        let mut incoming = snapshot(0, 2, vec![message("u1"), message("a1")]);
        incoming["currentModel"] = Value::String("GPT current".to_string());

        let merged = merge(Some(&previous), incoming, true);

        assert_eq!(ids(&merged), vec!["u1", "a1", "u2", "a2"]);
        assert_eq!(merged["messages"][1]["state"], "completed");
        assert_eq!(merged["currentModel"], "GPT current");
        assert_eq!(merged["observedMessageCount"], 4);
    }

    #[test]
    fn position_only_short_window_is_not_treated_as_stable_identity() {
        let previous = snapshot(
            0,
            4,
            vec![message("u1"), message("a1"), message("u2"), message("a2")],
        );
        let incoming = snapshot(0, 2, vec![message("user-0"), message("assistant-1")]);

        let merged = merge(Some(&previous), incoming, true);

        assert_eq!(ids(&merged), vec!["u1", "a1", "u2", "a2"]);
    }

    #[test]
    fn cached_position_only_ids_do_not_duplicate_a_later_stable_window() {
        let previous = snapshot(0, 4, vec![message("user-0"), message("assistant-1")]);
        let incoming = snapshot(0, 2, vec![message("u2"), message("a2")]);

        let merged = merge(Some(&previous), incoming, true);

        assert_eq!(ids(&merged), vec!["user-0", "assistant-1"]);
    }

    #[test]
    fn stable_history_keeps_the_latest_one_hundred_sixty_messages() {
        let previous_messages = (0..160)
            .map(|index| message(&format!("old-{index}")))
            .collect();
        let incoming_messages = (0..4)
            .map(|index| message(&format!("new-{index}")))
            .collect();
        let previous = snapshot(0, 160, previous_messages);
        let incoming = snapshot(0, 4, incoming_messages);

        let merged = merge(Some(&previous), incoming, true);

        assert_eq!(merged["messages"].as_array().unwrap().len(), HISTORY_LIMIT);
        assert_eq!(merged["messageWindowStart"], 4);
        assert_eq!(merged["observedMessageCount"], 164);
        assert_eq!(merged["messages"][159]["id"], "new-3");
    }

    fn snapshot(start: usize, observed: usize, messages: Vec<Value>) -> Value {
        json!({
            "type": "message_snapshot",
            "messageWindowStart": start,
            "observedMessageCount": observed,
            "messages": messages
        })
    }

    fn message(id: &str) -> Value {
        json!({"id": id, "role": "assistant", "state": "completed", "content": []})
    }

    fn ids(snapshot: &Value) -> Vec<&str> {
        snapshot["messages"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|message| message["id"].as_str())
            .collect()
    }
}
