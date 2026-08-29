//! Sealed Barrier outcome identity derived from mutually exclusive event sequences.

use super::super::{
    a2b2_cases::{
        BarrierActualIdentity, BarrierActualTarget, BarrierFailureClass, BarrierPhase,
        BarrierSelector, BarrierTiming,
    },
    ManagedTestShmTargetWitness,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ObservedBarrierOutcome {
    AdmissionRejected,
    WrapperBefore,
    FenceBefore,
    FenceAfter,
    CompletionBefore,
    CompletionNativeUncertain,
    CompletionAfterSuccessKnown,
    Success,
}

impl ObservedBarrierOutcome {
    pub(super) fn selector(self) -> BarrierSelector {
        match self {
            Self::AdmissionRejected => BarrierSelector::AdmissionRejected,
            Self::WrapperBefore => BarrierSelector::WrapperBefore,
            Self::FenceBefore => BarrierSelector::FenceBefore,
            Self::FenceAfter => BarrierSelector::FenceAfter,
            Self::CompletionBefore => BarrierSelector::CompletionBefore,
            Self::CompletionNativeUncertain => BarrierSelector::CompletionNativeUncertain,
            Self::CompletionAfterSuccessKnown => BarrierSelector::CompletionAfterSuccessKnown,
            Self::Success => BarrierSelector::Success,
        }
    }

    pub(super) fn into_identity(
        self,
        target: ManagedTestShmTargetWitness,
        pre_shared_mask: u8,
        pre_exclusive_mask: u8,
    ) -> BarrierActualIdentity {
        let (variant, phase, timing, class) = match self {
            Self::AdmissionRejected => (
                0,
                BarrierPhase::CallbackAdmission,
                BarrierTiming::BeforeCall,
                BarrierFailureClass::RegistryRejected,
            ),
            Self::WrapperBefore => (
                1,
                BarrierPhase::BarrierFence,
                BarrierTiming::BeforeCall,
                BarrierFailureClass::IoBeforeMutation,
            ),
            Self::FenceBefore => (
                0,
                BarrierPhase::BarrierFence,
                BarrierTiming::BeforeCall,
                BarrierFailureClass::IoBeforeMutation,
            ),
            Self::FenceAfter => (
                0,
                BarrierPhase::BarrierFence,
                BarrierTiming::AfterSuccessUncertain,
                BarrierFailureClass::OutcomeUncertainPoisoned,
            ),
            Self::CompletionBefore => (
                0,
                BarrierPhase::CallbackCompletion,
                BarrierTiming::BeforeCall,
                BarrierFailureClass::RegistryRejected,
            ),
            Self::CompletionNativeUncertain => (
                0,
                BarrierPhase::CallbackCompletion,
                BarrierTiming::NativeUncertain,
                BarrierFailureClass::RegistryRejected,
            ),
            Self::CompletionAfterSuccessKnown => (
                0,
                BarrierPhase::CallbackCompletion,
                BarrierTiming::AfterSuccessKnown,
                BarrierFailureClass::RegistryRejected,
            ),
            Self::Success => (
                0,
                BarrierPhase::Success,
                BarrierTiming::Success,
                BarrierFailureClass::None,
            ),
        };
        BarrierActualIdentity {
            path_is_barrier: true,
            topology_is_shared_non_final: true,
            unmap_is_not_applicable: true,
            node_is_live: true,
            variant,
            pre_shared_mask,
            pre_exclusive_mask,
            phase,
            cause_phase_is_none: true,
            timing,
            class,
            target: BarrierActualTarget {
                scope_is_route_main: true,
                registration_id: target.registration_id(),
                route_ordinal: target.route_ordinal(),
                runtime_generation: target.runtime_generation(),
                shm_connection_id: target.shm_connection_id(),
                role_is_main: matches!(
                    target.role(),
                    crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole::Main
                ),
                callback_is_shm: true,
                occurrence: 1,
            },
            sqlite_outcome_is_void_no_result_code: true,
        }
    }

    pub(super) fn is_success(self) -> bool {
        self == Self::Success
    }

    pub(super) fn action_succeeded(self) -> bool {
        matches!(
            self,
            Self::FenceAfter
                | Self::CompletionBefore
                | Self::CompletionNativeUncertain
                | Self::CompletionAfterSuccessKnown
                | Self::Success
        )
    }

    pub(super) fn callback_began(self) -> bool {
        !matches!(self, Self::AdmissionRejected | Self::WrapperBefore)
    }

    pub(super) fn completion_attempted(self) -> bool {
        matches!(
            self,
            Self::CompletionNativeUncertain | Self::CompletionAfterSuccessKnown | Self::Success
        )
    }

    pub(super) fn completion_succeeded(self) -> bool {
        matches!(self, Self::CompletionAfterSuccessKnown | Self::Success)
    }
}
