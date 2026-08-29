//! Sealed identity derived from the observed RegistryLifecycle stage boundary.

use super::super::{
    a2b2_cases::{
        RegistryLifecycleActualIdentity, RegistryLifecycleActualTarget,
        RegistryLifecycleFailureClass, RegistryLifecyclePhase, RegistryLifecycleSelector,
        RegistryLifecycleSqliteOutcome, RegistryLifecycleTiming,
    },
    ManagedTestShmTargetWitness,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ObservedRegistryLifecycleOutcome {
    CallbackCompletionBefore,
    CallbackCompletionNativeUncertain,
    CallbackCompletionAfterSuccessKnown,
    ConnectionObservationBefore,
    ConnectionObservationOutstandingSidecar,
    ConnectionObservationAfterSuccessKnown,
    RegistryRouteRemovalBefore,
    RegistryRouteRemovalOwnerNative,
    RegistryRouteRemovalPublishNative,
    RegistryRouteRemovalAfterSuccessKnown,
    LogicalRouteRemovalBefore,
    LogicalRouteRemovalClaimNative,
    LogicalRouteRemovalIndexNative,
    LogicalRouteRemovalAfterSuccessKnown,
    SuccessSharedNonFinal,
    SuccessFinal,
}

impl ObservedRegistryLifecycleOutcome {
    pub(super) const fn selector(self) -> RegistryLifecycleSelector {
        use ObservedRegistryLifecycleOutcome as O;
        match self {
            O::CallbackCompletionBefore => RegistryLifecycleSelector::CallbackCompletionBefore,
            O::CallbackCompletionNativeUncertain => {
                RegistryLifecycleSelector::CallbackCompletionNativeUncertain
            }
            O::CallbackCompletionAfterSuccessKnown => {
                RegistryLifecycleSelector::CallbackCompletionAfterSuccessKnown
            }
            O::ConnectionObservationBefore => {
                RegistryLifecycleSelector::ConnectionObservationBefore
            }
            O::ConnectionObservationOutstandingSidecar => {
                RegistryLifecycleSelector::ConnectionObservationOutstandingSidecar
            }
            O::ConnectionObservationAfterSuccessKnown => {
                RegistryLifecycleSelector::ConnectionObservationAfterSuccessKnown
            }
            O::RegistryRouteRemovalBefore => RegistryLifecycleSelector::RegistryRouteRemovalBefore,
            O::RegistryRouteRemovalOwnerNative => {
                RegistryLifecycleSelector::RegistryRouteRemovalOwnerNative
            }
            O::RegistryRouteRemovalPublishNative => {
                RegistryLifecycleSelector::RegistryRouteRemovalPublishNative
            }
            O::RegistryRouteRemovalAfterSuccessKnown => {
                RegistryLifecycleSelector::RegistryRouteRemovalAfterSuccessKnown
            }
            O::LogicalRouteRemovalBefore => RegistryLifecycleSelector::LogicalRouteRemovalBefore,
            O::LogicalRouteRemovalClaimNative => {
                RegistryLifecycleSelector::LogicalRouteRemovalClaimNative
            }
            O::LogicalRouteRemovalIndexNative => {
                RegistryLifecycleSelector::LogicalRouteRemovalIndexNative
            }
            O::LogicalRouteRemovalAfterSuccessKnown => {
                RegistryLifecycleSelector::LogicalRouteRemovalAfterSuccessKnown
            }
            O::SuccessSharedNonFinal => RegistryLifecycleSelector::SuccessSharedNonFinal,
            O::SuccessFinal => RegistryLifecycleSelector::SuccessFinal,
        }
    }

    pub(super) const fn is_success(self) -> bool {
        matches!(self, Self::SuccessSharedNonFinal | Self::SuccessFinal)
    }

    pub(super) const fn is_shared(self) -> bool {
        matches!(self, Self::SuccessSharedNonFinal)
    }

    pub(super) const fn is_logical_failure(self) -> bool {
        matches!(
            self,
            Self::LogicalRouteRemovalBefore
                | Self::LogicalRouteRemovalClaimNative
                | Self::LogicalRouteRemovalIndexNative
                | Self::LogicalRouteRemovalAfterSuccessKnown
        )
    }

    pub(super) const fn registry_route_removed(self) -> bool {
        matches!(
            self,
            Self::RegistryRouteRemovalPublishNative
                | Self::RegistryRouteRemovalAfterSuccessKnown
                | Self::LogicalRouteRemovalBefore
                | Self::LogicalRouteRemovalClaimNative
                | Self::LogicalRouteRemovalIndexNative
                | Self::LogicalRouteRemovalAfterSuccessKnown
                | Self::SuccessSharedNonFinal
                | Self::SuccessFinal
        )
    }

    pub(super) const fn logical_route_removed(self) -> bool {
        matches!(
            self,
            Self::LogicalRouteRemovalAfterSuccessKnown
                | Self::SuccessSharedNonFinal
                | Self::SuccessFinal
        )
    }

    pub(super) const fn fault_counts(self) -> (u8, u8) {
        match self {
            Self::CallbackCompletionNativeUncertain
            | Self::ConnectionObservationOutstandingSidecar
            | Self::RegistryRouteRemovalOwnerNative
            | Self::LogicalRouteRemovalIndexNative => (1, 0),
            Self::RegistryRouteRemovalPublishNative
            | Self::LogicalRouteRemovalClaimNative
            | Self::SuccessSharedNonFinal
            | Self::SuccessFinal => (0, 0),
            _ => (1, 1),
        }
    }

    pub(super) fn into_identity(
        self,
        target: ManagedTestShmTargetWitness,
        pre_shared_mask: u8,
        pre_exclusive_mask: u8,
    ) -> RegistryLifecycleActualIdentity {
        use ObservedRegistryLifecycleOutcome as O;
        let (variant, phase, timing, sqlite_outcome) = match self {
            O::CallbackCompletionBefore => (
                0,
                RegistryLifecyclePhase::CallbackCompletion,
                RegistryLifecycleTiming::BeforeCall,
                RegistryLifecycleSqliteOutcome::IoerrClose,
            ),
            O::CallbackCompletionNativeUncertain => (
                0,
                RegistryLifecyclePhase::CallbackCompletion,
                RegistryLifecycleTiming::NativeUncertain,
                RegistryLifecycleSqliteOutcome::IoerrClose,
            ),
            O::CallbackCompletionAfterSuccessKnown => (
                0,
                RegistryLifecyclePhase::CallbackCompletion,
                RegistryLifecycleTiming::AfterSuccessKnown,
                RegistryLifecycleSqliteOutcome::IoerrClose,
            ),
            O::ConnectionObservationBefore => (
                0,
                RegistryLifecyclePhase::ConnectionObservation,
                RegistryLifecycleTiming::BeforeCall,
                RegistryLifecycleSqliteOutcome::IoerrClose,
            ),
            O::ConnectionObservationOutstandingSidecar => (
                1,
                RegistryLifecyclePhase::ConnectionObservation,
                RegistryLifecycleTiming::Validation,
                RegistryLifecycleSqliteOutcome::IoerrClose,
            ),
            O::ConnectionObservationAfterSuccessKnown => (
                0,
                RegistryLifecyclePhase::ConnectionObservation,
                RegistryLifecycleTiming::AfterSuccessKnown,
                RegistryLifecycleSqliteOutcome::IoerrClose,
            ),
            O::RegistryRouteRemovalBefore => (
                0,
                RegistryLifecyclePhase::RegistryRouteRemoval,
                RegistryLifecycleTiming::BeforeCall,
                RegistryLifecycleSqliteOutcome::IoerrClose,
            ),
            O::RegistryRouteRemovalOwnerNative => (
                1,
                RegistryLifecyclePhase::RegistryRouteRemoval,
                RegistryLifecycleTiming::NativeUncertain,
                RegistryLifecycleSqliteOutcome::IoerrClose,
            ),
            O::RegistryRouteRemovalPublishNative => (
                2,
                RegistryLifecyclePhase::RegistryRouteRemoval,
                RegistryLifecycleTiming::NativeUncertain,
                RegistryLifecycleSqliteOutcome::IoerrClose,
            ),
            O::RegistryRouteRemovalAfterSuccessKnown => (
                0,
                RegistryLifecyclePhase::RegistryRouteRemoval,
                RegistryLifecycleTiming::AfterSuccessKnown,
                RegistryLifecycleSqliteOutcome::IoerrClose,
            ),
            O::LogicalRouteRemovalBefore => (
                0,
                RegistryLifecyclePhase::LogicalRouteRemoval,
                RegistryLifecycleTiming::BeforeCall,
                RegistryLifecycleSqliteOutcome::NotApplicable,
            ),
            O::LogicalRouteRemovalClaimNative => (
                1,
                RegistryLifecyclePhase::LogicalRouteRemoval,
                RegistryLifecycleTiming::NativeUncertain,
                RegistryLifecycleSqliteOutcome::NotApplicable,
            ),
            O::LogicalRouteRemovalIndexNative => (
                2,
                RegistryLifecyclePhase::LogicalRouteRemoval,
                RegistryLifecycleTiming::NativeUncertain,
                RegistryLifecycleSqliteOutcome::NotApplicable,
            ),
            O::LogicalRouteRemovalAfterSuccessKnown => (
                0,
                RegistryLifecyclePhase::LogicalRouteRemoval,
                RegistryLifecycleTiming::AfterSuccessKnown,
                RegistryLifecycleSqliteOutcome::NotApplicable,
            ),
            O::SuccessSharedNonFinal | O::SuccessFinal => (
                0,
                RegistryLifecyclePhase::Success,
                RegistryLifecycleTiming::Success,
                RegistryLifecycleSqliteOutcome::Ok,
            ),
        };
        RegistryLifecycleActualIdentity {
            path_is_registry_lifecycle: true,
            topology_is_shared_non_final: self.is_shared(),
            unmap_is_keep: true,
            node_is_live: true,
            variant,
            pre_shared_mask,
            pre_exclusive_mask,
            phase,
            cause_phase_is_none: true,
            timing,
            class: if self.is_success() {
                RegistryLifecycleFailureClass::None
            } else {
                RegistryLifecycleFailureClass::RegistryRejected
            },
            target: RegistryLifecycleActualTarget {
                scope_is_route_main: true,
                registration_id: target.registration_id(),
                route_ordinal: target.route_ordinal(),
                runtime_generation: target.runtime_generation(),
                shm_connection_id: target.shm_connection_id(),
                role_is_main: matches!(
                    target.role(),
                    crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole::Main
                ),
                callback_is_close: true,
                occurrence: 1,
            },
            sqlite_outcome,
        }
    }
}
