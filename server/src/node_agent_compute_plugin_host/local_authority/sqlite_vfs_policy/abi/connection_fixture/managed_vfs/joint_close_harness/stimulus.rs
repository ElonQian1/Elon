//! Route-bound installation for exactly one real JointClose boundary.

use anyhow::anyhow;

use super::{
    super::{
        a2b2_cases::JointCloseSelector as S, ManagedTestCallbackFaultOperation,
        ManagedTestCallbackFaultStep, ManagedTestCallbackFaultTiming, ManagedTestJointCloseControl,
        ManagedTestLifecycleFaultPhase, ManagedTestLifecycleFaultStep,
        ManagedTestLifecycleFaultTiming,
    },
    outcome,
    prepare::JointCloseFixture,
    shm,
};
use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole,
    node_agent_managed_fs::ManagedSqliteMainCloseTestNativeRequest as Native,
};

pub(super) fn install(fixture: &JointCloseFixture, selector: S) -> anyhow::Result<()> {
    if outcome::observes_physical_actions(selector) && !outcome::is_shm(selector) {
        fixture
            .binding
            .begin_unmap_action_observation()
            .map_err(anyhow::Error::msg)?;
    }
    match selector {
        S::RawStateTakeRejected => fixture.arm_raw_state_take_rejection(),
        S::BeginConnectionCloseRejected => control(
            fixture,
            ManagedTestJointCloseControl::BeginConnectionCloseRejected,
        ),
        S::CallbackAdmissionRejected => control(
            fixture,
            ManagedTestJointCloseControl::CallbackAdmissionRejected,
        ),
        S::CallbackWrapperBefore => callback_wrapper(fixture),
        selector if outcome::is_shm(selector) => shm::install(&fixture.binding, selector),
        S::MainLockReleaseBefore => lifecycle(
            fixture,
            ManagedTestLifecycleFaultPhase::MainUnlock,
            ManagedTestLifecycleFaultTiming::BeforeCall,
        ),
        S::MainLockReleaseNativeUncertainShared => control(
            fixture,
            ManagedTestJointCloseControl::MainNative(Native::MainLockReleaseNativeUncertainShared),
        ),
        S::MainLockReleaseNativeUncertainReserved => control(
            fixture,
            ManagedTestJointCloseControl::MainNative(
                Native::MainLockReleaseNativeUncertainReserved,
            ),
        ),
        S::MainLockReleaseAfterKnown => lifecycle(
            fixture,
            ManagedTestLifecycleFaultPhase::MainUnlock,
            ManagedTestLifecycleFaultTiming::AfterSuccess,
        ),
        S::MainFileCloseBefore => lifecycle(
            fixture,
            ManagedTestLifecycleFaultPhase::MainFileClose,
            ManagedTestLifecycleFaultTiming::BeforeCall,
        ),
        S::MainFileCloseNativeRetryable => control(
            fixture,
            ManagedTestJointCloseControl::MainNative(Native::MainFileCloseNativeRetryable),
        ),
        S::MainFileCloseNativeUncertain => control(
            fixture,
            ManagedTestJointCloseControl::MainNative(Native::MainFileCloseNativeUncertain),
        ),
        S::MainFileCloseAfterKnown => lifecycle(
            fixture,
            ManagedTestLifecycleFaultPhase::MainFileClose,
            ManagedTestLifecycleFaultTiming::AfterSuccess,
        ),
        S::PhysicalSuccess => control(
            fixture,
            ManagedTestJointCloseControl::PhysicalSuccessHandoff,
        ),
        S::RegistryWalMainCloseBefore => lifecycle(
            fixture,
            ManagedTestLifecycleFaultPhase::RegistryWalMainClose,
            ManagedTestLifecycleFaultTiming::BeforeCall,
        ),
        S::RegistryWalMainCloseNativeUncertain => control(
            fixture,
            ManagedTestJointCloseControl::RegistryWalMainNativeUncertain,
        ),
        S::RegistryWalMainCloseAfterKnown => lifecycle(
            fixture,
            ManagedTestLifecycleFaultPhase::RegistryWalMainClose,
            ManagedTestLifecycleFaultTiming::AfterSuccess,
        ),
        _ => Err(anyhow!(
            "JointClose selector has no exact route-bound stimulus"
        )),
    }
}

fn callback_wrapper(fixture: &JointCloseFixture) -> anyhow::Result<()> {
    let step = ManagedTestCallbackFaultStep::new(
        fixture.route,
        ManagedSqliteLogicalFileRole::Main,
        ManagedTestCallbackFaultOperation::FileClose,
        1,
        ManagedTestCallbackFaultTiming::BeforeCall,
    )
    .map_err(anyhow::Error::msg)?;
    fixture
        .owner()
        .install_callback_fault_script(&[step])
        .map_err(anyhow::Error::msg)
}

fn lifecycle(
    fixture: &JointCloseFixture,
    phase: ManagedTestLifecycleFaultPhase,
    timing: ManagedTestLifecycleFaultTiming,
) -> anyhow::Result<()> {
    let step = ManagedTestLifecycleFaultStep::route(fixture.route, phase, 1, timing)
        .map_err(anyhow::Error::msg)?;
    fixture
        .owner()
        .install_lifecycle_fault_script(&[step])
        .map_err(anyhow::Error::msg)
}

fn control(
    fixture: &JointCloseFixture,
    selected: ManagedTestJointCloseControl,
) -> anyhow::Result<()> {
    fixture
        .lifecycle
        .install_joint_close_control(selected)
        .map_err(anyhow::Error::msg)
}
