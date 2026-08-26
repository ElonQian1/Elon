//! System-image content-lease failure classification with exact linear outcome custody.

use super::*;

impl<'root> WindowsRunnerContentLeaseAcquisitionUnusableCustody<'root> {
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn reject_system_image_acquisition(
        policy_current_namespace: PolicyCurrentPreFinalWindowsLoaderNamespaceGrantSet<'root>,
        acquired_leases: Vec<WindowsLoaderAcquiredImmutableContentLeaseCustody>,
        active_attempt: ManagedLoaderSystemImageContentLeaseAcquisitionAttemptCustody,
        authenticated_negative: ManagedLoaderSystemImageContentLeaseAuthenticatedNegativeReceipt,
        pending: Vec<WindowsRunnerPendingContentLeaseRef>,
    ) -> Self {
        let resolution_request_ordinal = active_attempt.binding().0;
        let class = if authenticated_negative.matches_attempt(&active_attempt) {
            WindowsRunnerContentLeaseAcquisitionFailureClass::DefinitiveRejected
        } else {
            WindowsRunnerContentLeaseAcquisitionFailureClass::OutcomeUncertain
        };
        Self {
            class,
            _policy_current_namespace: policy_current_namespace,
            _acquired_leases: acquired_leases,
            _active:
                WindowsRunnerActiveContentLeaseAcquisitionCustody::ResolvedFilesystemSystemImage {
                    resolution_request_ordinal,
                    attempt: active_attempt,
                    authenticated_negative: Some(authenticated_negative),
                },
            _pending: pending,
        }
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn system_image_outcome_uncertain(
        policy_current_namespace: PolicyCurrentPreFinalWindowsLoaderNamespaceGrantSet<'root>,
        acquired_leases: Vec<WindowsLoaderAcquiredImmutableContentLeaseCustody>,
        active_attempt: ManagedLoaderSystemImageContentLeaseAcquisitionAttemptCustody,
        pending: Vec<WindowsRunnerPendingContentLeaseRef>,
    ) -> Self {
        let resolution_request_ordinal = active_attempt.binding().0;
        Self {
            class: WindowsRunnerContentLeaseAcquisitionFailureClass::OutcomeUncertain,
            _policy_current_namespace: policy_current_namespace,
            _acquired_leases: acquired_leases,
            _active:
                WindowsRunnerActiveContentLeaseAcquisitionCustody::ResolvedFilesystemSystemImage {
                    resolution_request_ordinal,
                    attempt: active_attempt,
                    authenticated_negative: None,
                },
            _pending: pending,
        }
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn system_image_positive_outcome_uncertain(
        policy_current_namespace: PolicyCurrentPreFinalWindowsLoaderNamespaceGrantSet<'root>,
        acquired_leases: Vec<WindowsLoaderAcquiredImmutableContentLeaseCustody>,
        outcome: ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody,
        pending: Vec<WindowsRunnerPendingContentLeaseRef>,
    ) -> Self {
        let resolution_request_ordinal = outcome.binding().0;
        Self {
            class: WindowsRunnerContentLeaseAcquisitionFailureClass::OutcomeUncertain,
            _policy_current_namespace: policy_current_namespace,
            _acquired_leases: acquired_leases,
            _active:
                WindowsRunnerActiveContentLeaseAcquisitionCustody::ResolvedFilesystemSystemImagePositiveOutcome {
                    resolution_request_ordinal,
                    outcome,
                },
            _pending: pending,
        }
    }
}
