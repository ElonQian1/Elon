use anyhow::Result;

use super::{canonical::domain_digest, validation::support, *};

const LANE_SUBJECT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-PRODUCTION-LANE-SUBJECT-V1";

/// Derives only the non-authoritative in-memory lane partition from frozen Provider roots.
/// This type deliberately has no executor/string conversion.
pub(crate) fn derive_external_pool_adapter_task_production_lane_subject(
    subject: ExternalPoolAdapterTaskProductionLaneSubjectInput,
) -> Result<ExternalPoolAdapterTaskProductionLaneSubject> {
    support::identifier(&subject.provider_id)?;
    support::identifier(&subject.provider_owner_account_id)?;
    support::identifier(&subject.provider_binding_id)?;
    support::digest(&subject.provider_binding_digest)?;
    support::identifier(&subject.registry_release_id)?;
    support::digest(&subject.registry_release_digest)?;
    support::identifier(&subject.route_adapter_projection_id)?;
    support::digest(&subject.logical_adapter_binding_digest)?;
    support::digest(&subject.logical_projection_compatibility_digest)?;
    let lane_subject_digest = domain_digest(LANE_SUBJECT_DOMAIN, &subject)?;
    Ok(ExternalPoolAdapterTaskProductionLaneSubject {
        subject,
        lane_subject_digest,
    })
}
