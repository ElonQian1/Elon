mod fixture;

use fixture::CatalogFixture;

use super::{
    COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA, COMPUTE_PLUGIN_MANIFEST_CATALOG_SIGNATURE_DOMAIN,
};
use crate::node_agent_compute_plugin_host::{
    install_plan::COMPUTE_PLUGIN_INSTALL_PLAN_SIGNATURE_DOMAIN,
    plugin_manifest::COMPUTE_PLUGIN_MANIFEST_SIGNATURE_DOMAIN,
};

#[test]
fn accepts_distinct_publisher_and_control_signatures() {
    let fixture = CatalogFixture::new(
        false,
        COMPUTE_PLUGIN_MANIFEST_SIGNATURE_DOMAIN,
        COMPUTE_PLUGIN_MANIFEST_CATALOG_SIGNATURE_DOMAIN,
    );

    let validated = fixture.verify().unwrap();

    assert_eq!(
        validated.catalog().schema,
        COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA
    );
    assert_eq!(validated.catalog().entries.len(), 1);
    assert_eq!(
        validated.catalog().entries[0].signing_key_fingerprint,
        fixture.publisher_fingerprint()
    );
    assert_eq!(
        validated.control_signing_key_fingerprint(),
        fixture.control_fingerprint()
    );
    assert_ne!(
        validated.catalog().entries[0].signing_key_fingerprint,
        validated.control_signing_key_fingerprint()
    );
}

#[test]
fn rejects_manifest_signed_under_the_catalog_domain() {
    let fixture = CatalogFixture::new(
        false,
        COMPUTE_PLUGIN_MANIFEST_CATALOG_SIGNATURE_DOMAIN,
        COMPUTE_PLUGIN_MANIFEST_CATALOG_SIGNATURE_DOMAIN,
    );

    let error = fixture
        .verify()
        .err()
        .expect("manifest signature must fail");

    assert!(format!("{error:#}").contains("COMPUTE_PLUGIN_SIGNATURE_INVALID"));
}

#[test]
fn rejects_catalog_signed_under_the_install_plan_domain() {
    let fixture = CatalogFixture::new(
        false,
        COMPUTE_PLUGIN_MANIFEST_SIGNATURE_DOMAIN,
        COMPUTE_PLUGIN_INSTALL_PLAN_SIGNATURE_DOMAIN,
    );

    let error = fixture.verify().err().expect("catalog signature must fail");

    assert!(format!("{error:#}").contains("COMPUTE_PLUGIN_SIGNATURE_INVALID"));
}

#[test]
fn rejects_publisher_key_material_reused_by_control() {
    let fixture = CatalogFixture::new(
        true,
        COMPUTE_PLUGIN_MANIFEST_SIGNATURE_DOMAIN,
        COMPUTE_PLUGIN_MANIFEST_CATALOG_SIGNATURE_DOMAIN,
    );

    let error = fixture.verify().err().expect("role reuse must fail");

    assert!(format!("{error:#}").contains("COMPUTE_PLUGIN_MANIFEST_CATALOG_SIGNING_ROLE_REUSED"));
}

#[test]
fn rejects_manifest_target_outside_the_catalog_authority() {
    let fixture = CatalogFixture::new(
        false,
        COMPUTE_PLUGIN_MANIFEST_SIGNATURE_DOMAIN,
        COMPUTE_PLUGIN_MANIFEST_CATALOG_SIGNATURE_DOMAIN,
    );

    let error = fixture
        .verify_for_target("linux_x86_64")
        .err()
        .expect("target mismatch must fail");

    assert!(format!("{error:#}").contains("COMPUTE_PLUGIN_MANIFEST_CATALOG_TARGET_MISMATCH"));
}
