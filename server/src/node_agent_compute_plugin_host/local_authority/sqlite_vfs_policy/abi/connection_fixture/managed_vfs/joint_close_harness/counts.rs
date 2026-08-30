//! Count projection from already sealed raw, action, control, and stage receipts.

use anyhow::{anyhow, Context};

use super::{
    super::a2b2_cases::{
        JointCloseActualCounts, JointClosePhase, JointCloseSelector as S, JointCloseTiming,
    },
    action::JointClosePhysicalObserved,
    boundary::SealedJointCloseBoundary,
    outcome,
    shm::ShmObserved,
};
use crate::node_agent_compute_plugin_host::local_authority::{
    sqlite_vfs_abi::HandleBoundSqliteAbiRawCloseWitnessSnapshot,
    sqlite_vfs_policy::registry::ManagedSqliteRegistryLifecycleStage as Stage,
};

pub(super) fn validate_and_project(
    selector: S,
    boundary: SealedJointCloseBoundary,
    raw: HandleBoundSqliteAbiRawCloseWitnessSnapshot,
    stages: &[Stage],
    shm: Option<ShmObserved>,
    physical: JointClosePhysicalObserved,
    terminal_transitions: usize,
) -> anyhow::Result<JointCloseActualCounts> {
    let has = |stage| -> anyhow::Result<u8> {
        let count = stages.iter().filter(|actual| **actual == stage).count();
        if count > 1 {
            return Err(anyhow!("JointClose stage count is not at-most-once"));
        }
        Ok(u8::from(count == 1))
    };
    let (selected_action_attempt, selected_action_success) = selected_actions(shm);
    let (
        main_unlock_attempt,
        main_unlock_success,
        main_file_close_attempt,
        main_file_close_success,
    ) = main_counts(selector, boundary);
    let (fault_observe, fault_trigger) = fault_counts(selector, boundary, shm);
    Ok(JointCloseActualCounts {
        raw_state_take_attempt: checked(raw.state_take_attempts, "raw take attempt")?,
        raw_state_take_success: checked(raw.state_take_successes, "raw take success")?,
        raw_state_abandon: checked(raw.state_abandons, "raw abandon")?,
        methods_clear: checked(raw.methods_clears, "methods clear")?,
        callback_begin: has(Stage::CallbackBegin)?,
        callback_complete_attempt: has(Stage::CallbackCompletionAttempt)?,
        callback_complete_success: has(Stage::CallbackCompletionSucceeded)?,
        selected_action_attempt,
        selected_action_success,
        shm_detach: physical.shm_detach_success,
        main_unlock_attempt,
        main_unlock_success,
        main_file_close_attempt,
        main_file_close_success,
        registry_close_attempt: has(Stage::RegistryWalMainCloseAttempt)?,
        registry_close_success: has(Stage::RegistryWalMainCloseSucceeded)?,
        connection_observe_attempt: 0,
        connection_observe_success: 0,
        registry_route_remove_attempt: 0,
        registry_route_remove_success: 0,
        logical_names_remove_attempt: 0,
        logical_names_remove_success: 0,
        logical_names_remove: 0,
        vfs_unregister_attempt: 0,
        vfs_unregister_success: 0,
        fault_observe,
        fault_trigger,
        fault_pending: 0,
        custody_retain: checked(terminal_transitions, "terminal custody transition")?,
        physical_retry: 0,
    })
}

fn selected_actions(shm: Option<ShmObserved>) -> (u8, u8) {
    let Some(shm) = shm else {
        return (0, 0);
    };
    (shm.selected_action_attempt, shm.selected_action_success)
}

fn main_counts(selector: S, boundary: SealedJointCloseBoundary) -> (u8, u8, u8, u8) {
    if outcome::is_registry(selector) || selector == S::PhysicalSuccess {
        return (1, 1, 1, 1);
    }
    if !outcome::is_main(selector) {
        return (0, 0, 0, 0);
    }
    match boundary.phase() {
        JointClosePhase::MainLockRelease => (
            u8::from(boundary.timing() != JointCloseTiming::BeforeCall),
            u8::from(boundary.timing() == JointCloseTiming::AfterSuccessKnown),
            0,
            0,
        ),
        JointClosePhase::MainFileClose => (
            1,
            1,
            u8::from(boundary.timing() != JointCloseTiming::BeforeCall),
            u8::from(boundary.timing() == JointCloseTiming::AfterSuccessKnown),
        ),
        _ => (0, 0, 0, 0),
    }
}

fn fault_counts(
    selector: S,
    boundary: SealedJointCloseBoundary,
    shm: Option<ShmObserved>,
) -> (u8, u8) {
    if let Some(shm) = shm {
        return (shm.fault_observe, shm.fault_trigger);
    }
    if selector == S::CallbackWrapperBefore
        || (outcome::is_main(selector)
            && !matches!(
                boundary.timing(),
                JointCloseTiming::NativeRetryable | JointCloseTiming::NativeUncertain
            ))
        || (outcome::is_registry(selector)
            && boundary.timing() != JointCloseTiming::NativeUncertain)
    {
        (1, 1)
    } else if outcome::is_main(selector) || outcome::is_registry(selector) {
        (1, 0)
    } else {
        (0, 0)
    }
}

fn checked(value: usize, label: &'static str) -> anyhow::Result<u8> {
    u8::try_from(value).with_context(|| format!("JointClose {label} exceeds u8"))
}
