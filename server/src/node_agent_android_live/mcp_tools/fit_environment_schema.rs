use serde_json::{json, Value};

pub(super) fn fit_environment_schema() -> Value {
    let action = json!({
        "oneOf":[
            {
                "type":"object","required":["type","definitionId"],
                "properties":{
                    "type":{"const":"ACTIVATE_NODE"},
                    "definitionId":{"type":"string","minLength":1,"maxLength":500},
                    "instanceKey":{"type":"string","maxLength":500},
                    "occurrence":{"type":"integer","minimum":0,"maximum":50}
                }
            },
            {
                "type":"object","required":["type"],
                "properties":{"type":{"const":"BACK"}}
            },
            {
                "type":"object","required":["type","durationMs"],
                "properties":{
                    "type":{"const":"WAIT"},
                    "durationMs":{"type":"integer","minimum":100,"maximum":5000}
                }
            }
        ]
    });
    let step = json!({
        "type":"object","required":["name","action"],
        "properties":{
            "name":{"type":"string","minLength":1,"maxLength":80},
            "action":action
        }
    });
    let state_replay = json!({
        "type":"object",
        "description":"非根页面 patch-free 重装后使用的受控、限时、可持久化页面 trace；scenarioId 必须与 environment.scenario 一致。",
        "required":["scenarioId","capturedAt","expiresAt","steps"],
        "properties":{
            "schemaVersion":{"const":1},
            "scenarioId":{"type":"string","minLength":1,"maxLength":128},
            "capturedAt":{"type":"string","format":"date-time"},
            "expiresAt":{"type":"string","format":"date-time"},
            "steps":{"type":"array","minItems":1,"maxItems":16,"items":step}
        }
    });
    json!({
        "type":"object",
        "properties":{
            "screenId":{"type":"string","maxLength":500},
            "scenario":{"type":"string","maxLength":128},
            "theme":{"type":"string","maxLength":128},
            "locale":{"type":"string","maxLength":128},
            "stateReplay":state_replay
        }
    })
}

#[cfg(test)]
mod tests {
    use super::fit_environment_schema;

    #[test]
    fn schema_exposes_expiring_activate_node_trace() {
        let schema = fit_environment_schema();
        let replay = &schema["properties"]["stateReplay"];
        assert!(replay["required"]
            .as_array()
            .is_some_and(|values| values.iter().any(|value| value == "expiresAt")));
        assert_eq!(
            replay["properties"]["steps"]["items"]["properties"]["action"]["oneOf"][0]
                ["properties"]["type"]["const"],
            "ACTIVATE_NODE"
        );
    }
}
