use serde_json::json;

use super::protocol::{
    LivePatchOperation, LivePatchTarget, LivePropertyValue, LiveStylePatch, PROTOCOL_VERSION,
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
