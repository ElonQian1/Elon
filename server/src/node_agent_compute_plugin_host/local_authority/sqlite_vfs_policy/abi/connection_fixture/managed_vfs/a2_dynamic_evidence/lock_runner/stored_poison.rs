//! Real installed-ABI Lock execution from an exact stored-poison prestate.

pub(super) mod fixture;
pub(super) mod payload;

use std::path::Path;

use anyhow::{anyhow, Context};
use rusqlite::ffi;

use super::super::{SanitizedChildReport, A2_DYNAMIC_CHILD_NONCE_ENV};
use super::{
    LockRunnerActionV1, LockRunnerStoredPoisonBindingV1, LockRunnerStoredPoisonCompletionV1,
    LockRunnerStoredPoisonProfileV1,
};

pub(super) use payload::{exact_selector, validate_payload, ValidatedStoredPoisonPayloadV1};

pub(super) const SELECTED: usize = 0;

pub(super) fn validate_binding(binding: LockRunnerStoredPoisonBindingV1) -> anyhow::Result<()> {
    if binding.completion != LockRunnerStoredPoisonCompletionV1::RetentionSucceeded {
        return Err(anyhow!("q3 Lock stored-poison completion mismatch"));
    }
    validate_common_binding(binding)
}

pub(super) fn validate_common_binding(
    binding: LockRunnerStoredPoisonBindingV1,
) -> anyhow::Result<()> {
    let end = binding
        .first
        .checked_add(binding.count)
        .ok_or_else(|| anyhow!("Lock stored-poison range overflow"))?;
    if binding.count == 0 || binding.first >= 8 || end > 8 {
        return Err(anyhow!(
            "Lock stored-poison range is outside the eight-slot authority"
        ));
    }
    if matches!(
        binding.action,
        LockRunnerActionV1::LockShared | LockRunnerActionV1::UnlockShared
    ) && binding.count != 1
    {
        return Err(anyhow!("Lock stored-poison shared range is not one slot"));
    }
    let mask = (((1_u16 << binding.count) - 1) << binding.first) as u8;
    if binding.mask != mask {
        return Err(anyhow!("Lock stored-poison range mask mismatch"));
    }
    Ok(())
}

pub(super) fn exercise_child(
    root: &Path,
    binding: LockRunnerStoredPoisonBindingV1,
) -> anyhow::Result<()> {
    validate_binding(binding)?;
    exercise_child_inner(root, binding)
}

pub(super) fn exercise_child_inner(
    root: &Path,
    binding: LockRunnerStoredPoisonBindingV1,
) -> anyhow::Result<()> {
    let nonce = std::env::var(A2_DYNAMIC_CHILD_NONCE_ENV)
        .context("read parent-created Lock stored-poison child nonce")?;
    SanitizedChildReport::validate_root_before_exercise(&nonce, root)
        .map_err(anyhow::Error::msg)?;

    let fixture = fixture::prepare(root)?;
    let witness = fixture
        .route(SELECTED)?
        .installed_shm_fault_witness()
        .map_err(anyhow::Error::msg)?;
    let observer = witness.observer().map_err(anyhow::Error::msg)?;
    let target = witness.target_witness().map_err(anyhow::Error::msg)?;

    let baseline = observer.snapshot()?;
    fixture::validate_baseline(baseline)?;
    observer
        .begin_lock_action_observation(fixture::lock_expectation(binding))
        .map_err(anyhow::Error::msg)?;
    let pending_before = witness.pending_count().map_err(anyhow::Error::msg)?;
    let poison_receipt = observer
        .install_stored_poison_prestate_v1(fixture::managed_profile(binding.profile))
        .map_err(anyhow::Error::msg)?;
    fixture::validate_poison_receipt(
        binding,
        target.runtime_generation(),
        target.shm_connection_id(),
        poison_receipt,
    )?;
    let poisoned = observer.snapshot()?;
    fixture::validate_poisoned_snapshot(binding.profile, poisoned)?;

    if binding.completion == LockRunnerStoredPoisonCompletionV1::RetentionRouteUnknown {
        fixture
            .arm_unsafe_shm_route_preemption(SELECTED)
            .map_err(anyhow::Error::msg)?;
    }

    let callback = fixture
        .route(SELECTED)?
        .observe_main_shm_lock_raw(
            i32::from(binding.first),
            i32::from(binding.count),
            raw_flags(binding.action),
        )
        .map_err(anyhow::Error::msg)?;
    let after = observer.snapshot()?;
    fixture::validate_poisoned_snapshot(binding.profile, after)?;
    if callback.offset() != i32::from(binding.first)
        || callback.count() != i32::from(binding.count)
        || callback.raw_flags() != raw_flags(binding.action)
        || callback.result_code() != ffi::SQLITE_IOERR_SHMLOCK
        || !callback.before().methods_installed
        || !callback.before().state_installed
        || !callback.after().methods_installed
        || !callback.after().state_installed
    {
        return Err(anyhow!(
            "Lock stored-poison installed callback observation mismatch"
        ));
    }
    let lower_receipt = observer
        .finish_stored_poison_lock_observation()
        .map_err(anyhow::Error::msg)?;
    let pending_after = witness.pending_count().map_err(anyhow::Error::msg)?;
    fixture::validate_no_attempt_receipt(
        binding,
        target.runtime_generation(),
        target.shm_connection_id(),
        lower_receipt,
        pending_before,
        pending_after,
    )?;

    let terminal = fixture
        .route(SELECTED)?
        .terminal_custody_test_snapshot()
        .map_err(anyhow::Error::msg)?;
    let terminal_route = terminal
        .terminal_route()
        .ok_or_else(|| anyhow!("Lock stored-poison terminal route observation missing"))?;
    if terminal.joint_close_physical_failure_retention_count() != 0
        || terminal.joint_close_physical_failure().is_some()
    {
        return Err(anyhow!(
            "Lock stored-poison unexpectedly retained joint-close physical failure custody"
        ));
    }
    let terminal_values = [
        terminal.retention_count() as u64,
        terminal.callback_lease_retention_count() as u64,
        terminal.completion_evidence_retention_count() as u64,
        terminal.wal_main_physical_custody_retention_count() as u64,
        terminal.other_terminal_custody_retention_count() as u64,
        terminal.explicit_failure_custody_retained_count() as u64,
        terminal.terminal_route_observation_count() as u64,
        terminal.route_removal_count() as u64,
        u64::from(terminal.active_route_present()),
        terminal.physical_success_handoff_retention_count() as u64,
        u64::from(terminal.active_access_callback_allowed()),
        u64::from(terminal_route.terminal_reason_is_failure_custody_retained()),
        u64::from(terminal_route.connection_owner()),
        u64::from(terminal_route.main_file_lock_owner_lease()),
        terminal_route.sidecar_lease_count() as u64,
        u64::from(terminal_route.shm_lease()),
        u64::from(terminal_route.callbacks_in_flight()),
        u64::from(terminal_route.access_callback_allowed()),
    ];
    let expected_terminal_values = match binding.completion {
        LockRunnerStoredPoisonCompletionV1::RetentionSucceeded => {
            [2, 1, 0, 0, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0]
        }
        LockRunnerStoredPoisonCompletionV1::RetentionRouteUnknown => {
            [3, 1, 0, 0, 2, 2, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0]
        }
    };
    if terminal_values != expected_terminal_values {
        return Err(anyhow!(
            "Lock stored-poison completion custody ledger mismatch"
        ));
    }

    let preemption_values = match binding.completion {
        LockRunnerStoredPoisonCompletionV1::RetentionSucceeded => None,
        LockRunnerStoredPoisonCompletionV1::RetentionRouteUnknown => {
            let values = fixture
                .unsafe_shm_route_preemption_snapshot(SELECTED)
                .map_err(anyhow::Error::msg)?
                .ordered_values();
            if values != [1, 1, 1, 1, 1] {
                return Err(anyhow!(
                    "Lock stored-poison route-unknown ordered receipt mismatch"
                ));
            }
            Some(values)
        }
    };

    let registration = fixture.live_registration_snapshot()?;
    let registration_values = [
        u64::from(registration.registered()),
        u64::from(registration.table_present()),
        u64::from(registration.name_present()),
        u64::from(registration.context_present()),
    ];
    let (routes, logical_names) = fixture.logical_route_counts()?;
    let route_values = [
        fixture.live_connection_count() as u64,
        routes as u64,
        logical_names as u64,
    ];
    let root_shape_present = root.is_dir() && root.join("db").is_dir();
    if registration_values != [1, 1, 1, 1]
        || target.route_ordinal() != 1
        || route_values != [1, 1, 3]
        || !root_shape_present
    {
        return Err(anyhow!(
            "Lock stored-poison retained registration/root shape mismatch"
        ));
    }

    let q3_payload = payload::encode(
        binding,
        target.registration_id(),
        target.route_ordinal(),
        target.runtime_generation(),
        target.shm_connection_id(),
        poison_receipt,
        callback,
        baseline,
        poisoned,
        after,
        lower_receipt,
        pending_before,
        pending_after,
        terminal_values,
        registration_values,
        route_values,
        u64::from(root_shape_present),
    );
    let payload = match preemption_values {
        None => q3_payload,
        Some(receipt) => {
            super::stored_poison_route_unknown::upgrade_payload(binding, &q3_payload, receipt)?
        }
    };
    let report = SanitizedChildReport::encode_for_current_child(
        &nonce,
        root,
        target.registration_id(),
        &payload,
    )
    .map_err(anyhow::Error::msg)?;
    println!("{report}");
    Ok(())
}

pub(super) fn raw_flags(action: LockRunnerActionV1) -> i32 {
    match action {
        LockRunnerActionV1::LockShared => ffi::SQLITE_SHM_LOCK | ffi::SQLITE_SHM_SHARED,
        LockRunnerActionV1::LockExclusive => ffi::SQLITE_SHM_LOCK | ffi::SQLITE_SHM_EXCLUSIVE,
        LockRunnerActionV1::UnlockShared => ffi::SQLITE_SHM_UNLOCK | ffi::SQLITE_SHM_SHARED,
        LockRunnerActionV1::UnlockExclusive => ffi::SQLITE_SHM_UNLOCK | ffi::SQLITE_SHM_EXCLUSIVE,
    }
}

pub(super) const fn action_tag(action: LockRunnerActionV1) -> u64 {
    match action {
        LockRunnerActionV1::LockShared => 1,
        LockRunnerActionV1::LockExclusive => 2,
        LockRunnerActionV1::UnlockShared => 3,
        LockRunnerActionV1::UnlockExclusive => 4,
    }
}

impl LockRunnerStoredPoisonProfileV1 {
    pub(super) const fn tag(self) -> u64 {
        match self {
            Self::GateNoMutation => 1,
            Self::FileCloseNoMutation => 2,
            Self::ExactSiblingDeleteNoMutation => 3,
            Self::ExactSiblingOpenUncertain => 4,
            Self::DmsTruncateUncertain => 5,
            Self::FileCloseUncertain => 6,
            Self::ExactSiblingDeleteUncertain => 7,
            Self::FileGrowUncertain => 8,
            Self::MappingCloseUncertain => 9,
            Self::ViewUnmapUncertain => 10,
            Self::LockReleaseUncertain => 11,
            Self::ConnectionDetachUncertain => 12,
            Self::DeleteAuthorizationUncertain => 13,
            Self::DmsExclusiveReleaseUncertain => 14,
            Self::DmsSharedReleaseUncertain => 15,
        }
    }

    pub(super) const fn mutation_may_have_occurred(self) -> bool {
        matches!(
            self,
            Self::ExactSiblingOpenUncertain
                | Self::DmsTruncateUncertain
                | Self::FileCloseUncertain
                | Self::ExactSiblingDeleteUncertain
                | Self::FileGrowUncertain
                | Self::MappingCloseUncertain
                | Self::ViewUnmapUncertain
                | Self::DmsExclusiveReleaseUncertain
                | Self::DmsSharedReleaseUncertain
        )
    }

    pub(super) const fn lock_outcome_uncertain(self) -> bool {
        matches!(
            self,
            Self::LockReleaseUncertain
                | Self::ConnectionDetachUncertain
                | Self::DeleteAuthorizationUncertain
                | Self::DmsExclusiveReleaseUncertain
                | Self::DmsSharedReleaseUncertain
        )
    }
}
