use std::{collections::BTreeMap, fs, path::Path};

use serde_json::json;

use super::writeback_receipt::{
    begin_writeback_receipt, complete_writeback_receipt, BeginWritebackReceiptRequest,
    CompleteWritebackReceiptRequest, PlatformReceiptUpdate,
};

#[test]
fn receipt_requires_both_platform_build_evidence_before_completion() {
    let root = git_fixture();
    let receipt = begin_writeback_receipt(BeginWritebackReceiptRequest {
        operation_id: "draft:7".into(),
        project_root: root.to_string_lossy().to_string(),
        draft_revision: 7,
        target_platforms: vec!["pwa".into(), "apk".into()],
    })
    .unwrap();

    fs::write(root.join("web.css"), "button{color:red}").unwrap();
    fs::write(
        root.join("app.xml"),
        "<TextView android:textColor=\"#f00\"/>",
    )
    .unwrap();
    let partial = complete_writeback_receipt(CompleteWritebackReceiptRequest {
        receipt_id: receipt.receipt_id.clone(),
        project_root: root.to_string_lossy().to_string(),
        platform_results: BTreeMap::from([
            (
                "pwa".into(),
                platform_update(
                    "BUILD_VERIFIED",
                    vec!["web.css"],
                    Some(json!({
                        "status":"BUILD_VERIFIED",
                        "runtimeReloaded":true,
                        "routeRevision":"draft-r7"
                    })),
                ),
            ),
            (
                "apk".into(),
                platform_update("SAVED", vec!["app.xml"], None),
            ),
        ]),
    })
    .unwrap();
    assert_eq!(partial.status, "IN_PROGRESS");
    assert!(!partial.complete);
    assert!(partial.platform_results["pwa"].evidence_complete);
    assert_eq!(partial.platform_results["apk"].status, "SAVED");

    let complete = complete_writeback_receipt(CompleteWritebackReceiptRequest {
        receipt_id: receipt.receipt_id,
        project_root: root.to_string_lossy().to_string(),
        platform_results: BTreeMap::from([(
            "apk".into(),
            platform_update(
                "BUILD_VERIFIED",
                vec!["app.xml"],
                Some(json!({
                    "status":"BUILD_VERIFIED",
                    "runtimeConnected":true,
                    "apkPath":"build/app.apk"
                })),
            ),
        )]),
    })
    .unwrap();
    assert_eq!(complete.status, "COMPLETE");
    assert!(complete.complete);
    assert!(complete.evidence_complete);
    assert_eq!(complete.changed_files, vec!["app.xml", "web.css"]);
    assert!(complete.source_hash.starts_with("sha256:"));
    assert_eq!(complete.source_hashes.len(), 2);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn receipt_downgrades_build_verified_without_runtime_evidence() {
    let root = git_fixture();
    let receipt = begin_writeback_receipt(BeginWritebackReceiptRequest {
        operation_id: "draft:missing".into(),
        project_root: root.to_string_lossy().to_string(),
        draft_revision: 1,
        target_platforms: vec!["pwa".into()],
    })
    .unwrap();
    fs::write(root.join("web.css"), "body{color:red}").unwrap();
    let result = complete_writeback_receipt(CompleteWritebackReceiptRequest {
        receipt_id: receipt.receipt_id,
        project_root: root.to_string_lossy().to_string(),
        platform_results: BTreeMap::from([(
            "pwa".into(),
            platform_update(
                "BUILD_VERIFIED",
                vec!["web.css"],
                Some(json!({"status":"BUILD_VERIFIED"})),
            ),
        )]),
    })
    .unwrap();
    assert_eq!(result.status, "EVIDENCE_MISSING");
    assert_eq!(result.platform_results["pwa"].status, "EVIDENCE_MISSING");
    assert!(!result.complete);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ai_saved_receipt_requires_revision_change_after_checkpoint() {
    let root = git_fixture();
    let receipt = begin_writeback_receipt(BeginWritebackReceiptRequest {
        operation_id: "draft:ai".into(),
        project_root: root.to_string_lossy().to_string(),
        draft_revision: 2,
        target_platforms: vec!["pwa".into()],
    })
    .unwrap();
    let unchanged = complete_writeback_receipt(CompleteWritebackReceiptRequest {
        receipt_id: receipt.receipt_id,
        project_root: root.to_string_lossy().to_string(),
        platform_results: BTreeMap::from([(
            "pwa".into(),
            PlatformReceiptUpdate {
                status: "SAVED".into(),
                method: "CODEX".into(),
                changed_files: vec!["web.css".into()],
                source_revisions: BTreeMap::new(),
                expected_source_revision_before: Some(receipt.source_revision.clone()),
                build_evidence: None,
                ai_task_id: Some("task-1".into()),
                error: None,
            },
        )]),
    });
    assert!(
        unchanged.is_err(),
        "unmodified file is not part of this operation"
    );
    fs::remove_dir_all(root).unwrap();
}

fn platform_update(
    status: &str,
    changed_files: Vec<&str>,
    build_evidence: Option<serde_json::Value>,
) -> PlatformReceiptUpdate {
    PlatformReceiptUpdate {
        status: status.into(),
        method: "DETERMINISTIC".into(),
        changed_files: changed_files.into_iter().map(str::to_string).collect(),
        source_revisions: BTreeMap::new(),
        expected_source_revision_before: None,
        build_evidence,
        ai_task_id: None,
        error: None,
    }
}

fn git_fixture() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "elon-writeback-receipt-{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    run_git(&root, &["init"]);
    run_git(&root, &["config", "user.email", "test@example.com"]);
    run_git(&root, &["config", "user.name", "Test"]);
    fs::write(root.join("web.css"), "button{color:black}").unwrap();
    fs::write(
        root.join("app.xml"),
        "<TextView android:textColor=\"#000\"/>",
    )
    .unwrap();
    run_git(&root, &["add", "."]);
    run_git(&root, &["commit", "-m", "init"]);
    root
}

fn run_git(root: &Path, args: &[&str]) {
    assert!(crate::git_command_error::git_command()
        .args(args)
        .current_dir(root)
        .status()
        .unwrap()
        .success());
}
