use serde_json::Value;

pub(super) fn preserve_private_rich_content(previous: &[Value], incoming: &mut [Value]) {
    for message in incoming {
        let Some(id) = stable_id(message) else {
            continue;
        };
        let Some(known) = previous
            .iter()
            .find(|candidate| stable_id(candidate) == Some(id))
        else {
            continue;
        };
        *message = preserve_message_rich_content(known, message.clone());
    }
}

pub(super) fn preserve_message_rich_content(previous: &Value, mut incoming: Value) -> Value {
    if previous.get("role").and_then(Value::as_str) != Some("assistant")
        || incoming.get("role").and_then(Value::as_str) != Some("assistant")
    {
        return incoming;
    }
    let previous_rich = content(previous)
        .into_iter()
        .filter(valid_rich_card)
        .collect::<Vec<_>>();
    if previous_rich.is_empty() {
        return incoming;
    }
    let mut incoming_content = content(&incoming);
    let mut preserved = Vec::new();
    for previous_card in previous_rich {
        let Some(key) = rich_card_key(&previous_card) else {
            continue;
        };
        let incoming_index = incoming_content.iter().position(|part| {
            valid_rich_card(part) && rich_card_key(part).as_deref() == Some(key.as_str())
        });
        match incoming_index {
            Some(index) if private_card_supersedes(&previous_card, &incoming_content[index]) => {
                incoming_content[index] = previous_card.clone();
                preserved.push(previous_card);
            }
            Some(_) => {}
            None => preserved.push(previous_card),
        }
    }
    if preserved.is_empty() {
        return incoming;
    }
    incoming_content.retain(|part| !placeholder_replaced_by(part, &preserved));
    for part in preserved {
        let Some(key) = rich_card_key(&part) else {
            continue;
        };
        if !incoming_content.iter().any(|candidate| {
            valid_rich_card(candidate) && rich_card_key(candidate).as_deref() == Some(key.as_str())
        }) {
            incoming_content.push(part);
        }
    }
    if let Some(message) = incoming.as_object_mut() {
        message.insert("content".to_string(), Value::Array(incoming_content));
    }
    incoming
}

fn content(message: &Value) -> Vec<Value> {
    message
        .get("content")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn valid_rich_card(part: &Value) -> bool {
    part.get("type").and_then(Value::as_str) == Some("rich_card")
        && part
            .get("richContent")
            .and_then(Value::as_object)
            .is_some_and(|content| {
                content.get("schema").and_then(Value::as_str) == Some("yilong.rich-content.v1")
                    && content.get("kind").and_then(Value::as_str).is_some()
                    && content.get("payload").and_then(Value::as_object).is_some()
            })
}

fn rich_card_kind(part: &Value) -> Option<String> {
    part.get("richContent")?
        .get("kind")?
        .as_str()
        .map(ToOwned::to_owned)
}

fn rich_card_key(part: &Value) -> Option<String> {
    let rich = part.get("richContent")?;
    let kind = rich.get("kind")?.as_str()?;
    let title = rich
        .get("payload")?
        .get("title")
        .and_then(Value::as_str)
        .or_else(|| part.get("text").and_then(Value::as_str))
        .unwrap_or_default();
    Some(format!("{kind}\0{title}"))
}

fn private_card_supersedes(previous: &Value, incoming: &Value) -> bool {
    rich_card_source(previous) == Some("private_response")
        && rich_card_source(incoming) != Some("private_response")
}

fn rich_card_source(part: &Value) -> Option<&str> {
    part.get("richContent")?.get("source")?.as_str()
}

fn placeholder_replaced_by(part: &Value, replacements: &[Value]) -> bool {
    let part_type = part.get("type").and_then(Value::as_str).unwrap_or_default();
    let part_title = part
        .get("text")
        .and_then(Value::as_str)
        .or_else(|| part.get("title").and_then(Value::as_str))
        .unwrap_or_default()
        .trim();
    replacements.iter().any(|replacement| {
        let Some(kind) = rich_card_kind(replacement) else {
            return false;
        };
        let covers_type = match part_type {
            "interactive" => true,
            "chart" | "artifact" => kind == "finance" || kind == "chart",
            "map" => kind == "map",
            "image" => kind == "media_gallery",
            _ => false,
        };
        if !covers_type {
            return false;
        }
        let replacement_title = replacement
            .get("richContent")
            .and_then(|rich| rich.get("payload"))
            .and_then(|payload| payload.get("title"))
            .and_then(Value::as_str)
            .or_else(|| replacement.get("text").and_then(Value::as_str))
            .unwrap_or_default()
            .trim();
        part_title.is_empty()
            || generic_placeholder_title(part_title)
            || replacement_title.is_empty()
            || part_title.eq_ignore_ascii_case(replacement_title)
    })
}

fn generic_placeholder_title(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "交互内容"
            | "互动内容"
            | "图表"
            | "行情图表"
            | "市场行情"
            | "interactive content"
            | "interactive"
            | "chart"
            | "finance chart"
    )
}

#[cfg(test)]
fn with_source(mut message: Value, source: &str) -> Value {
    if let Some(parts) = message.get_mut("content").and_then(Value::as_array_mut) {
        if let Some(rich) = parts
            .iter_mut()
            .find(|part| part.get("type").and_then(Value::as_str) == Some("rich_card"))
            .and_then(|part| part.get_mut("richContent"))
            .and_then(Value::as_object_mut)
        {
            rich.insert("source".to_string(), Value::String(source.to_string()));
        }
    }
    message
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn newer_private_card_wins_without_duplication() {
        let previous = with_source(finance_message("US$77,000.00"), "private_response");
        let incoming = with_source(finance_message("US$78,805.00"), "private_response");

        let merged = preserve_message_rich_content(&previous, incoming);
        let cards = merged["content"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|part| part["type"] == "rich_card")
            .collect::<Vec<_>>();

        assert_eq!(cards.len(), 1);
        assert_eq!(
            cards[0]["richContent"]["payload"]["primaryValue"],
            "US$78,805.00"
        );
    }

    #[test]
    fn private_card_supersedes_same_title_dom_card() {
        let previous = with_source(finance_message("US$78,805.00"), "private_response");
        let incoming = with_source(finance_message("US$77,000.00"), "official_dom");

        let merged = preserve_message_rich_content(&previous, incoming);

        assert_eq!(
            merged["content"][0]["richContent"]["payload"]["primaryValue"],
            "US$78,805.00"
        );
        assert_eq!(
            merged["content"][0]["richContent"]["source"],
            "private_response"
        );
    }

    #[test]
    fn unrelated_interactive_content_is_not_removed() {
        let previous = with_source(finance_message("US$78,805.00"), "private_response");
        let incoming = json!({
            "id":"a1", "role":"assistant", "content":[
                {"type":"interactive", "text":"Bitcoin (BTC)"},
                {"type":"interactive", "text":"另一个独立工具"}
            ]
        });

        let merged = preserve_message_rich_content(&previous, incoming);
        let content = merged["content"].as_array().unwrap();

        assert!(!content
            .iter()
            .any(|part| { part["type"] == "interactive" && part["text"] == "Bitcoin (BTC)" }));
        assert!(content.iter().any(|part| part["text"] == "另一个独立工具"));
        assert!(content.iter().any(|part| part["type"] == "rich_card"));
    }

    #[test]
    fn generic_interactive_placeholder_is_replaced_by_private_finance_card() {
        let previous = with_source(finance_message("US$78,805.00"), "private_response");
        let incoming = json!({
            "id":"a1", "role":"assistant", "content":[
                {"type":"interactive", "text":"交互内容", "kind":"interactive"},
                {"type":"interactive", "text":"另一个独立工具", "kind":"interactive"}
            ]
        });

        let merged = preserve_message_rich_content(&previous, incoming);
        let content = merged["content"].as_array().unwrap();

        assert!(!content.iter().any(|part| part["text"] == "交互内容"));
        assert!(content.iter().any(|part| part["text"] == "另一个独立工具"));
        assert!(content.iter().any(|part| part["type"] == "rich_card"));
    }

    #[test]
    fn unknown_rich_schema_is_not_preserved() {
        let previous = json!({
            "id":"a1", "role":"assistant", "content":[{
                "type":"rich_card", "text":"future",
                "richContent":{"schema":"vendor.future.v2","kind":"finance","payload":{}}
            }]
        });
        let incoming = json!({"id":"a1", "role":"assistant", "content":[]});

        let merged = preserve_message_rich_content(&previous, incoming);

        assert!(merged["content"].as_array().unwrap().is_empty());
    }

    #[test]
    fn position_only_ids_do_not_carry_rich_content_between_snapshots() {
        let mut previous = finance_message("US$77,000.00");
        previous["id"] = Value::String("assistant-1".to_string());
        let mut incoming = json!({"id":"assistant-1", "role":"assistant", "content":[]});

        preserve_private_rich_content(&[previous], std::slice::from_mut(&mut incoming));

        assert!(incoming["content"].as_array().unwrap().is_empty());
    }

    fn finance_message(price: &str) -> Value {
        json!({
            "id":"a1", "role":"assistant", "content":[{
                "type":"rich_card", "text":"Bitcoin (BTC)", "kind":"finance",
                "richContent":{
                    "schema":"yilong.rich-content.v1", "kind":"finance",
                    "payload":{
                        "title":"Bitcoin (BTC)", "primaryValue":price,
                        "trend":"positive", "periods":[], "metrics":[]
                    }
                }
            }]
        })
    }
}
