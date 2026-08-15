use anyhow::{bail, Result};

use super::super::*;

pub(crate) fn validate_task_protocol_conformance_profile(
    value: &ExternalPoolAdapterTaskProtocolConformanceProfile,
) -> Result<()> {
    if value != &task_protocol_conformance_profile_for_validation() {
        bail!("task-protocol conformance profile is not the exact server catalog entry")
    }
    Ok(())
}

pub(crate) fn validate_task_protocol_conformance_profile_envelope(
    value: &ExternalPoolAdapterTaskProtocolConformanceProfileEnvelope,
) -> Result<()> {
    validate_task_protocol_conformance_profile(&value.profile)?;
    super::support::digest(&value.profile_digest)?;
    if value.schema != TASK_PROTOCOL_CONFORMANCE_PROFILE_ENVELOPE_SCHEMA
        || value.canonicalization != TASK_PROTOCOL_CONFORMANCE_CANONICALIZATION
        || value.digest_algorithm != TASK_PROTOCOL_CONFORMANCE_DIGEST_ALGORITHM
        || task_protocol_conformance_profile_digest(&value.profile)? != value.profile_digest
    {
        bail!("task-protocol conformance profile envelope is not exact")
    }
    Ok(())
}

pub(crate) fn validate_task_protocol_conformance_fixture_catalog(
    value: &ExternalPoolAdapterTaskProtocolConformanceFixtureCatalog,
) -> Result<()> {
    if value != &task_protocol_conformance_fixture_catalog_for_validation() {
        bail!("task-protocol conformance fixture catalog is not exact")
    }
    Ok(())
}

pub(crate) fn validate_task_protocol_conformance_fixture_catalog_envelope(
    value: &ExternalPoolAdapterTaskProtocolConformanceFixtureCatalogEnvelope,
) -> Result<()> {
    validate_task_protocol_conformance_fixture_catalog(&value.catalog)?;
    super::support::digest(&value.catalog_digest)?;
    if value.schema != TASK_PROTOCOL_CONFORMANCE_FIXTURE_CATALOG_ENVELOPE_SCHEMA
        || value.canonicalization != TASK_PROTOCOL_CONFORMANCE_CANONICALIZATION
        || value.digest_algorithm != TASK_PROTOCOL_CONFORMANCE_DIGEST_ALGORITHM
        || task_protocol_conformance_fixture_catalog_digest(&value.catalog)? != value.catalog_digest
    {
        bail!("task-protocol conformance fixture catalog envelope is not exact")
    }
    Ok(())
}
