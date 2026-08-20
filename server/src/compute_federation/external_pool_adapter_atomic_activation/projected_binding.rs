use anyhow::{bail, Result};

use crate::compute_federation::{
    attempt_gateway::{
        canonical_adapter_binding_json_and_digest, ComputeAttemptAdapterBinding,
        COMPUTE_ATTEMPT_ADAPTER_BINDING_SCHEMA, COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER,
    },
    provider::{ComputeProvider, PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_ACTIVE},
};

use super::ExternalPoolProjectedV211AdapterBinding;

pub(crate) fn derive_external_pool_projected_v211_adapter_binding(
    target: &ComputeProvider,
    route_adapter_projection_id: &str,
) -> Result<(
    ComputeAttemptAdapterBinding,
    ExternalPoolProjectedV211AdapterBinding,
)> {
    let adapter = target
        .adapter
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("V277 target Provider lacks projected Adapter"))?;
    if target.provider_kind != PROVIDER_KIND_EXTERNAL_POOL
        || target.status != PROVIDER_STATUS_ACTIVE
        || route_adapter_projection_id.trim() != route_adapter_projection_id
        || route_adapter_projection_id.is_empty()
        || adapter.adapter_id != route_adapter_projection_id
    {
        bail!("V277 projected v211 binding target is not exact active projection")
    }
    let binding = ComputeAttemptAdapterBinding {
        schema: COMPUTE_ATTEMPT_ADAPTER_BINDING_SCHEMA.into(),
        provider_id: target.provider_id.clone(),
        provider_kind: PROVIDER_KIND_EXTERNAL_POOL.into(),
        route_kind: COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER.into(),
        endpoint_id: None,
        endpoint_transport: None,
        adapter_id: route_adapter_projection_id.into(),
        adapter_version: adapter.adapter_version.clone(),
        config_revision: adapter.config_revision,
        config_digest: adapter.config_digest.clone(),
    };
    let (projected_v211_adapter_binding_json, projected_v211_adapter_binding_digest) =
        canonical_adapter_binding_json_and_digest(&binding)?;
    Ok((
        binding,
        ExternalPoolProjectedV211AdapterBinding {
            projected_v211_adapter_binding_json,
            projected_v211_adapter_binding_digest,
        },
    ))
}
