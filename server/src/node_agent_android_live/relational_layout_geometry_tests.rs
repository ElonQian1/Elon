use serde_json::json;

use super::{evaluate_trace, GeometryAssertion};

fn trace() -> serde_json::Value {
    json!({
        "deviceId": "device-1",
        "packageName": "com.example",
        "states": [
            state("home", "bottomNav", "com.example:id/bottomNav", [70, 2498, 964, 2694]),
            state("chat", "inputCapsule", "com.example:id/inputCapsule", [70, 2498, 1202, 2694]),
            state("homeAgain", "bottomNav", "com.example:id/bottomNav", [70, 2498, 964, 2694])
        ]
    })
}

fn state(name: &str, label: &str, resource_id: &str, bounds: [i64; 4]) -> serde_json::Value {
    json!({
        "name": name,
        "insets": {
            "displayWidth": 1260,
            "displayHeight": 2800,
            "systemBars": {"left":0,"top":80,"right":0,"bottom":78}
        },
        "nodes": [{
            "label": label,
            "matched": true,
            "resourceId": resource_id,
            "text": null,
            "bounds": {
                "left": bounds[0], "top": bounds[1],
                "right": bounds[2], "bottom": bounds[3]
            }
        }]
    })
}

fn assertions(value: serde_json::Value) -> Vec<GeometryAssertion> {
    serde_json::from_value(value).unwrap()
}

#[test]
fn evaluates_cross_state_bottom_alignment_and_safe_area_gap() {
    let assertions = assertions(json!([
        {
            "name":"home and chat bottoms align",
            "left":{"step":"home","source":"NODE","selector":"bottomNav","anchor":"BOTTOM"},
            "right":{"step":"chat","source":"NODE","selector":"inputCapsule","anchor":"BOTTOM"},
            "expectedDeltaPx":0,"tolerancePx":1
        },
        {
            "name":"home bottom gap to safe content",
            "left":{"step":"home","source":"SAFE_CONTENT","anchor":"BOTTOM"},
            "right":{"step":"home","source":"NODE","selector":"bottomNav","anchor":"BOTTOM"},
            "expectedDeltaPx":28,"tolerancePx":0
        }
    ]));
    let result = evaluate_trace(&trace(), &assertions).unwrap();
    assert_eq!(result["status"], "PASSED");
    assert_eq!(result["summary"]["passed"], 2);
    assert_eq!(result["assertions"][0]["deltaPx"], 0);
    assert_eq!(result["assertions"][1]["deltaPx"], 28);
}

#[test]
fn reports_missing_node_as_failed_assertion_instead_of_hiding_it() {
    let assertions = assertions(json!([{
        "name":"missing",
        "left":{"step":"home","source":"NODE","selector":"unknown","anchor":"BOTTOM"},
        "right":{"step":"chat","source":"DISPLAY","anchor":"BOTTOM"},
        "expectedDeltaPx":0,"tolerancePx":1
    }]));
    let result = evaluate_trace(&trace(), &assertions).unwrap();
    assert_eq!(result["status"], "FAILED");
    assert_eq!(result["summary"]["failed"], 1);
    assert!(result["assertions"][0]["error"]
        .as_str()
        .unwrap()
        .contains("unknown"));
}

#[test]
fn emits_stable_selector_evidence_across_revisited_state() {
    let assertions = assertions(json!([{
        "name":"home remains aligned after return",
        "left":{"step":"home","source":"NODE","selector":"bottomNav","anchor":"BOTTOM"},
        "right":{"step":"homeAgain","source":"NODE","selector":"bottomNav","anchor":"BOTTOM"},
        "expectedDeltaPx":0,"tolerancePx":0
    }]));
    let result = evaluate_trace(&trace(), &assertions).unwrap();
    assert_eq!(result["selectorStability"][0]["stable"], true);
    assert_eq!(
        result["selectorStability"][0]["matchedSteps"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
