use std::collections::BTreeMap;
use std::fs;

use serde_json::json;

use super::broker::LiveCommitSnapshot;
use super::protocol::{
    LiveGeometry, LivePatchOperation, LivePatchTarget, LivePropertySnapshot, LivePropertyValue,
    LiveStylePatch, LiveUiNode, PROTOCOL_VERSION,
};
use super::source_commit::{apply_source_commit_plan, build_plan, SourceCommitRequest};

#[test]
fn plans_and_writes_bound_android_resources() {
    let root = std::env::temp_dir().join(format!(
        "elon-live-source-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let layout_dir = root.join("app/src/main/res/layout");
    let values_dir = root.join("app/src/main/res/values");
    fs::create_dir_all(&layout_dir).expect("create layout dir");
    fs::create_dir_all(&values_dir).expect("create values dir");
    fs::write(
        layout_dir.join("checkout.xml"),
        r#"<LinearLayout xmlns:android="http://schemas.android.com/apk/res/android">
    <TextView
        android:id="@+id/pay_button"
        android:layout_width="match_parent"
        android:layout_height="@dimen/pay_height"
        android:textColor="@color/pay_color" />
</LinearLayout>"#,
    )
    .expect("write layout");
    fs::write(
        values_dir.join("dimens.xml"),
        r#"<resources><dimen name="pay_height">48dp</dimen></resources>"#,
    )
    .expect("write dimens");
    fs::write(
        values_dir.join("colors.xml"),
        r#"<resources><color name="pay_color">#FF112233</color></resources>"#,
    )
    .expect("write colors");

    let snapshot = LiveCommitSnapshot {
        project_root: Some(root.display().to_string()),
        nodes: vec![live_node()],
        patches: vec![patch(vec![
            operation("height", "dp", json!(56)),
            operation("contentColor", "argb", json!("#FF6750A4")),
        ])],
    };
    let plan = build_plan("live_test", snapshot).expect("build source commit plan");
    assert_eq!(plan.deterministic_count, 2);
    assert_eq!(plan.codex_count, 0);
    assert!(plan.entries.iter().any(|entry| {
        entry.source_key.as_deref() == Some("dimen:pay_height")
            && entry.old_value.as_deref() == Some("48dp")
    }));

    let revision = plan.source_revision.clone();
    let result = apply_source_commit_plan(
        plan,
        SourceCommitRequest {
            source_revision: revision,
        },
    )
    .expect("commit source plan");
    assert_eq!(result.status, "SOURCE_SAVED");
    assert_eq!(result.committed_count, 2);
    assert!(fs::read_to_string(values_dir.join("dimens.xml"))
        .expect("read dimens")
        .contains(">56dp</dimen>"));
    assert!(fs::read_to_string(values_dir.join("colors.xml"))
        .expect("read colors")
        .contains(">#FF6750A4</color>"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn plans_and_writes_compose_style_and_token_json() {
    let root =
        std::env::temp_dir().join(format!("elon-live-json-{}", uuid::Uuid::new_v4().simple()));
    let style_dir = root.join(".elon/ui-styles");
    let token_dir = root.join(".elon/ui-standards");
    fs::create_dir_all(&style_dir).expect("create style dir");
    fs::create_dir_all(&token_dir).expect("create token dir");
    let style_file = style_dir.join("checkout.json");
    let token_file = token_dir.join("tokens.json");
    fs::write(
        &style_file,
        r#"{"payButton":{"cornerRadius":16,"paddingStart":20}}"#,
    )
    .expect("write style json");
    fs::write(
        &token_file,
        r##"{"colors":{"action":{"primary":"#FF5D3FD3"}}}"##,
    )
    .expect("write token json");

    let mut node = live_node();
    node.resource_id = None;
    node.definition_id = "checkout.pay_button.compose".to_string();
    node.properties.insert(
        "cornerRadius.all".to_string(),
        bound_property(json!({
            "kind": "STYLE_JSON",
            "relativeFile": ".elon/ui-styles/checkout.json",
            "jsonPointer": "/payButton/cornerRadius"
        })),
    );
    node.properties.insert(
        "backgroundColor".to_string(),
        bound_property(json!({
            "kind": "TOKEN",
            "path": "colors.action.primary"
        })),
    );
    let mut style_patch = patch(vec![
        operation("cornerRadius.all", "dp", json!(18)),
        operation("backgroundColor", "argb", json!("#FF6750A4")),
    ]);
    style_patch.target.definition_id = Some(node.definition_id.clone());

    let plan = build_plan(
        "live_json",
        LiveCommitSnapshot {
            project_root: Some(root.display().to_string()),
            nodes: vec![node],
            patches: vec![style_patch],
        },
    )
    .expect("build JSON source commit plan");
    assert_eq!(plan.deterministic_count, 2);
    assert_eq!(plan.codex_count, 0);
    assert!(plan.entries.iter().any(|entry| {
        entry.source_key.as_deref() == Some("json:/payButton/cornerRadius")
            && entry.old_value.as_deref() == Some("16")
    }));
    assert!(plan.entries.iter().any(|entry| {
        entry.source_key.as_deref() == Some("token:colors.action.primary")
            && entry.old_value.as_deref() == Some("#FF5D3FD3")
    }));

    let revision = plan.source_revision.clone();
    let result = apply_source_commit_plan(
        plan,
        SourceCommitRequest {
            source_revision: revision,
        },
    )
    .expect("commit JSON source plan");
    assert_eq!(result.committed_count, 2);
    let style: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(style_file).expect("read style json"))
            .expect("parse style json");
    let tokens: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(token_file).expect("read token json"))
            .expect("parse token json");
    assert_eq!(style.pointer("/payButton/cornerRadius"), Some(&json!(18)));
    assert_eq!(
        tokens.pointer("/colors/action/primary"),
        Some(&json!("#FF6750A4"))
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn defers_single_instance_commit_for_repeated_definition() {
    let root = std::env::temp_dir().join(format!(
        "elon-live-repeat-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let layout_dir = root.join("app/src/main/res/layout");
    fs::create_dir_all(&layout_dir).expect("create layout dir");
    fs::write(
        layout_dir.join("item.xml"),
        r#"<TextView xmlns:android="http://schemas.android.com/apk/res/android"
    android:id="@+id/pay_button"
    android:layout_width="48dp"
    android:layout_height="48dp" />"#,
    )
    .expect("write layout");
    let mut second = live_node();
    second.runtime_node_id = "rn_2".to_string();
    second.instance_key = Some("order:2".to_string());
    let mut first = live_node();
    first.instance_key = Some("order:1".to_string());
    let mut instance_patch = patch(vec![operation("height", "dp", json!(60))]);
    instance_patch.target.scope = "INSTANCE".to_string();
    instance_patch.target.runtime_node_id = Some("rn_1".to_string());
    let plan = build_plan(
        "live_repeat",
        LiveCommitSnapshot {
            project_root: Some(root.display().to_string()),
            nodes: vec![first, second],
            patches: vec![instance_patch],
        },
    )
    .expect("build repeated source plan");
    assert_eq!(plan.deterministic_count, 0);
    assert_eq!(plan.entries[0].commit_mode, "SESSION_ONLY");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn mixed_plan_writes_deterministic_entries_and_preserves_codex_deferred_entries() {
    let root = std::env::temp_dir().join(format!(
        "elon-live-mixed-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let layout_dir = root.join("app/src/main/res/layout");
    fs::create_dir_all(&layout_dir).expect("create layout dir");
    let layout_file = layout_dir.join("checkout.xml");
    fs::write(
        &layout_file,
        r#"<TextView xmlns:android="http://schemas.android.com/apk/res/android"
    android:id="@+id/pay_button"
    android:layout_width="match_parent"
    android:layout_height="48dp" />"#,
    )
    .expect("write layout");

    let plan = build_plan(
        "live_mixed",
        LiveCommitSnapshot {
            project_root: Some(root.display().to_string()),
            nodes: vec![live_node()],
            patches: vec![patch(vec![
                operation("height", "dp", json!(56)),
                operation("parentArrangement", "enum", json!("spaceBetween")),
            ])],
        },
    )
    .expect("build mixed plan");
    assert_eq!(plan.deterministic_count, 1);
    assert_eq!(plan.codex_count, 1);
    let revision = plan.source_revision.clone();

    let result = apply_source_commit_plan(
        plan,
        SourceCommitRequest {
            source_revision: revision,
        },
    )
    .expect("commit deterministic subset");

    assert_eq!(result.committed_count, 1);
    assert_eq!(result.deferred_count, 1);
    assert_eq!(result.deferred[0].property, "parentArrangement");
    assert!(fs::read_to_string(layout_file)
        .expect("read layout")
        .contains("android:layout_height=\"56dp\""));

    let _ = fs::remove_dir_all(root);
}

fn live_node() -> LiveUiNode {
    LiveUiNode {
        runtime_node_id: "rn_1".to_string(),
        definition_id: "checkout.pay_button".to_string(),
        instance_key: None,
        parent_runtime_node_id: None,
        screen_id: "checkout".to_string(),
        kind: "android.text".to_string(),
        text: Some("立即支付".to_string()),
        resource_id: Some("com.example:id/pay_button".to_string()),
        class_name: "android.widget.TextView".to_string(),
        source: None,
        geometry: LiveGeometry::default(),
        properties: BTreeMap::new(),
        capabilities: BTreeMap::new(),
    }
}

fn patch(operations: Vec<LivePatchOperation>) -> LiveStylePatch {
    LiveStylePatch {
        protocol_version: PROTOCOL_VERSION,
        message_type: "patch.apply".to_string(),
        session_id: "live_test".to_string(),
        request_id: "req_test".to_string(),
        gesture_id: None,
        sequence: 1,
        base_tree_revision: None,
        target: LivePatchTarget {
            scope: "DEFINITION".to_string(),
            runtime_node_id: None,
            definition_id: Some("checkout.pay_button".to_string()),
            instance_key: None,
        },
        atomic: true,
        ephemeral: true,
        operations,
    }
}

fn operation(property: &str, value_type: &str, value: serde_json::Value) -> LivePatchOperation {
    LivePatchOperation {
        property: property.to_string(),
        value: LivePropertyValue {
            value_type: value_type.to_string(),
            value,
        },
    }
}

fn bound_property(binding: serde_json::Value) -> LivePropertySnapshot {
    LivePropertySnapshot {
        effective: None,
        measured: None,
        change_level: "LIVE".to_string(),
        commit_mode: "DETERMINISTIC".to_string(),
        binding: Some(binding),
        constraints: None,
    }
}
