//! Selector-to-stimulus routing for real final-connection Unmap boundaries.

use anyhow::anyhow;

use super::super::super::{
    a2b2_cases::UnmapSelector, lifecycle_faults::ManagedTestUnmapCompletionFault,
    ManagedSqliteMultiConnectionFixture, ManagedTestRouteOrdinal, ManagedTestShmFaultPlanBinding,
};
use crate::node_agent_managed_fs::{
    ManagedSqliteShmFailureClass as Class, ManagedSqliteShmFailurePhase as Phase,
    ManagedSqliteShmTestUnmapDeletePrestate as Prestate,
    ManagedSqliteShmTestUnmapNativeOperation as Native,
};

pub(super) fn raw_delete(selector: UnmapSelector) -> i32 {
    i32::from(super::outcome::is_delete(selector))
}

pub(super) fn install(
    fixture: &ManagedSqliteMultiConnectionFixture,
    binding: &ManagedTestShmFaultPlanBinding,
    route: ManagedTestRouteOrdinal,
    selector: UnmapSelector,
) -> anyhow::Result<()> {
    use UnmapSelector as S;

    match selector {
        S::FinalKeepViewUnmapBefore => before(binding, Phase::ViewUnmap),
        S::FinalKeepViewUnmapNativeUncertain => native(binding, Native::ViewUnmapOutcomeUncertain),
        S::FinalKeepViewUnmapAfterKnown => after_known(binding, Phase::ViewUnmap),
        S::FinalKeepViewUnmapAfterUncertain => after_uncertain(binding, Phase::ViewUnmap),
        S::FinalKeepMappingCloseBefore => before(binding, Phase::MappingClose),
        S::FinalKeepMappingCloseNativeUncertain => {
            native(binding, Native::MappingCloseOutcomeUncertain)
        }
        S::FinalKeepMappingCloseAfterKnown => after_known(binding, Phase::MappingClose),
        S::FinalKeepMappingCloseAfterUncertain => after_uncertain(binding, Phase::MappingClose),
        S::FinalKeepDmsReleaseBefore => before(binding, Phase::DmsSharedRelease),
        S::FinalKeepDmsReleaseNativeUncertain => {
            native(binding, Native::DmsSharedReleaseOutcomeUncertain)
        }
        S::FinalKeepDmsReleaseAfterKnown => after_known(binding, Phase::DmsSharedRelease),
        S::FinalKeepDmsReleaseAfterUncertain => after_uncertain(binding, Phase::DmsSharedRelease),
        S::FinalKeepFileCloseBefore => before(binding, Phase::FileClose),
        S::FinalKeepFileCloseNativeRetryable => native(binding, Native::FileCloseRetryable),
        S::FinalKeepFileCloseNativeUncertain => native(binding, Native::FileCloseOutcomeUncertain),
        S::FinalKeepFileCloseAfterKnown => after_known(binding, Phase::FileClose),
        S::FinalKeepFileCloseAfterUncertain => after_uncertain(binding, Phase::FileClose),
        S::FinalKeepDetachBefore | S::FinalDeleteDetachBefore => {
            before(binding, Phase::ConnectionDetach)
        }
        S::FinalKeepDetachAfterKnown | S::FinalDeleteDetachAfterKnown => {
            after_known(binding, Phase::ConnectionDetach)
        }
        S::FinalKeepDetachAfterUncertain | S::FinalDeleteDetachAfterUncertain => {
            after_uncertain(binding, Phase::ConnectionDetach)
        }
        S::FinalKeepCompletionNativeUncertain | S::FinalDeleteCompletionNativeUncertain => {
            completion(fixture, route)
        }
        S::FinalDeleteAuthMainIdentityMissing => prestate(binding, Prestate::MissingIdentity),
        S::FinalDeleteAuthMainOrGenerationMismatch => prestate(binding, Prestate::IdentityMismatch),
        S::FinalDeleteAuthLockStateUncertain => prestate(binding, Prestate::LockQueryUnavailable),
        S::FinalDeleteSiblingBefore => before(binding, Phase::ExactSiblingDelete),
        S::FinalDeleteSiblingNativeRetryable => {
            native(binding, Native::ExactSiblingDeleteRetryable)
        }
        S::FinalDeleteSiblingNativeUncertain => {
            native(binding, Native::ExactSiblingDeleteOutcomeUncertain)
        }
        S::FinalDeleteSiblingAfterKnown => after_known(binding, Phase::ExactSiblingDelete),
        S::FinalDeleteSiblingAfterUncertain => after_uncertain(binding, Phase::ExactSiblingDelete),
        S::FinalDeleteSuccessNotFound => {
            prestate(binding, Prestate::NotFound)?;
            after_known(binding, Phase::ExactSiblingDelete)
        }
        S::FinalKeepSuccessLiveNode
        | S::FinalKeepSuccessNodeAbsent
        | S::FinalDeleteAuthMainNotExclusive
        | S::FinalDeleteSuccessDeleted => Ok(()),
        _ => Err(anyhow!(
            "Unmap selector is outside final-connection stimulus authority"
        )),
    }
}

fn before(binding: &ManagedTestShmFaultPlanBinding, phase: Phase) -> anyhow::Result<()> {
    binding
        .install(&[(phase, 1)], &[])
        .map_err(anyhow::Error::msg)
}

fn after_known(binding: &ManagedTestShmFaultPlanBinding, phase: Phase) -> anyhow::Result<()> {
    binding
        .install(&[], &[(phase, 1, Class::MutatedButKnown)])
        .map_err(anyhow::Error::msg)
}

fn after_uncertain(binding: &ManagedTestShmFaultPlanBinding, phase: Phase) -> anyhow::Result<()> {
    binding
        .install(&[], &[(phase, 1, Class::OutcomeUncertainPoisoned)])
        .map_err(anyhow::Error::msg)
}

fn native(binding: &ManagedTestShmFaultPlanBinding, operation: Native) -> anyhow::Result<()> {
    binding
        .install_unmap_native_operation(operation)
        .map_err(anyhow::Error::msg)
}

fn prestate(binding: &ManagedTestShmFaultPlanBinding, value: Prestate) -> anyhow::Result<()> {
    binding
        .set_unmap_delete_prestate(value)
        .map_err(anyhow::Error::msg)
}

fn completion(
    fixture: &ManagedSqliteMultiConnectionFixture,
    route: ManagedTestRouteOrdinal,
) -> anyhow::Result<()> {
    let step =
        ManagedTestUnmapCompletionFault::native_uncertain(route).map_err(anyhow::Error::msg)?;
    fixture
        .install_lifecycle_fault_script(&[step])
        .map_err(anyhow::Error::msg)
}
