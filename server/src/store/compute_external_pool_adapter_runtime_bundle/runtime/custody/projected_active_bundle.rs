//! Purpose-specific commitment over a renewed-route active runtime bundle.

use anyhow::Result;

use super::{
    support::{constant_time_equal, is_lower_hex_sha256, update_field, update_u64},
    ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
};
use crate::{
    compute_federation::external_pool_adapter_provider_runtime_readiness::PROVIDER_RUNTIME_READINESS_BUNDLE_IDENTITY_COMMITMENT_DOMAIN,
    store::compute_external_pool_adapter_runtime_bundle::CurrentExternalPoolAdapterProjectedActiveRuntimeBundleAuthority,
};

impl ExternalPoolAdapterProviderRuntimeReadinessProcessCustody {
    pub(in crate::store) fn projected_active_runtime_bundle_identity_commitment(
        &self,
        bundle: &CurrentExternalPoolAdapterProjectedActiveRuntimeBundleAuthority<'_, '_>,
    ) -> Result<String> {
        let roots = bundle.roots();
        let carrier = bundle.carrier();
        let profile = carrier.profile();
        let credential = carrier.credential().receipt();
        let route = carrier.renewed_route();
        let runtime = carrier.runtime_compatibility().verification();
        self.with_commitment(
            PROVIDER_RUNTIME_READINESS_BUNDLE_IDENTITY_COMMITMENT_DOMAIN,
            |mac| {
                update_u64(mac, roots.bundle_generation());
                update_u64(mac, roots.config_size_bytes());
                update_field(mac, roots.config_sha256());
                update_u64(mac, roots.credential_size_bytes());
                update_field(mac, roots.credential_sha256());
                update_field(mac, profile.profile_id.as_bytes());
                update_field(mac, profile.profile_digest.as_bytes());
                update_field(mac, route.provider_binding_id().as_bytes());
                update_field(mac, route.provider_binding_digest().as_bytes());
                update_field(mac, credential.reattestation_receipt_id.as_bytes());
                update_field(mac, credential.reattestation_receipt_digest.as_bytes());
                update_field(mac, runtime.verification_receipt_id.as_bytes());
                update_field(mac, runtime.verification_receipt_digest.as_bytes());
                update_field(mac, route.receipt().route_renewal_receipt_digest.as_bytes());
            },
        )
    }

    pub(in crate::store) fn attests_projected_active_runtime_bundle_identity_commitment(
        &self,
        bundle: &CurrentExternalPoolAdapterProjectedActiveRuntimeBundleAuthority<'_, '_>,
        expected: &str,
    ) -> Result<bool> {
        if !is_lower_hex_sha256(expected) {
            return Ok(false);
        }
        Ok(constant_time_equal(
            &self.projected_active_runtime_bundle_identity_commitment(bundle)?,
            expected,
        ))
    }
}
