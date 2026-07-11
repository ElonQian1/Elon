use image::{DynamicImage, Rgba, RgbaImage};

use super::verification_gate::{
    evaluate_verification_gates, VerificationGateInput, VerificationGateState,
};
use super::visual_diff::compare_dynamic_images;

fn image(color: [u8; 4]) -> DynamicImage {
    DynamicImage::ImageRgba8(RgbaImage::from_pixel(40, 20, Rgba(color)))
}

#[test]
fn both_gates_must_pass_for_design_fit_build_verified() {
    let source = compare_dynamic_images(
        &image([10, 20, 30, 255]),
        &image([10, 20, 30, 255]),
        None,
        None,
    )
    .unwrap();
    let target = source.clone();
    let result = evaluate_verification_gates(VerificationGateInput::new(
        Some(&source),
        Some(&target),
        true,
    ));
    assert_eq!(result.status, "BUILD_VERIFIED");
    assert!(result.verified);
    assert_eq!(result.source_parity, VerificationGateState::Passed);
    assert_eq!(result.target_fidelity, VerificationGateState::Passed);
}

#[test]
fn target_failure_cannot_be_hidden_by_source_parity() {
    let source = compare_dynamic_images(
        &image([10, 20, 30, 255]),
        &image([10, 20, 30, 255]),
        None,
        None,
    )
    .unwrap();
    let target = compare_dynamic_images(
        &image([0, 0, 0, 255]),
        &image([255, 255, 255, 255]),
        None,
        None,
    )
    .unwrap();
    let result = evaluate_verification_gates(VerificationGateInput::new(
        Some(&source),
        Some(&target),
        true,
    ));
    assert_eq!(result.status, "TARGET_MISMATCH");
    assert!(!result.verified);
}

#[test]
fn missing_target_pair_is_not_reported_as_verified() {
    let source = compare_dynamic_images(
        &image([10, 20, 30, 255]),
        &image([10, 20, 30, 255]),
        None,
        None,
    )
    .unwrap();
    let result = evaluate_verification_gates(VerificationGateInput::new(Some(&source), None, true));
    assert_eq!(result.status, "TARGET_NOT_CONFIGURED");
    assert_eq!(result.target_fidelity, VerificationGateState::NotConfigured);
}

#[test]
fn source_failure_has_priority_over_target_result() {
    let source = compare_dynamic_images(
        &image([0, 0, 0, 255]),
        &image([255, 255, 255, 255]),
        None,
        None,
    )
    .unwrap();
    let target = compare_dynamic_images(
        &image([10, 20, 30, 255]),
        &image([10, 20, 30, 255]),
        None,
        None,
    )
    .unwrap();
    let result = evaluate_verification_gates(VerificationGateInput::new(
        Some(&source),
        Some(&target),
        true,
    ));
    assert_eq!(result.status, "SOURCE_MISMATCH");
    assert!(!result.verified);
}

#[test]
fn source_only_workflow_remains_backward_compatible() {
    let source = compare_dynamic_images(
        &image([10, 20, 30, 255]),
        &image([10, 20, 30, 255]),
        None,
        None,
    )
    .unwrap();
    let result =
        evaluate_verification_gates(VerificationGateInput::new(Some(&source), None, false));
    assert_eq!(result.status, "BUILD_VERIFIED");
    assert_eq!(result.target_fidelity, VerificationGateState::NotRequired);
}
