use anyhow::{bail, Result};

use crate::node_agent_compute_plugin_host::manifest_validation::is_sha256;

use super::resolution::{
    SealedWindowsLoaderNamespaceAuthority, SealedWindowsLoaderNamespacePrerequisite,
};

pub(super) fn validate_namespace_queries(
    prerequisite: &SealedWindowsLoaderNamespacePrerequisite,
    namespace: &SealedWindowsLoaderNamespaceAuthority,
    expected_content_lease_generation_set_digest: &str,
) -> Result<()> {
    let (session_identity, grant_generation, generation_domain) = prerequisite.session.binding();
    let (
        initial_attempt_session,
        initial_attempt_generation,
        initial_attempt_domain,
        initial_request,
        initial_nonce,
        initial_attempt_fence_set,
        initial_attempt_content_lease_set,
    ) = prerequisite.initial_query_attempt.binding();
    let (
        initial_receipt_session,
        initial_receipt_generation,
        initial_query_generation,
        initial_receipt_domain,
        initial_receipt_digest,
        initial_receipt_request,
        initial_receipt_nonce,
        initial_fence_set,
        initial_content_lease_set,
    ) = prerequisite.initial_query_receipt.binding();
    let (
        final_attempt_session,
        final_attempt_generation,
        final_attempt_domain,
        final_request,
        final_nonce,
        final_attempt_fence_set,
        final_attempt_content_lease_set,
    ) = namespace.final_query_attempt.binding();
    let (
        final_receipt_session,
        final_receipt_generation,
        final_query_generation,
        final_receipt_domain,
        final_receipt_digest,
        final_receipt_request,
        final_receipt_nonce,
        final_fence_set,
        final_content_lease_set,
    ) = namespace.final_query_receipt.binding();

    if !prerequisite
        .initial_query_attempt
        .matches_session(&prerequisite.session)
        || !prerequisite
            .initial_query_receipt
            .matches_session(&prerequisite.session)
        || !namespace
            .final_query_attempt
            .matches_session(&prerequisite.session)
        || !namespace
            .final_query_receipt
            .matches_session(&prerequisite.session)
        || !prerequisite
            .initial_query_receipt
            .authenticated_response_is_bound()
        || !namespace
            .final_query_receipt
            .authenticated_response_is_bound()
        || [
            initial_attempt_session,
            initial_receipt_session,
            final_attempt_session,
            final_receipt_session,
        ]
        .iter()
        .any(|value| *value != session_identity)
        || [
            initial_attempt_generation,
            initial_receipt_generation,
            final_attempt_generation,
            final_receipt_generation,
        ]
        .iter()
        .any(|value| *value != grant_generation)
        || [
            initial_attempt_domain,
            initial_receipt_domain,
            final_attempt_domain,
            final_receipt_domain,
        ]
        .iter()
        .any(|value| *value != generation_domain)
        || initial_request != initial_receipt_request
        || initial_nonce != initial_receipt_nonce
        || initial_attempt_fence_set != initial_fence_set
        || initial_attempt_content_lease_set != initial_content_lease_set
        || final_request != final_receipt_request
        || final_nonce != final_receipt_nonce
        || final_attempt_fence_set != final_fence_set
        || final_attempt_content_lease_set != final_content_lease_set
        || final_request == initial_request
        || final_nonce == initial_nonce
        || initial_fence_set != prerequisite.fence_generation_set_digest
        || final_fence_set != prerequisite.fence_generation_set_digest
        || initial_content_lease_set != expected_content_lease_generation_set_digest
        || final_content_lease_set != expected_content_lease_generation_set_digest
        || initial_query_generation < grant_generation
        || final_query_generation <= initial_query_generation
        || [
            initial_request,
            initial_nonce,
            initial_receipt_digest,
            final_request,
            final_nonce,
            final_receipt_digest,
        ]
        .iter()
        .any(|digest| !is_sha256(digest))
    {
        bail!("COMPUTE_PLUGIN_LOADER_NAMESPACE_QUERY_BINDING_CHANGED");
    }
    Ok(())
}
