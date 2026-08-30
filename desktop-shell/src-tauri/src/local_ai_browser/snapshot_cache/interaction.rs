use std::collections::BTreeMap;

use serde_json::Value;

const MAX_INTERACTION_BYTES: usize = 128 * 1024;
const MAX_ITEMS: usize = 100;

pub(super) fn sanitize_composer(value: Option<&Value>) -> Option<Value> {
    let mut event = bounded_event(value, "composer_controls_snapshot")?;
    let section = event.get("section")?.as_str()?;
    if !matches!(section, "model" | "tools") {
        return None;
    }
    sanitize_items(&mut event, "options")?;
    Some(event)
}

pub(super) fn sanitize_composers(values: &BTreeMap<String, Value>) -> BTreeMap<String, Value> {
    values
        .iter()
        .filter_map(|(section, value)| {
            let sanitized = sanitize_composer(Some(value))?;
            (sanitized.get("section").and_then(Value::as_str) == Some(section.as_str()))
                .then(|| (section.clone(), sanitized))
        })
        .collect()
}

pub(super) fn sanitize_features(value: Option<&Value>) -> Option<Value> {
    let mut event = bounded_event(value, "navigation_snapshot")?;
    sanitize_items(&mut event, "features")?;
    Some(event)
}

fn bounded_event(value: Option<&Value>, expected_type: &str) -> Option<Value> {
    let event = value?.as_object()?;
    if event.get("type").and_then(Value::as_str) != Some(expected_type) {
        return None;
    }
    if serde_json::to_vec(event).ok()?.len() > MAX_INTERACTION_BYTES {
        return None;
    }
    Some(Value::Object(event.clone()))
}

fn sanitize_items(event: &mut Value, field: &str) -> Option<()> {
    let items = event.get_mut(field)?.as_array_mut()?;
    items.truncate(MAX_ITEMS);
    items.retain(|item| {
        item.get("id")
            .and_then(Value::as_str)
            .is_some_and(|id| !id.is_empty())
            && item
                .get("label")
                .and_then(Value::as_str)
                .is_some_and(|label| !label.is_empty())
    });
    for item in items {
        item.as_object_mut()?
            .insert("selected".to_string(), Value::Bool(false));
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cached_interactions_are_bounded_and_never_keep_stale_selection() {
        let composer = sanitize_composer(Some(&json!({
            "type":"composer_controls_snapshot", "section":"model", "currentModel":"Auto",
            "options":[{"id":"auto","label":"Auto","selected":true,"kind":"","semantic":"model","opensSubmenu":false}]
        }))).unwrap();
        assert_eq!(composer["options"][0]["selected"], false);
        assert!(sanitize_composer(Some(&json!({
            "type":"composer_controls_snapshot", "section":"unknown", "options":[]
        })))
        .is_none());

        let features = sanitize_features(Some(&json!({
            "type":"navigation_snapshot", "features":[{"id":"images","label":"Images","selected":true,"kind":"images"}]
        }))).unwrap();
        assert_eq!(features["features"][0]["selected"], false);
    }
}
