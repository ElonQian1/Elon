//! Selector-to-stimulus routing; no expected observation is constructed here.

use anyhow::anyhow;

use super::super::{
    a2b2_cases::UnmapSelector, lifecycle_faults::ManagedTestUnmapCompletionFault,
    ManagedSqliteMultiConnectionFixture, ManagedTestCallbackFaultOperation,
    ManagedTestCallbackFaultStep, ManagedTestCallbackFaultTiming, ManagedTestRouteOrdinal,
};
use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole,
    node_agent_managed_fs::{ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase},
};

use super::prepare::SELECTED;

pub(super) fn supports_shared(selector: UnmapSelector) -> bool {
    matches!(
        selector,
        UnmapSelector::SharedDeleteRequestValidation
            | UnmapSelector::SharedKeepCallbackAdmission
            | UnmapSelector::SharedKeepCallbackWrapperBefore
            | UnmapSelector::SharedKeepHeldSharedLock
            | UnmapSelector::SharedKeepHeldExclusiveLock
            | UnmapSelector::SharedKeepDetachBefore
            | UnmapSelector::SharedKeepDetachAfterKnown
            | UnmapSelector::SharedKeepDetachAfterUncertain
            | UnmapSelector::SharedKeepCompletionNativeUncertain
            | UnmapSelector::SharedKeepSuccess
            | UnmapSelector::SharedDeleteSuccess
    )
}

pub(super) fn raw_delete(selector: UnmapSelector) -> anyhow::Result<i32> {
    match selector {
        UnmapSelector::SharedDeleteRequestValidation => Ok(2),
        UnmapSelector::SharedDeleteSuccess => Ok(1),
        selector if supports_shared(selector) => Ok(0),
        _ => Err(anyhow!(
            "Unmap selector is outside SharedNonFinal authority"
        )),
    }
}

pub(super) fn install(
    fixture: &ManagedSqliteMultiConnectionFixture,
    selector: UnmapSelector,
    route: ManagedTestRouteOrdinal,
) -> anyhow::Result<()> {
    use UnmapSelector as S;
    match selector {
        S::SharedDeleteRequestValidation | S::SharedKeepSuccess | S::SharedDeleteSuccess => Ok(()),
        S::SharedKeepCallbackAdmission => fixture.quarantine_unmap_admission(SELECTED),
        S::SharedKeepCallbackWrapperBefore => {
            let step = ManagedTestCallbackFaultStep::new(
                route,
                ManagedSqliteLogicalFileRole::Main,
                ManagedTestCallbackFaultOperation::ShmUnmap,
                1,
                ManagedTestCallbackFaultTiming::BeforeCall,
            )
            .map_err(anyhow::Error::msg)?;
            fixture
                .install_callback_fault_script(&[step])
                .map_err(anyhow::Error::msg)
        }
        S::SharedKeepHeldSharedLock | S::SharedKeepHeldExclusiveLock => {
            fixture.acquire_unmap_shm_lock(SELECTED, selector == S::SharedKeepHeldExclusiveLock)
        }
        S::SharedKeepDetachBefore => fixture
            .route(SELECTED)?
            .install_shm_fault_script(&[(ManagedSqliteShmFailurePhase::ConnectionDetach, 1)], &[])
            .map_err(anyhow::Error::msg),
        S::SharedKeepDetachAfterKnown | S::SharedKeepDetachAfterUncertain => {
            let class = if selector == S::SharedKeepDetachAfterKnown {
                ManagedSqliteShmFailureClass::MutatedButKnown
            } else {
                ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned
            };
            fixture
                .route(SELECTED)?
                .install_shm_fault_script(
                    &[],
                    &[(ManagedSqliteShmFailurePhase::ConnectionDetach, 1, class)],
                )
                .map_err(anyhow::Error::msg)
        }
        S::SharedKeepCompletionNativeUncertain => {
            let step = ManagedTestUnmapCompletionFault::native_uncertain(route)
                .map_err(anyhow::Error::msg)?;
            fixture
                .install_lifecycle_fault_script(&[step])
                .map_err(anyhow::Error::msg)
        }
        _ => Err(anyhow!(
            "Unmap selector is outside SharedNonFinal authority"
        )),
    }
}
