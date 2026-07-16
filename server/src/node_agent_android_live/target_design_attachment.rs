use anyhow::{anyhow, bail, Result};
use serde_json::Value;

pub(super) fn select_target_design_entry<'a>(
    envelope: &Value,
    entries: &'a [Value],
    requested_id: Option<&str>,
) -> Result<&'a Value> {
    let task_target_id = envelope
        .pointer("/task/target_design_attachment_id")
        .or_else(|| envelope.pointer("/task/targetDesignAttachmentId"))
        .and_then(Value::as_str);
    let preferred_id = requested_id.or(task_target_id);
    let overall_target = envelope
        .pointer("/task/attachment_intent")
        .or_else(|| envelope.pointer("/task/attachmentIntent"))
        .and_then(Value::as_str)
        .is_some_and(|intent| intent.eq_ignore_ascii_case("TARGET_DESIGN"));
    let entry = if let Some(attachment_id) = preferred_id {
        entries
            .iter()
            .find(|entry| entry_attachment_id(entry) == Some(attachment_id))
            .ok_or_else(|| anyhow!("找不到目标设计附件: {attachment_id}"))?
    } else if let Some(entry) = entries.iter().find(|entry| entry_is_target_design(entry)) {
        entry
    } else if overall_target {
        entries
            .first()
            .ok_or_else(|| anyhow!("找不到目标设计附件"))?
    } else {
        bail!("设计任务中没有 TARGET_DESIGN 附件；标注层和风格参考不得参与像素拟合");
    };
    if !entry_is_target_design(entry) && !overall_target {
        let intent = entry
            .pointer("/metadata/intent")
            .and_then(Value::as_str)
            .unwrap_or("AUTO");
        bail!("只有 TARGET_DESIGN 附件可绑定为像素目标；选中附件是 {intent}");
    }
    Ok(entry)
}

fn entry_attachment_id(entry: &Value) -> Option<&str> {
    entry
        .pointer("/metadata/attachment_id")
        .or_else(|| entry.pointer("/metadata/attachmentId"))
        .and_then(Value::as_str)
}

fn entry_is_target_design(entry: &Value) -> bool {
    entry
        .pointer("/metadata/intent")
        .and_then(Value::as_str)
        .is_some_and(|intent| intent.eq_ignore_ascii_case("TARGET_DESIGN"))
}

#[cfg(test)]
mod tests {
    use super::{entry_attachment_id, select_target_design_entry};
    use serde_json::json;

    #[test]
    fn mixed_attachment_task_binds_attachment_level_target_design() {
        let envelope = json!({"task": {
            "attachmentIntent": "AUTO",
            "targetDesignAttachmentId": "target"
        }});
        let entries = vec![
            json!({"metadata":{"attachmentId":"target", "intent":"TARGET_DESIGN"}}),
            json!({"metadata":{"attachmentId":"reference", "intent":"REFERENCE_STYLE"}}),
        ];

        let selected = select_target_design_entry(&envelope, &entries, None).unwrap();

        assert_eq!(entry_attachment_id(selected), Some("target"));
    }

    #[test]
    fn explicit_reference_attachment_cannot_be_bound_as_target() {
        let envelope = json!({"task": {"attachmentIntent":"AUTO"}});
        let entries = vec![
            json!({"metadata":{"attachmentId":"target", "intent":"TARGET_DESIGN"}}),
            json!({"metadata":{"attachmentId":"reference", "intent":"REFERENCE_STYLE"}}),
        ];

        let error = select_target_design_entry(&envelope, &entries, Some("reference")).unwrap_err();

        assert!(error.to_string().contains("REFERENCE_STYLE"));
    }
}
