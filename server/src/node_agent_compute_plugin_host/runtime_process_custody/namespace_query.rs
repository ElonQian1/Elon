use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::{bail, Error, Result};
use serde::Serialize;

use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256, signed_artifact_verification::jcs_sha256_hex,
};
use crate::node_agent_managed_fs::{
    ManagedLoaderAuthenticatedNegativeReceipt, ManagedLoaderNamespaceQueryAttemptCustody,
    ManagedLoaderNamespaceQueryReceipt,
};

use super::model::ValidatedWindowsRunnerProcessPreparation;

/// Exact name-fence/content-lease currentness observation made after fallible Job/attribute-list
/// setup and immediately before the path-based process open. The Windows resolution strings below
/// are sealed start-material echoes, not live KnownDLL/API-set/SxS observations; live OS resolution
/// currentness remains a separate resume blocker. A producer must bind the retained session,
/// grant/query generations, and final start material. Kernel grants are persistent-until-explicit-
/// release; the unavailable release/recovery authority is therefore also a resume blocker.
#[must_use = "pre-create namespace query must remain in process or failure custody"]
pub(super) struct WindowsRunnerPreCreateLoaderCurrentness {
    namespace_authority_digest: String,
    fence_generation_set_digest: String,
    content_lease_generation_set_digest: String,
    resolution_profile_digest: String,
    known_dll_os_build_identity_digest: String,
    known_dll_object_manager_identity_digest: String,
    known_dll_section_binding_set_digest: String,
    known_dll_section_generation_digest: String,
    api_set_os_build_identity_digest: String,
    api_set_schema_identity_digest: String,
    api_set_contract_host_binding_set_digest: String,
    activation_context_identity_digest: String,
    side_by_side_manifest_set_digest: String,
    side_by_side_assembly_binding_set_digest: String,
    system_component_image_set_digest: String,
    process_machine_context_digest: String,
    query_attempt: ManagedLoaderNamespaceQueryAttemptCustody,
    query_receipt: ManagedLoaderNamespaceQueryReceipt,
    observed_at: Instant,
    base_start_material_digest: String,
    start_material_digest: String,
    release_policy: WindowsRunnerLoaderFenceReleasePolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowsRunnerLoaderFenceReleasePolicy {
    ExplicitAuthorizedReleaseRequiredButUnavailable,
}

#[derive(Serialize)]
struct WindowsRunnerPreCreateLoaderStartBinding<'a> {
    schema: &'static str,
    base_start_material_digest: &'a str,
    namespace_authority_digest: &'a str,
    fence_generation_set_digest: &'a str,
    content_lease_generation_set_digest: &'a str,
    resolution_profile_digest: &'a str,
    known_dll_os_build_identity_digest: &'a str,
    known_dll_object_manager_identity_digest: &'a str,
    known_dll_section_binding_set_digest: &'a str,
    known_dll_section_generation_digest: &'a str,
    api_set_os_build_identity_digest: &'a str,
    api_set_schema_identity_digest: &'a str,
    api_set_contract_host_binding_set_digest: &'a str,
    activation_context_identity_digest: &'a str,
    side_by_side_manifest_set_digest: &'a str,
    side_by_side_assembly_binding_set_digest: &'a str,
    system_component_image_set_digest: &'a str,
    process_machine_context_digest: &'a str,
    driver_session_identity_digest: &'a str,
    grant_generation: u64,
    query_generation: u64,
    generation_domain_digest: &'a str,
    query_receipt_digest: &'a str,
    query_request_digest: &'a str,
    query_nonce_digest: &'a str,
    release_policy: &'static str,
}

pub(super) struct LoaderCurrentWindowsRunnerProcessPreparation<'root> {
    pub(super) preparation: ValidatedWindowsRunnerProcessPreparation<'root>,
    pub(super) currentness: WindowsRunnerPreCreateLoaderCurrentness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowsRunnerPreCreateLoaderCurrentnessFailureClass {
    DefinitiveRejected,
    OutcomeUncertain,
}

/// A failed/broken query consumes the preparation into a non-reusable quarantine. No API returns
/// the still-nested loader successor or launch-security owner for another CreateProcess attempt.
pub(super) struct WindowsRunnerPreCreateLoaderCurrentnessUnusableCustody<'root> {
    _state: WindowsRunnerPreCreateLoaderCurrentnessUnusableState<'root>,
}

enum WindowsRunnerPreCreateLoaderCurrentnessUnusableState<'root> {
    DefinitiveRejected {
        preparation: ValidatedWindowsRunnerProcessPreparation<'root>,
        query_attempt: ManagedLoaderNamespaceQueryAttemptCustody,
        returned_positive: Option<ManagedLoaderNamespaceQueryReceipt>,
        authenticated_negative: ManagedLoaderAuthenticatedNegativeReceipt,
    },
    OutcomeUncertain {
        preparation: ValidatedWindowsRunnerProcessPreparation<'root>,
        query_attempt: ManagedLoaderNamespaceQueryAttemptCustody,
        returned_positive: Option<ManagedLoaderNamespaceQueryReceipt>,
        authenticated_negative: Option<ManagedLoaderAuthenticatedNegativeReceipt>,
    },
    ReturnedCurrent(LoaderCurrentWindowsRunnerProcessPreparation<'root>),
}

pub(super) struct WindowsRunnerPreCreateLoaderCurrentnessFailure<'root> {
    class: WindowsRunnerPreCreateLoaderCurrentnessFailureClass,
    error: Error,
    custody: WindowsRunnerPreCreateLoaderCurrentnessUnusableCustody<'root>,
}

/// No implementation exists in this slice. A future backend must consume preparation, perform a
/// live kernel query, and return either a namespace-current owner or non-reusable custody. A point
/// snapshot is insufficient unless the retained kernel grant remains enforced across disconnect.
pub(super) trait WindowsRunnerPreCreateLoaderCurrentnessBackend {
    fn query_current_and_seal<'root>(
        self,
        preparation: ValidatedWindowsRunnerProcessPreparation<'root>,
    ) -> std::result::Result<
        LoaderCurrentWindowsRunnerProcessPreparation<'root>,
        WindowsRunnerPreCreateLoaderCurrentnessFailure<'root>,
    >;
}

impl WindowsRunnerPreCreateLoaderCurrentness {
    pub(super) fn start_material_digest(&self) -> &str {
        &self.start_material_digest
    }
}

impl<'root> LoaderCurrentWindowsRunnerProcessPreparation<'root> {
    fn seal(
        preparation: ValidatedWindowsRunnerProcessPreparation<'root>,
        currentness: WindowsRunnerPreCreateLoaderCurrentness,
    ) -> std::result::Result<Self, WindowsRunnerPreCreateLoaderCurrentnessFailure<'root>> {
        let sealed = Self {
            preparation,
            currentness,
        };
        match sealed.validate_binding() {
            Ok(()) => Ok(sealed),
            Err(error) => Err(sealed.reject_invalid_binding(error)),
        }
    }

    pub(super) fn validate_binding(&self) -> Result<()> {
        let image = self.preparation.loader_locked.image();
        let currentness = &self.currentness;
        let (session_digest, grant_generation, generation_domain_digest) =
            image.namespace_session_binding();
        let (
            receipt_session_digest,
            receipt_grant_generation,
            query_generation,
            receipt_generation_domain_digest,
            query_receipt_digest,
            receipt_request_digest,
            receipt_nonce_digest,
            receipt_fence_generation_set_digest,
            receipt_content_lease_generation_set_digest,
        ) = currentness.query_receipt.binding();
        if !currentness.query_receipt.authenticated_response_is_bound() {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRECREATE_QUERY_RESPONSE_INVALID");
        }
        let (
            attempt_session_digest,
            attempt_grant_generation,
            attempt_generation_domain_digest,
            attempt_request_digest,
            attempt_nonce_digest,
            attempt_fence_generation_set_digest,
            attempt_content_lease_generation_set_digest,
        ) = currentness.query_attempt.binding();
        if currentness.namespace_authority_digest
            != image.startup_import_namespace_authority_digest()
            || currentness.fence_generation_set_digest
                != image.namespace_fence_generation_set_digest()
            || currentness.content_lease_generation_set_digest
                != image.immutable_content_lease_set_digest()
            || currentness.resolution_profile_digest
                != image.startup_import_resolution_profile_digest()
            || currentness.known_dll_os_build_identity_digest
                != image.known_dll_os_build_identity_digest()
            || currentness.known_dll_object_manager_identity_digest
                != image.known_dll_object_manager_identity_digest()
            || currentness.known_dll_section_binding_set_digest
                != image.known_dll_section_binding_set_digest()
            || currentness.known_dll_section_generation_digest
                != image.known_dll_section_generation_digest()
            || currentness.api_set_os_build_identity_digest
                != image.api_set_os_build_identity_digest()
            || currentness.api_set_schema_identity_digest != image.api_set_schema_identity_digest()
            || currentness.api_set_contract_host_binding_set_digest
                != image.api_set_contract_host_binding_set_digest()
            || currentness.activation_context_identity_digest
                != image.activation_context_identity_digest()
            || currentness.side_by_side_manifest_set_digest
                != image.side_by_side_manifest_set_digest()
            || currentness.side_by_side_assembly_binding_set_digest
                != image.side_by_side_assembly_binding_set_digest()
            || currentness.system_component_image_set_digest
                != image.system_component_image_set_digest()
            || currentness.process_machine_context_digest
                != image.process_machine_context_digest()
            || !image.namespace_attempt_matches_session(&currentness.query_attempt)
            || !image.namespace_receipt_matches_session(&currentness.query_receipt)
            || attempt_session_digest != session_digest
            || receipt_session_digest != session_digest
            || attempt_grant_generation != grant_generation
            || receipt_grant_generation != grant_generation
            || attempt_generation_domain_digest != generation_domain_digest
            || receipt_generation_domain_digest != generation_domain_digest
            || attempt_request_digest != receipt_request_digest
            || attempt_nonce_digest != receipt_nonce_digest
            || attempt_fence_generation_set_digest != receipt_fence_generation_set_digest
            || attempt_content_lease_generation_set_digest
                != receipt_content_lease_generation_set_digest
            || receipt_fence_generation_set_digest
                != image.namespace_fence_generation_set_digest()
            || receipt_content_lease_generation_set_digest
                != image.immutable_content_lease_set_digest()
            || query_generation < grant_generation
            || query_generation <= image.final_namespace_query_generation()
            || receipt_request_digest == image.final_namespace_query_request_digest()
            || receipt_nonce_digest == image.final_namespace_query_nonce_digest()
            || currentness.base_start_material_digest
                != self.preparation.policy.start_material_digest
            || currentness.release_policy
                != WindowsRunnerLoaderFenceReleasePolicy::ExplicitAuthorizedReleaseRequiredButUnavailable
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRECREATE_LOADER_CURRENTNESS_CHANGED");
        }
        for digest in [
            receipt_session_digest,
            attempt_session_digest,
            generation_domain_digest,
            attempt_generation_domain_digest,
            query_receipt_digest,
            attempt_request_digest,
            attempt_nonce_digest,
            &currentness.start_material_digest,
        ] {
            if !is_sha256(digest) {
                bail!("COMPUTE_PLUGIN_WINDOWS_PRECREATE_LOADER_DIGEST_INVALID");
            }
        }
        let material = WindowsRunnerPreCreateLoaderStartBinding {
            schema: "elon.compute_plugin.windows_runner_precreate_loader_currentness.v1",
            base_start_material_digest: &currentness.base_start_material_digest,
            namespace_authority_digest: &currentness.namespace_authority_digest,
            fence_generation_set_digest: &currentness.fence_generation_set_digest,
            content_lease_generation_set_digest: &currentness.content_lease_generation_set_digest,
            resolution_profile_digest: &currentness.resolution_profile_digest,
            known_dll_os_build_identity_digest: &currentness.known_dll_os_build_identity_digest,
            known_dll_object_manager_identity_digest: &currentness
                .known_dll_object_manager_identity_digest,
            known_dll_section_binding_set_digest: &currentness.known_dll_section_binding_set_digest,
            known_dll_section_generation_digest: &currentness.known_dll_section_generation_digest,
            api_set_os_build_identity_digest: &currentness.api_set_os_build_identity_digest,
            api_set_schema_identity_digest: &currentness.api_set_schema_identity_digest,
            api_set_contract_host_binding_set_digest: &currentness
                .api_set_contract_host_binding_set_digest,
            activation_context_identity_digest: &currentness.activation_context_identity_digest,
            side_by_side_manifest_set_digest: &currentness.side_by_side_manifest_set_digest,
            side_by_side_assembly_binding_set_digest: &currentness
                .side_by_side_assembly_binding_set_digest,
            system_component_image_set_digest: &currentness.system_component_image_set_digest,
            process_machine_context_digest: &currentness.process_machine_context_digest,
            driver_session_identity_digest: receipt_session_digest,
            grant_generation,
            query_generation,
            generation_domain_digest,
            query_receipt_digest,
            query_request_digest: receipt_request_digest,
            query_nonce_digest: attempt_nonce_digest,
            release_policy: "explicit_authorized_release_required_but_unavailable",
        };
        if jcs_sha256_hex(&material)? != currentness.start_material_digest {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRECREATE_START_BINDING_CHANGED");
        }
        Ok(())
    }

    pub(super) fn reject_invalid_binding(
        self,
        error: Error,
    ) -> WindowsRunnerPreCreateLoaderCurrentnessFailure<'root> {
        WindowsRunnerPreCreateLoaderCurrentnessFailure {
            class: WindowsRunnerPreCreateLoaderCurrentnessFailureClass::OutcomeUncertain,
            error,
            custody: WindowsRunnerPreCreateLoaderCurrentnessUnusableCustody {
                _state: WindowsRunnerPreCreateLoaderCurrentnessUnusableState::ReturnedCurrent(self),
            },
        }
    }
}

impl<'root> WindowsRunnerPreCreateLoaderCurrentnessFailure<'root> {
    pub(super) fn definitive_rejected(
        error: Error,
        preparation: ValidatedWindowsRunnerProcessPreparation<'root>,
        query_attempt: ManagedLoaderNamespaceQueryAttemptCustody,
        returned_positive: Option<ManagedLoaderNamespaceQueryReceipt>,
        authenticated_negative: ManagedLoaderAuthenticatedNegativeReceipt,
    ) -> Self {
        let image = preparation.loader_locked.image();
        let (
            attempt_session,
            attempt_grant_generation,
            attempt_generation_domain,
            request_digest,
            query_nonce_digest,
            attempt_fence_set,
            attempt_content_lease_set,
        ) = query_attempt.binding();
        let (session, grant_generation, generation_domain) = image.namespace_session_binding();
        let negative_is_authenticated = returned_positive.is_none()
            && image.namespace_attempt_matches_session(&query_attempt)
            && attempt_session == session
            && attempt_grant_generation == grant_generation
            && attempt_generation_domain == generation_domain
            && attempt_fence_set == image.namespace_fence_generation_set_digest()
            && attempt_content_lease_set == image.immutable_content_lease_set_digest()
            && image.namespace_negative_matches_query(
                &authenticated_negative,
                request_digest,
                query_nonce_digest,
            );
        if negative_is_authenticated {
            Self {
                class: WindowsRunnerPreCreateLoaderCurrentnessFailureClass::DefinitiveRejected,
                error,
                custody: WindowsRunnerPreCreateLoaderCurrentnessUnusableCustody {
                    _state:
                        WindowsRunnerPreCreateLoaderCurrentnessUnusableState::DefinitiveRejected {
                            preparation,
                            query_attempt,
                            returned_positive,
                            authenticated_negative,
                        },
                },
            }
        } else {
            Self {
                class: WindowsRunnerPreCreateLoaderCurrentnessFailureClass::OutcomeUncertain,
                error,
                custody: WindowsRunnerPreCreateLoaderCurrentnessUnusableCustody {
                    _state:
                        WindowsRunnerPreCreateLoaderCurrentnessUnusableState::OutcomeUncertain {
                            preparation,
                            query_attempt,
                            returned_positive,
                            authenticated_negative: Some(authenticated_negative),
                        },
                },
            }
        }
    }

    pub(super) fn outcome_uncertain(
        error: Error,
        preparation: ValidatedWindowsRunnerProcessPreparation<'root>,
        query_attempt: ManagedLoaderNamespaceQueryAttemptCustody,
        returned_positive: Option<ManagedLoaderNamespaceQueryReceipt>,
    ) -> Self {
        Self {
            class: WindowsRunnerPreCreateLoaderCurrentnessFailureClass::OutcomeUncertain,
            error,
            custody: WindowsRunnerPreCreateLoaderCurrentnessUnusableCustody {
                _state: WindowsRunnerPreCreateLoaderCurrentnessUnusableState::OutcomeUncertain {
                    preparation,
                    query_attempt,
                    returned_positive,
                    authenticated_negative: None,
                },
            },
        }
    }

    pub(super) fn into_parts(
        self,
    ) -> (
        WindowsRunnerPreCreateLoaderCurrentnessFailureClass,
        Error,
        WindowsRunnerPreCreateLoaderCurrentnessUnusableCustody<'root>,
    ) {
        (self.class, self.error, self.custody)
    }
}

impl fmt::Debug for WindowsRunnerPreCreateLoaderCurrentness {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowsRunnerPreCreateLoaderCurrentness")
            .field("namespace_authority_digest", &"<redacted>")
            .field("fence_generation_set_digest", &"<redacted>")
            .field("query_receipt", &self.query_receipt)
            .field("observed_at", &self.observed_at)
            .field("start_material_digest", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for WindowsRunnerPreCreateLoaderCurrentnessFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowsRunnerPreCreateLoaderCurrentnessFailure")
            .field("class", &self.class)
            .field("error", &self.error)
            .field("custody", &"<preparation-quarantined>")
            .finish()
    }
}

impl fmt::Display for WindowsRunnerPreCreateLoaderCurrentnessFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl StdError for WindowsRunnerPreCreateLoaderCurrentnessFailure<'_> {}
