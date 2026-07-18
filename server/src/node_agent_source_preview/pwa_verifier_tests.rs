use super::pwa_verifier::{
    verify_resources_for_test, verify_sources_for_test, PwaSourceRevision, VerifyPwaSourceRequest,
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

struct TestProject(PathBuf);

impl TestProject {
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture() -> (TestProject, VerifyPwaSourceRequest) {
    let directory = TestProject(
        std::env::temp_dir().join(format!("elon-pwa-verifier-{}", uuid::Uuid::new_v4())),
    );
    fs::create_dir_all(directory.path().join("web/src")).expect("source directory");
    fs::write(
        directory.path().join("web/package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .expect("package json");
    let source = ".pay { height: 48px; border-radius: 12px; }";
    fs::write(directory.path().join("web/src/pay.css"), source).expect("css source");
    let revision = hex::encode(Sha256::digest(source.as_bytes()));
    let request = VerifyPwaSourceRequest {
        project_root: directory.path().display().to_string(),
        changed_files: vec![PwaSourceRevision {
            source_file: "web/src/pay.css".to_string(),
            source_revision: revision,
        }],
        expected_values: vec!["48px".to_string(), "12px".to_string()],
    };
    (directory, request)
}

#[test]
fn accepts_exact_changed_file_revisions_without_repository_scan() {
    let (_directory, request) = fixture();
    verify_sources_for_test(&request).expect("matching source hash");
}

#[test]
fn rejects_source_saved_receipt_after_file_changes() {
    let (directory, request) = fixture();
    fs::write(
        directory.path().join("web/src/pay.css"),
        ".pay { height: 49px; }",
    )
    .expect("change source");
    assert!(verify_sources_for_test(&request)
        .expect_err("stale receipt must fail")
        .to_string()
        .contains("sourceRevision 已变化"));
}

#[test]
fn resource_verification_requires_every_expected_value() {
    let (directory, _request) = fixture();
    let module = directory.path().join("web");
    fs::create_dir_all(module.join("dist/assets")).expect("dist directory");
    fs::write(
        module.join("dist/assets/app.css"),
        ".pay{height:48px;border-radius:12px}",
    )
    .expect("built css");
    let (_, count) = verify_resources_for_test(
        directory.path(),
        &module,
        vec!["dist".to_string()],
        &["48px".to_string(), "12px".to_string()],
    )
    .expect("resource values");
    assert_eq!(count, 2);
    assert!(verify_resources_for_test(
        directory.path(),
        &module,
        vec!["dist".to_string()],
        &["99px".to_string()],
    )
    .expect_err("missing value must fail")
    .to_string()
    .contains("缺少 1 个目标样式值"));
}
