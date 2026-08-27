use std::collections::{HashMap, HashSet};

use serde_json::{json, Map, Value};

pub(super) fn merge(previous: Option<&Value>, mut incoming: Value) -> Value {
    let complete = is_complete(&incoming);
    let previous_conversations = records(previous, "conversations");
    let incoming_conversations = records(Some(&incoming), "conversations");
    let incoming_conversation_count = incoming_conversations.len();
    let previous_projects = records(previous, "projects");
    let incoming_projects = records(Some(&incoming), "projects");

    let conversations = merge_conversations(
        previous_conversations,
        incoming_conversations,
        complete,
    );
    let projects = merge_projects(previous_projects, incoming_projects);
    let observed_count = incoming
        .get("collection")
        .and_then(Value::as_object)
        .and_then(|collection| collection.get("observedCount"))
        .and_then(Value::as_u64)
        .unwrap_or(incoming_conversation_count as u64);
    let available_count = conversations.len() as u64;

    if let Some(snapshot) = incoming.as_object_mut() {
        snapshot.insert("conversations".to_string(), Value::Array(conversations));
        snapshot.insert("projects".to_string(), Value::Array(projects));
        let collection = snapshot
            .entry("collection")
            .or_insert_with(|| json!({}))
            .as_object_mut();
        if let Some(collection) = collection {
            collection.insert("complete".to_string(), Value::Bool(complete));
            collection.insert("observedCount".to_string(), Value::from(observed_count));
            collection.insert("availableCount".to_string(), Value::from(available_count));
            collection.insert(
                "source".to_string(),
                Value::String(if complete {
                    "official_complete"
                } else {
                    "official_partial"
                }
                .to_string()),
            );
        }
    }
    incoming
}

pub(super) fn is_complete(snapshot: &Value) -> bool {
    let Some(collection) = snapshot.get("collection").and_then(Value::as_object) else {
        return false;
    };
    collection
        .get("complete")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| {
            collection
                .get("reachedEnd")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && collection
                    .get("scrollRestored")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                && !collection
                    .get("truncated")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                && !collection
                    .get("timedOut")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
}

fn records(snapshot: Option<&Value>, key: &str) -> Vec<Value> {
    snapshot
        .and_then(|snapshot| snapshot.get(key))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn merge_conversations(
    previous: Vec<Value>,
    incoming: Vec<Value>,
    complete: bool,
) -> Vec<Value> {
    let previous_by_path = indexed(&previous);
    let observed_paths = incoming
        .iter()
        .filter_map(path)
        .map(str::to_string)
        .collect::<HashSet<_>>();
    let mut merged = incoming
        .into_iter()
        .filter_map(|next| {
            let identity = path(&next)?.to_string();
            Some(match previous_by_path.get(&identity) {
                Some(old) => combine_conversation(old, next, complete),
                None => next,
            })
        })
        .collect::<Vec<_>>();
    for mut previous in previous {
        let Some(identity) = path(&previous).map(str::to_string) else {
            continue;
        };
        if observed_paths.contains(&identity) {
            continue;
        }
        if !complete || is_project_conversation(&previous) {
            if let Some(previous) = previous.as_object_mut() {
                previous.insert("active".to_string(), Value::Bool(false));
            }
            merged.push(previous);
        }
    }
    merged.truncate(100);
    merged
}

fn merge_projects(previous: Vec<Value>, incoming: Vec<Value>) -> Vec<Value> {
    let previous_by_path = indexed(&previous);
    let observed_paths = incoming
        .iter()
        .filter_map(path)
        .map(str::to_string)
        .collect::<HashSet<_>>();
    let mut merged = incoming
        .into_iter()
        .filter_map(|next| {
            let identity = path(&next)?.to_string();
            Some(match previous_by_path.get(&identity) {
                Some(old) => combine_project(old, next),
                None => next,
            })
        })
        .collect::<Vec<_>>();
    merged.extend(previous.into_iter().filter(|project| {
        path(project).is_some_and(|identity| !observed_paths.contains(identity))
    }));
    merged.truncate(40);
    merged
}

fn indexed(values: &[Value]) -> HashMap<String, Value> {
    let mut indexed = HashMap::new();
    for value in values {
        let Some(identity) = path(&value).map(str::to_string) else {
            continue;
        };
        indexed.insert(identity, value.clone());
    }
    indexed
}

fn combine_conversation(old: &Value, mut next: Value, complete: bool) -> Value {
    let Some(next_object) = next.as_object_mut() else {
        return next;
    };
    let Some(old_object) = old.as_object() else {
        return next;
    };
    preserve_text(next_object, old_object, "title");
    preserve_text(next_object, old_object, "groupLabel");
    preserve_nullable(next_object, old_object, "projectId");
    preserve_text(next_object, old_object, "projectTitle");
    preserve_nullable(next_object, old_object, "projectPath");
    let pinned = next_object
        .get("pinned")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || (!complete
            && old_object
                .get("pinned")
                .and_then(Value::as_bool)
                .unwrap_or(false));
    next_object.insert("pinned".to_string(), Value::Bool(pinned));
    next_object.insert(
        "activityDates".to_string(),
        Value::Array(merged_dates(old_object, next_object)),
    );
    next
}

fn combine_project(old: &Value, mut next: Value) -> Value {
    let Some(next_object) = next.as_object_mut() else {
        return next;
    };
    let Some(old_object) = old.as_object() else {
        return next;
    };
    preserve_text(next_object, old_object, "title");
    next
}

fn preserve_text(next: &mut Map<String, Value>, old: &Map<String, Value>, key: &str) {
    let missing = next
        .get(key)
        .and_then(Value::as_str)
        .is_none_or(|value| value.trim().is_empty());
    if missing {
        if let Some(value) = old.get(key).cloned() {
            next.insert(key.to_string(), value);
        }
    }
}

fn preserve_nullable(next: &mut Map<String, Value>, old: &Map<String, Value>, key: &str) {
    let missing = next
        .get(key)
        .is_none_or(|value| value.is_null() || value.as_str().is_some_and(str::is_empty));
    if missing {
        if let Some(value) = old.get(key).cloned() {
            next.insert(key.to_string(), value);
        }
    }
}

fn merged_dates(old: &Map<String, Value>, next: &Map<String, Value>) -> Vec<Value> {
    let mut seen = HashSet::new();
    let mut dates = next
        .get("activityDates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .chain(
            old.get("activityDates")
                .and_then(Value::as_array)
                .into_iter()
                .flatten(),
        )
        .filter_map(Value::as_str)
        .filter(|date| seen.insert((*date).to_string()))
        .map(|date| Value::String(date.to_string()))
        .collect::<Vec<_>>();
    dates.truncate(32);
    dates
}

fn path(value: &Value) -> Option<&str> {
    value.get("path").and_then(Value::as_str).filter(|path| !path.is_empty())
}

fn is_project_conversation(value: &Value) -> bool {
    value
        .get("projectId")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_directory_retains_cached_items_projects_and_pins() {
        let previous = json!({
            "type":"conversation_snapshot",
            "conversations":[
                {"path":"/c/one","title":"One","pinned":true,"active":false,"activityDates":["2026-08-15"]},
                {"path":"/c/two","title":"Two","pinned":false,"active":false,"activityDates":[]}
            ],
            "projects":[{"path":"/g/g-p-roadmap/project","id":"g-p-roadmap","title":"Roadmap"}]
        });
        let incoming = json!({
            "type":"conversation_snapshot",
            "conversations":[{"path":"/c/one","title":"One now","pinned":false,"active":true,"activityDates":["2026-08-16"]}],
            "projects":[],
            "collection":{"complete":false,"observedCount":1}
        });
        let merged = merge(Some(&previous), incoming);
        assert_eq!(merged["conversations"].as_array().unwrap().len(), 2);
        assert_eq!(merged["conversations"][0]["pinned"], true);
        assert_eq!(merged["conversations"][0]["activityDates"].as_array().unwrap().len(), 2);
        assert_eq!(merged["projects"].as_array().unwrap().len(), 1);
        assert_eq!(merged["collection"]["source"], "official_partial");
    }

    #[test]
    fn complete_directory_prunes_missing_global_chats_but_keeps_project_chats() {
        let previous = json!({
            "type":"conversation_snapshot",
            "conversations":[
                {"path":"/c/global","title":"Global","projectId":null},
                {"path":"/g/g-p-roadmap/c/project-chat","title":"Project","projectId":"g-p-roadmap"}
            ],
            "projects":[]
        });
        let incoming = json!({
            "type":"conversation_snapshot",
            "conversations":[],
            "projects":[],
            "collection":{"complete":true,"observedCount":0}
        });
        let merged = merge(Some(&previous), incoming);
        let conversations = merged["conversations"].as_array().unwrap();
        assert_eq!(conversations.len(), 1);
        assert_eq!(conversations[0]["path"], "/g/g-p-roadmap/c/project-chat");
        assert_eq!(merged["collection"]["source"], "official_complete");
    }
}
