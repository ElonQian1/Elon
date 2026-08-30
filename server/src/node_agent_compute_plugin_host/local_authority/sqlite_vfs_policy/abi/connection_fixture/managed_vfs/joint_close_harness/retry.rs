//! Proof that the saved second xClose call performs zero additional physical work.

use anyhow::anyhow;

use super::{
    super::{
        a2b2_cases::JointCloseSelector as S, ManagedTestCallbackFaultObservation,
        ManagedTestJointCloseControlSnapshot, ManagedTestLifecycleFaultObservation,
        ManagedTestRegistryLifecycleTraceSnapshot,
    },
    invoke::{self, FirstClose},
    outcome,
    prepare::JointCloseFixture,
};
use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryTerminalCustodyTestSnapshot,
    node_agent_managed_fs::{ManagedSqliteShmTestTargetSnapshot, ManagedSqliteShmTestUnmapReceipt},
};

#[derive(Clone, PartialEq, Eq)]
struct RetryBaseline {
    physical: ManagedSqliteShmTestTargetSnapshot,
    receipt: Option<ManagedSqliteShmTestUnmapReceipt>,
    callbacks: Vec<ManagedTestCallbackFaultObservation>,
    lifecycle: Vec<ManagedTestLifecycleFaultObservation>,
    callback_pending: usize,
    lifecycle_pending: usize,
    generic_pending: usize,
    trace: ManagedTestRegistryLifecycleTraceSnapshot,
    custody: ManagedSqliteRegistryTerminalCustodyTestSnapshot,
    control: Option<ManagedTestJointCloseControlSnapshot>,
    registry_native_claims: Option<usize>,
    callback_admission_claims: Option<usize>,
    begin_connection_close_claims: Option<usize>,
    runtime: Vec<
        crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryUnmapRuntimeEvent,
    >,
}

pub(super) fn invoke_and_validate(
    fixture: &mut JointCloseFixture,
    selector: S,
    first: FirstClose,
) -> anyhow::Result<i32> {
    let before = capture(fixture, selector)?;
    let code = invoke::second(fixture, selector, first)?;
    let after = capture(fixture, selector)?;
    if after != before {
        return Err(anyhow!(
            "JointClose saved-callback retry changed physical, typed, trace, or custody evidence"
        ));
    }
    let finished = fixture
        .owner()
        .finish_unmap_runtime_observation(super::prepare::SELECTED)?;
    if finished != after.runtime {
        return Err(anyhow!(
            "JointClose outer runtime receipt changed while its observation window closed"
        ));
    }
    Ok(code)
}

fn capture(fixture: &JointCloseFixture, selector: S) -> anyhow::Result<RetryBaseline> {
    let physical = fixture
        .binding
        .observer()
        .map_err(anyhow::Error::msg)?
        .snapshot()
        .map_err(|failure| anyhow!("observe JointClose retry physical state: {failure:?}"))?;
    let receipt = if outcome::observes_physical_actions(selector) {
        Some(
            fixture
                .binding
                .observe_unmap_test_receipt()
                .map_err(anyhow::Error::msg)?,
        )
    } else {
        None
    };
    let control = if has_control(selector) {
        Some(
            fixture
                .lifecycle
                .joint_close_control_snapshot()
                .map_err(anyhow::Error::msg)?,
        )
    } else {
        None
    };
    Ok(RetryBaseline {
        physical,
        receipt,
        callbacks: fixture
            .owner()
            .callback_fault_observations()
            .map_err(anyhow::Error::msg)?,
        lifecycle: fixture
            .owner()
            .lifecycle_fault_observations()
            .map_err(anyhow::Error::msg)?,
        callback_pending: fixture
            .owner()
            .pending_callback_fault_count()
            .map_err(anyhow::Error::msg)?,
        lifecycle_pending: fixture
            .owner()
            .pending_lifecycle_fault_count()
            .map_err(anyhow::Error::msg)?,
        generic_pending: fixture
            .binding
            .pending_count()
            .map_err(anyhow::Error::msg)?,
        trace: fixture.route_observer.trace()?,
        custody: fixture.route_observer.terminal_custody()?,
        control,
        registry_native_claims: if selector == S::RegistryWalMainCloseNativeUncertain {
            Some(
                fixture
                    .route_observer
                    .registry_wal_main_native_uncertain_claim_count()?,
            )
        } else {
            None
        },
        callback_admission_claims: if selector == S::CallbackAdmissionRejected {
            Some(
                fixture
                    .route_observer
                    .close_callback_admission_claim_count()?,
            )
        } else {
            None
        },
        begin_connection_close_claims: if selector == S::BeginConnectionCloseRejected {
            Some(
                fixture
                    .route_observer
                    .begin_connection_close_claim_count()?,
            )
        } else {
            None
        },
        runtime: fixture
            .owner()
            .unmap_runtime_trace(super::prepare::SELECTED)?,
    })
}

fn has_control(selector: S) -> bool {
    matches!(
        selector,
        S::BeginConnectionCloseRejected
            | S::CallbackAdmissionRejected
            | S::MainLockReleaseNativeUncertainShared
            | S::MainLockReleaseNativeUncertainReserved
            | S::MainFileCloseNativeRetryable
            | S::MainFileCloseNativeUncertain
            | S::PhysicalSuccess
            | S::RegistryWalMainCloseNativeUncertain
    )
}
