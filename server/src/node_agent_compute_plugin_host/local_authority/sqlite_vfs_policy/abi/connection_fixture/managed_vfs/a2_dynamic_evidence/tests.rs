use std::{fs, path::PathBuf};

use super::{
    capture::drain_capped_for_test,
    child::validate_payload_for_test,
    environment::{capture_root_binding, validate_git_sha, validate_sqlite_version},
    SanitizedChildReport,
};

const NONCE: &str = "0123456789abcdef0123456789abcdef";

#[test]
fn child_report_v2_extracts_one_bound_line_from_libtest_framing() {
    let root = test_root("parse");
    fs::create_dir_all(&root).expect("create report root");
    let actual = actual_payload(7);
    let line = SanitizedChildReport::encode_for_current_child(NONCE, &root, 7, &actual)
        .expect("encode bound child report");
    let stdout = format!("running 1 test\n{line}\ntest child ... ok\n");
    let report = SanitizedChildReport::parse_captured_stdout(stdout.as_bytes())
        .expect("parse bound child report");
    assert_eq!(report.actual_payload(), actual.as_str());
    fs::remove_dir_all(root).expect("remove report root");
}

#[test]
fn child_report_rejects_duplicate_or_unsafe_actual_fields() {
    let root = test_root("reject");
    fs::create_dir_all(&root).expect("create rejection root");
    let actual = actual_payload(7);
    let line = SanitizedChildReport::encode_for_current_child(NONCE, &root, 7, &actual)
        .expect("encode bound child report");
    let duplicate = format!("{line}\n{line}\n");
    assert_eq!(
        SanitizedChildReport::parse_captured_stdout(duplicate.as_bytes()).err(),
        Some("A2_DYNAMIC_CHILD_REPORT_DUPLICATE")
    );
    assert_eq!(
        validate_payload_for_test("a2b2rs1,success,0"),
        Err("A2_DYNAMIC_CHILD_ACTUAL_FIELDS_INVALID")
    );
    assert_eq!(
        validate_payload_for_test(&payload("a2b2rs1", "fence-before", 7)),
        Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
    );
    assert_eq!(
        validate_payload_for_test(&payload("a2b2br1", "outstanding-callback-gate", 7)),
        Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
    );
    assert!(validate_payload_for_test(&payload("a2b2br1", "fence-before", 7)).is_ok());
    let mut noncanonical = vec!["0".to_owned(); 81];
    noncanonical[0] = "00".to_owned();
    assert_eq!(
        validate_payload_for_test(&format!("a2b2br1,fence-before,{}", noncanonical.join(","))),
        Err("A2_DYNAMIC_CHILD_ACTUAL_FIELDS_INVALID")
    );
    noncanonical[0] = "18446744073709551616".to_owned();
    assert_eq!(
        validate_payload_for_test(&format!("a2b2br1,fence-before,{}", noncanonical.join(","))),
        Err("A2_DYNAMIC_CHILD_ACTUAL_FIELDS_INVALID")
    );
    assert_eq!(
        SanitizedChildReport::encode_for_current_child(NONCE, &root, 0, &actual).err(),
        Some("A2_DYNAMIC_REGISTRATION_ID_INVALID")
    );
    fs::remove_dir_all(root).expect("remove rejection root");
}

#[test]
fn root_and_registration_commitments_bind_real_inputs_without_exposing_them() {
    let root = test_root("bindings");
    fs::create_dir_all(&root).expect("create binding root");
    let actual = actual_payload(7);
    let first = encoded_report(&root, 7, &actual);
    let different_registration = encoded_report(&root, 8, &actual);
    let changed_actual = encoded_report(&root, 7, &changed_actual_payload());
    assert!(first.root_commitment == different_registration.root_commitment);
    assert!(first.registration_commitment != different_registration.registration_commitment);
    assert!(first.registration_commitment != changed_actual.registration_commitment);
    let recomputed =
        capture_root_binding(&root, std::process::id(), NONCE).expect("recompute root binding");
    assert!(first.root_commitment == recomputed.commitment);
    fs::remove_dir_all(root).expect("remove binding root");
}

#[test]
fn capped_capture_drains_but_never_retains_beyond_the_limit() {
    assert_eq!(drain_capped_for_test(b"1234", 4), Ok(b"1234".to_vec()));
    assert_eq!(
        drain_capped_for_test(b"12345", 4),
        Err("A2_DYNAMIC_CHILD_TEST_TOO_LARGE")
    );
}

#[test]
fn environment_scalar_validators_reject_ambiguous_build_identity() {
    assert_eq!(
        validate_git_sha("0123456789abcdef0123456789abcdef01234567"),
        Ok("0123456789abcdef0123456789abcdef01234567")
    );
    assert_eq!(
        validate_git_sha("0123456"),
        Err("A2_DYNAMIC_GIT_SHA_INVALID")
    );
    assert!(validate_sqlite_version("3.45.0", 3_045_000).is_ok());
    assert_eq!(
        validate_sqlite_version("3.45.0", 3_046_000),
        Err("A2_DYNAMIC_SQLITE_VERSION_MISMATCH")
    );
}

fn encoded_report(
    root: &std::path::Path,
    registration_id: u64,
    actual: &str,
) -> SanitizedChildReport {
    let line = SanitizedChildReport::encode_for_current_child(NONCE, root, registration_id, actual)
        .expect("encode child report");
    SanitizedChildReport::parse_captured_stdout(line.as_bytes()).expect("parse child report")
}

fn test_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "elon-a2-dynamic-evidence-{label}-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ))
}

fn actual_payload(registration_id: u64) -> String {
    payload("a2b2rs1", "success", registration_id)
}

fn payload(version: &str, selector: &str, registration_id: u64) -> String {
    let mut fields = vec!["0".to_owned(); 81];
    fields[12] = registration_id.to_string();
    format!("{version},{selector},{}", fields.join(","))
}

fn changed_actual_payload() -> String {
    let mut fields = vec!["0".to_owned(); 81];
    fields[12] = "7".to_owned();
    fields[20] = "1".to_owned();
    format!("a2b2rs1,success,{}", fields.join(","))
}
