use serde_json::json;

use super::broker::patches_share_gesture;
use super::protocol::{
    LivePatchOperation, LivePatchTarget, LivePropertyValue, LiveStylePatch, LiveUiNode,
    PROTOCOL_VERSION,
};

fn patch(property: &str, value_type: &str, value: serde_json::Value) -> LiveStylePatch {
    LiveStylePatch {
        protocol_version: PROTOCOL_VERSION,
        message_type: String::new(),
        session_id: String::new(),
        request_id: String::new(),
        gesture_id: None,
        sequence: 0,
        base_tree_revision: None,
        target: LivePatchTarget {
            scope: "INSTANCE".to_string(),
            runtime_node_id: Some("rn_1".to_string()),
            definition_id: Some("home.action".to_string()),
            instance_key: None,
        },
        atomic: true,
        ephemeral: true,
        operations: vec![LivePatchOperation {
            property: property.to_string(),
            value: LivePropertyValue {
                value_type: value_type.to_string(),
                value,
            },
        }],
    }
}

#[test]
fn accepts_typed_live_style_properties() {
    assert!(patch("height", "dp", json!(56)).validate().is_ok());
    assert!(patch("backgroundColor", "argb", json!("#FF112233"))
        .validate()
        .is_ok());
    assert!(patch("opacity", "float", json!(0.7)).validate().is_ok());
    assert!(patch("text", "text", json!("立即支付")).validate().is_ok());
}

#[test]
fn accepts_live_typography_properties() {
    for (property, value_type, value) in [
        ("fontWeight", "float", serde_json::json!(650)),
        ("lineHeight", "sp", serde_json::json!(22)),
        ("letterSpacing", "float", serde_json::json!(0.02)),
    ] {
        let value = patch(property, value_type, value);
        assert!(value.validate().is_ok(), "{property} should be editable");
    }
}

#[test]
fn rejects_unknown_properties_and_invalid_values() {
    assert!(patch("onClick", "text", json!("rm -rf"))
        .validate()
        .is_err());
    assert!(patch("backgroundColor", "argb", json!("red"))
        .validate()
        .is_err());
    assert!(patch("opacity", "float", json!(1.5)).validate().is_err());
}

#[test]
fn prepare_binds_patch_to_session_and_request() {
    let mut value = patch("padding.start", "dp", json!(16));
    value.prepare("live_demo");
    assert_eq!(value.session_id, "live_demo");
    assert_eq!(value.message_type, "patch.apply");
    assert!(value.request_id.starts_with("req_"));
}

#[test]
fn coalesces_continuous_canvas_patches_into_one_gesture() {
    let mut first = patch("width", "dp", json!(100));
    first.gesture_id = Some("gesture-1".to_string());
    let mut next = patch("width", "dp", json!(160));
    next.gesture_id = Some("gesture-1".to_string());
    assert!(patches_share_gesture(&first, &next));

    next.gesture_id = Some("gesture-2".to_string());
    assert!(!patches_share_gesture(&first, &next));
}

#[test]
fn accepts_android_tree_nodes_with_omitted_null_fields() {
    let node: LiveUiNode = serde_json::from_value(json!({
        "runtimeNodeId": "rn_1",
        "definitionId": "checkout.pay_button",
        "screenId": "checkout",
        "kind": "material.button",
        "className": "com.google.android.material.button.MaterialButton",
        "geometry": {
            "boundsInDisplayPx": {
                "left": 0, "top": 10, "right": 100, "bottom": 58,
                "width": 100, "height": 48
            },
            "density": 3.0,
            "fontScale": 1.0,
            "rotation": 0,
            "visible": true
        },
        "properties": {
            "height": {
                "effective": { "type": "dp", "value": 48 },
                "changeLevel": "LIVE",
                "commitMode": "CODEX"
            }
        },
        "capabilities": { "resizeHeight": true }
    }))
    .expect("Android Gson 默认省略 null 字段时仍应兼容");

    assert_eq!(node.definition_id, "checkout.pay_button");
    assert!(node.instance_key.is_none());
    assert!(node.properties["height"].binding.is_none());
}
