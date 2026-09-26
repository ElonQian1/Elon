use serde_json::{json, Value};

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        json!({
            "name":"win_group_ai_action",
            "description":"Use the signed-in Win user's production group AI workflow without clicks. groups/messages are read-only, returning bounded previews and an owner binding. start sends selected original attachments and the question to ChatGPT and publishes the completed answer to the same group; requires explicit user authorization and confirmed=true. It never enables conversation sharing. Reuse command_id after a transport failure; do not create another start. status verifies the group result; resume only checks an already-dispatched task without resending. Queued/completed command receipts are NOT proof the AI task completed.",
            "inputSchema":{"type":"object","additionalProperties":false,"required":["command_id","action"],"properties":{
                "command_id":{"type":"string","format":"uuid"},
                "action":{"type":"string","enum":["groups","messages","start","status","resume","cancel"]},
                "owner_binding":{"type":"string","maxLength":100},
                "group_id":{"type":"string","maxLength":100},
                "message_ids":{"type":"array","minItems":1,"maxItems":50,"uniqueItems":true,"items":{"type":"string","maxLength":100}},
                "message_revisions":{"type":"object","maxProperties":50,"additionalProperties":{"type":"integer","minimum":1}},
                "question":{"type":"string","maxLength":2000},
                "task_id":{"type":"string","format":"uuid"},
                "confirmed":{"type":"boolean","default":false},
                "offset":{"type":"integer","minimum":0,"maximum":10000}
            }}
        }),
        json!({
            "name":"win_group_ai_action_status",
            "description":"Read a project-bound command receipt. For a started task, use win_group_ai_action action=status with task_id until phase=completed AND delivery_verified=true AND source_verified=true. Never treat a queued receipt as successful delivery.",
            "inputSchema":{"type":"object","additionalProperties":false,"required":["command_id"],"properties":{"command_id":{"type":"string","format":"uuid"}}}
        }),
    ]
}
