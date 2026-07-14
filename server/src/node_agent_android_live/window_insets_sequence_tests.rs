use super::*;

#[test]
fn parses_visible_system_bar_and_ime_insets() {
    let raw = r#"mCurrentFocus=Window{abc u0 com.elon.app/.MainActivity}
source=InsetsSource: {type=statusBars frame=[0,0][1080,126] visible=true}
source=InsetsSource: {type=navigationBars frame=[0,2274][1080,2400] visible=true}
source=InsetsSource: {type=ime frame=[0,1200][1080,2400] visible=false}"#;
    let parsed = parse_window_insets(raw, 1080, 2400).unwrap();
    assert_eq!(
        parsed.system_bars,
        InsetsEdges {
            left: 0,
            top: 126,
            right: 0,
            bottom: 126
        }
    );
    assert_eq!(
        parsed.ime,
        InsetsEdges {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0
        }
    );
    assert!(parsed.current_focus.unwrap().contains("MainActivity"));
}

#[test]
fn matches_resource_id_suffix_without_debug_package_coupling() {
    let node = RuntimeUiNode {
        id: "node-1".into(),
        depth: 1,
        index_path: vec![0],
        xpath: "/hierarchy/node[0]".into(),
        text: "全部".into(),
        content_desc: String::new(),
        resource_id: Some("com.elon.app.uitest:id/toolbar".into()),
        package_name: Some("com.elon.app.uitest".into()),
        class_name: Some("android.view.ViewGroup".into()),
        bounds: BoundsRect {
            left: 0,
            top: 126,
            right: 1080,
            bottom: 300,
            width: 1080,
            height: 174,
        },
        clickable: false,
        enabled: true,
        focusable: false,
        focused: false,
        scrollable: false,
        checkable: false,
        checked: false,
        selected: false,
        password: false,
        visible: true,
        source: None,
        source_candidates: vec![],
    };
    let matcher = NodeMatcher {
        resource_id_suffix: Some(":id/toolbar".into()),
        ..Default::default()
    };
    assert_eq!(find_node(&matcher, &[node]).unwrap().bounds.top, 126);
}

#[test]
fn accepts_compact_launch_tap_node_back_sequence() {
    let request: TraceRequest = serde_json::from_value(json!({
        "deviceId": "192.168.1.2:5555",
        "packageName": "com.elon.app",
        "settleMs": 500,
        "steps": [
            {"name":"home", "action":{"type":"LAUNCH"}},
            {"name":"chat", "action":{"type":"TAP_NODE", "text":"一龙AI"}},
            {"name":"home-return", "action":{"type":"BACK"}}
        ],
        "selectors": [{"label":"Toolbar", "resourceIdSuffix":":id/toolbar"}]
    }))
    .unwrap();
    validate_request(&request).unwrap();
    assert!(matches!(
        request.steps[1].action,
        TraceAction::TapNode { .. }
    ));
}
