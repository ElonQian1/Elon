use anyhow::{bail, Result};
use serde::Serialize;

use super::canonical::domain_digest;

const CARRIER_POLICY_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-TASK-PRODUCTION-CARRIER-POLICY-V1";

pub(crate) const TASK_PRODUCTION_CARRIER_POLICY_DIGEST: &str =
    "0e2f1ee192d4701c09327a94a0a30de8fe9714c049231f8a89eeb0d4c896645b";

#[derive(Serialize)]
struct TaskProductionCarrierPolicy<'a> {
    schema: &'a str,
    startup: &'a str,
    protocol: &'a str,
    authority: &'a str,
    dispatch_gate: &'a str,
    effects: &'a str,
}

pub(crate) fn validate_task_production_carrier_policy_digest(value: &str) -> Result<()> {
    let policy = TaskProductionCarrierPolicy {
        schema: "compute_federation.external_pool_adapter_task_production_carrier_policy.v1",
        startup: "default_off",
        protocol: "eltp_v1",
        authority: "non_authoritative_carrier_only",
        dispatch_gate: "requires_v278_current_authority_reproof",
        effects: "none",
    };
    if value != TASK_PRODUCTION_CARRIER_POLICY_DIGEST
        || domain_digest(CARRIER_POLICY_DOMAIN, &policy)? != value
    {
        bail!("task production carrier policy digest is not the exact server policy")
    }
    Ok(())
}
