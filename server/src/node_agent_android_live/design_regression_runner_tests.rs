use std::{fs, io::Cursor, path::Path};

use image::{DynamicImage, ImageFormat, RgbaImage};
use serde_json::json;
use sha2::{Digest, Sha256};

use super::{
    broker::LiveUiBroker,
    design_regression_store::{
        self as store, DesignRegressionTask, RegressionEvidenceRef, RegressionThresholds,
    },
    design_tools,
};

fn fixture_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "elon_design_regression_runner_{name}_{}",
        uuid::Uuid::new_v4().simple()
    ))
}

fn encode_png(pixels: Vec<u8>) -> Vec<u8> {
    let image = RgbaImage::from_raw(2, 2, pixels).unwrap();
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}

fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn evidence(root: &Path, name: &str, bytes: &[u8]) -> RegressionEvidenceRef {
    fs::write(root.join(name), bytes).unwrap();
    RegressionEvidenceRef {
        path: name.to_string(),
        sha256: sha(bytes),
        width: None,
        height: None,
        node_count: None,
    }
}

fn tree(status: &str) -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({
        "schema":"elon.web.semantic-tree.v1",
        "nodes":[
            {"selector":"main","parentSelector":null,"tag":"main","role":"main",
                "label":format!("容器 {status}"),"interactive":false,"disabled":false,"style":{}},
            {"selector":"#status","parentSelector":"main","tag":"p","role":"status",
                "label":status,"interactive":false,"disabled":false,
                "style":{"color":"rgb(1, 2, 3)"}}
        ]
    }))
    .unwrap()
}

fn duplicate_selector_tree(second_label: &str) -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({
        "schema":"elon.web.semantic-tree.v1",
        "nodes":[
            {"selector":"main","parentSelector":null,"tag":"main","role":"main",
                "label":"容器","interactive":false,"disabled":false,"style":{}},
            {"selector":"main > button","parentSelector":"main","tag":"button","role":"button",
                "label":"第一个","interactive":true,"disabled":false,"style":{}},
            {"selector":"main > button","parentSelector":"main","tag":"button","role":"button",
                "label":second_label,"interactive":true,"disabled":false,"style":{}}
        ]
    }))
    .unwrap()
}

async fn fixture_session(root: &Path) -> std::sync::Arc<super::broker::LiveUiSession> {
    fs::write(root.join("package.json"), r#"{"scripts":{"dev":"vite"}}"#).unwrap();
    LiveUiBroker::new()
        .create_session(
            "design-regression-runner-test".to_string(),
            "ui.design.regression.runner.test".to_string(),
            Some(root.display().to_string()),
            38921,
        )
        .await
}

fn task(
    root: &Path,
    id_suffix: char,
    before_pixels: &[u8],
    after_pixels: &[u8],
    before_tree: &[u8],
    after_tree: &[u8],
    threshold: f64,
    changed_selectors: Vec<String>,
) -> DesignRegressionTask {
    let comparison_id = format!("compare_{}", id_suffix.to_string().repeat(32));
    DesignRegressionTask {
        schema_version: 1,
        comparison_id,
        revision: 1,
        baseline_id: format!("baseline_{}", id_suffix.to_string().repeat(32)),
        before_design_session_id: format!("design_{}", "1".repeat(32)),
        after_design_session_id: format!("design_{}", "2".repeat(32)),
        platform: "web".to_string(),
        route: "/".to_string(),
        before_pixels: evidence(root, "before.png", before_pixels),
        after_pixels: evidence(root, "after.png", after_pixels),
        before_ui_tree: evidence(root, "before.ui.json", before_tree),
        after_ui_tree: evidence(root, "after.ui.json", after_tree),
        thresholds: RegressionThresholds {
            max_pixel_diff_ratio: threshold,
            max_missing_selectors: 0,
            max_changed_selectors: 0,
            require_same_viewport: true,
            ignore_selectors: Vec::new(),
        },
        changed_selectors,
        status: "READY_TO_COMPARE".to_string(),
        result: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    }
}

#[tokio::test]
async fn local_comparator_generates_verified_artifacts_and_passes_allowed_change() {
    let root = fixture_root("pass");
    fs::create_dir_all(&root).unwrap();
    let root = root.canonicalize().unwrap();
    let session = fixture_session(&root).await;
    let pixels = encode_png(vec![0, 0, 0, 255].repeat(4));
    let regression_task = task(
        &root,
        'a',
        &pixels,
        &pixels,
        &tree("等待"),
        &tree("成功"),
        0.0,
        vec!["#status".to_string()],
    );
    let comparison_id = regression_task.comparison_id.clone();
    store::persist_task(&root, &regression_task).unwrap();

    let result = design_tools::call(
        &session,
        "ui_run_design_regression_comparison",
        json!({"comparisonId":comparison_id,"expectedRevision":1}),
    )
    .await
    .unwrap();

    assert_eq!(result["comparison"]["status"], "PASSED");
    assert_eq!(result["comparison"]["revision"], 2);
    assert_eq!(
        result["comparison"]["result"]["comparatorId"],
        "elon-node-local-regression-v1"
    );
    assert_eq!(result["comparison"]["result"]["pixelDiffRatio"], 0.0);
    assert_eq!(
        result["comparison"]["result"]["changedSelectors"][0],
        "#status"
    );
    assert_eq!(result["comparison"]["result"]["unexpectedChangedCount"], 0);
    for key in ["visualDiffArtifact", "semanticDiffArtifact"] {
        let path = result["comparison"]["result"][key]["path"]
            .as_str()
            .unwrap();
        assert!(!std::path::Path::new(path).is_absolute());
        assert!(root.join(path).is_file());
    }
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn local_comparator_fails_pixel_threshold_and_rejects_input_drift() {
    let root = fixture_root("fail");
    fs::create_dir_all(&root).unwrap();
    let root = root.canonicalize().unwrap();
    let session = fixture_session(&root).await;
    let before = encode_png(vec![0, 0, 0, 255].repeat(4));
    let mut after_pixels = vec![0, 0, 0, 255].repeat(4);
    after_pixels[0..4].copy_from_slice(&[255, 255, 255, 255]);
    let after = encode_png(after_pixels);
    let unchanged_tree = tree("相同");
    let failed_task = task(
        &root,
        'b',
        &before,
        &after,
        &unchanged_tree,
        &unchanged_tree,
        0.0,
        Vec::new(),
    );
    let comparison_id = failed_task.comparison_id.clone();
    store::persist_task(&root, &failed_task).unwrap();

    let failed = design_tools::call(
        &session,
        "ui_run_design_regression_comparison",
        json!({"comparisonId":comparison_id,"expectedRevision":1}),
    )
    .await
    .unwrap();
    assert_eq!(failed["comparison"]["status"], "FAILED");
    assert_eq!(failed["comparison"]["result"]["changedPixelCount"], 1);
    assert_eq!(failed["comparison"]["result"]["pixelDiffRatio"], 0.25);

    let drift_task = task(
        &root,
        'c',
        &before,
        &before,
        &unchanged_tree,
        &unchanged_tree,
        0.0,
        Vec::new(),
    );
    let drift_id = drift_task.comparison_id.clone();
    store::persist_task(&root, &drift_task).unwrap();
    fs::write(root.join("before.png"), b"tampered").unwrap();
    let error = design_tools::call(
        &session,
        "ui_run_design_regression_comparison",
        json!({"comparisonId":drift_id,"expectedRevision":1}),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("DESIGN_REGRESSION_INPUT_DRIFT"));
    assert_eq!(
        store::read_task(&root, &drift_id).unwrap().status,
        "READY_TO_COMPARE"
    );
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn local_comparator_compares_duplicate_selectors_as_deterministic_multisets() {
    let root = fixture_root("duplicate-selectors");
    fs::create_dir_all(&root).unwrap();
    let root = root.canonicalize().unwrap();
    let session = fixture_session(&root).await;
    let pixels = encode_png(vec![0, 0, 0, 255].repeat(4));
    let regression_task = task(
        &root,
        'd',
        &pixels,
        &pixels,
        &duplicate_selector_tree("第二个"),
        &duplicate_selector_tree("已修改"),
        0.0,
        vec!["main > button".to_string()],
    );
    let comparison_id = regression_task.comparison_id.clone();
    store::persist_task(&root, &regression_task).unwrap();

    let result = design_tools::call(
        &session,
        "ui_run_design_regression_comparison",
        json!({"comparisonId":comparison_id,"expectedRevision":1}),
    )
    .await
    .unwrap();

    assert_eq!(result["comparison"]["status"], "PASSED");
    assert_eq!(
        result["comparison"]["result"]["changedSelectors"],
        json!(["main > button"])
    );
    assert_eq!(result["comparison"]["result"]["unexpectedChangedCount"], 0);
    fs::remove_dir_all(root).unwrap();
}
