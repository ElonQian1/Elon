use anyhow::Result;

use super::*;
use crate::node_agent_compute_plugin_host::{
    install_plan::COMPUTE_PLUGIN_INSTALL_PLAN_SIGNATURE_DOMAIN,
    keyring::{KEY_PURPOSE_CONTROL_INSTALL_PLAN, KEY_PURPOSE_PUBLISHER_MANIFEST},
    plugin_manifest::COMPUTE_PLUGIN_MANIFEST_SIGNATURE_DOMAIN,
};

mod fixtures;

use fixtures::valid_contracts;

#[test]
fn valid_shapes_and_exact_cross_binding_are_accepted_as_non_authorizing_data() {
    let (plan, manifest) = valid_contracts();

    validate_signed_privileged_component_manifest_shape(&manifest).unwrap();
    validate_signed_privileged_component_install_plan_shape(&plan).unwrap();
    validate_install_plan_manifest_binding(&plan, &manifest).unwrap();
}

#[test]
fn privileged_key_purposes_and_domains_are_isolated_from_each_other_and_plugins() {
    assert_ne!(
        PRIVILEGED_COMPONENT_RELEASE_KEY_PURPOSE,
        PRIVILEGED_COMPONENT_INSTALL_PLAN_KEY_PURPOSE
    );
    assert_ne!(
        PRIVILEGED_COMPONENT_RELEASE_KEY_PURPOSE,
        KEY_PURPOSE_PUBLISHER_MANIFEST
    );
    assert_ne!(
        PRIVILEGED_COMPONENT_INSTALL_PLAN_KEY_PURPOSE,
        KEY_PURPOSE_CONTROL_INSTALL_PLAN
    );
    assert_ne!(
        PRIVILEGED_COMPONENT_MANIFEST_SIGNATURE_DOMAIN,
        COMPUTE_PLUGIN_MANIFEST_SIGNATURE_DOMAIN
    );
    assert_ne!(
        PRIVILEGED_COMPONENT_INSTALL_PLAN_SIGNATURE_DOMAIN,
        COMPUTE_PLUGIN_INSTALL_PLAN_SIGNATURE_DOMAIN
    );

    let (mut plan, mut manifest) = valid_contracts();
    manifest.signature.key_purpose = PRIVILEGED_COMPONENT_INSTALL_PLAN_KEY_PURPOSE.to_string();
    assert_error(
        validate_signed_privileged_component_manifest_shape(&manifest),
        "PRIVILEGED_COMPONENT_SIGNATURE_METADATA_INVALID",
    );
    plan.signature.key_purpose = PRIVILEGED_COMPONENT_RELEASE_KEY_PURPOSE.to_string();
    assert_error(
        validate_signed_privileged_component_install_plan_shape(&plan),
        "PRIVILEGED_COMPONENT_SIGNATURE_METADATA_INVALID",
    );
}

#[test]
fn manifest_and_plan_reject_signing_key_id_reuse_even_with_distinct_purposes() {
    let (mut plan, manifest) = valid_contracts();
    plan.signature.signing_key_id = manifest.signature.signing_key_id.clone();

    assert_error(
        validate_install_plan_manifest_binding(&plan, &manifest),
        "PRIVILEGED_COMPONENT_SIGNING_KEY_ID_REUSE",
    );
}

#[test]
fn cross_binding_rejects_individually_well_shaped_plan_tampering() {
    let (plan, manifest) = valid_contracts();

    let mut changed = plan.clone();
    changed.plan.target_manifest_digest = "6".repeat(64);
    assert_binding_mismatch(&changed, &manifest);

    let mut changed = plan.clone();
    changed.plan.target_package_digest = "7".repeat(64);
    assert_binding_mismatch(&changed, &manifest);

    let mut changed = plan.clone();
    changed.plan.target_release_identity = format!("2.3.5+{}", "8".repeat(40));
    assert_binding_mismatch(&changed, &manifest);

    let mut changed = plan.clone();
    changed.plan.target_rollback_generation += 1;
    assert_binding_mismatch(&changed, &manifest);

    let mut changed = plan;
    changed.plan.node_version = "3.0.0".to_string();
    changed.plan.node_release_identity = format!("3.0.0+{}", "9".repeat(40));
    assert_binding_mismatch(&changed, &manifest);
}

#[test]
fn installation_gate_stays_fail_closed_before_altitude_and_trust_exist() {
    let (plan, manifest) = valid_contracts();

    assert_error(
        enforce_current_privileged_component_installation_gate(&plan, &manifest),
        "PRIVILEGED_COMPONENT_MINIFILTER_ALTITUDE_UNASSIGNED",
    );
}

fn assert_binding_mismatch(
    plan: &SignedPrivilegedComponentInstallPlan,
    manifest: &SignedPrivilegedComponentManifest,
) {
    validate_signed_privileged_component_install_plan_shape(plan).unwrap();
    assert_error(
        validate_install_plan_manifest_binding(plan, manifest),
        "PRIVILEGED_COMPONENT_PLAN_MANIFEST_BINDING_MISMATCH",
    );
}

fn assert_error(result: Result<()>, expected: &str) {
    assert_eq!(result.unwrap_err().to_string(), expected);
}
