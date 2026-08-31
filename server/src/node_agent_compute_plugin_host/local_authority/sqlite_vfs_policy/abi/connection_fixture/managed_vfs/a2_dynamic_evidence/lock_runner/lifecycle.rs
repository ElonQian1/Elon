//! Exact installed-ABI execution for positive Lock lifecycles.

mod fixture;
mod payload;

use std::path::Path;

use anyhow::{anyhow, Context};
use rusqlite::ffi;

use crate::node_agent_managed_fs::{
    ManagedSqliteShmLockAction, ManagedSqliteShmTestLockExpectation, ManagedSqliteShmTestLockPath,
    ManagedSqliteShmTestLockReceipt, ManagedSqliteShmTestTargetObserver,
};

use super::super::{SanitizedChildReport, A2_DYNAMIC_CHILD_NONCE_ENV};
use super::{LockRunnerActionV1, LockRunnerLifecycleBindingV1, LockRunnerLifecyclePathV1};

pub(super) use payload::{validate_payload, ValidatedLifecyclePayloadV1};

pub(super) const SELECTED: usize = 0;
pub(super) const SIBLING: usize = 1;

struct ArmedLockObservation<'a> {
    observer: &'a ManagedSqliteShmTestTargetObserver,
    active: bool,
}

impl<'a> ArmedLockObservation<'a> {
    fn begin(
        observer: &'a ManagedSqliteShmTestTargetObserver,
        expectation: ManagedSqliteShmTestLockExpectation,
    ) -> anyhow::Result<Self> {
        observer
            .begin_lock_action_observation(expectation)
            .map_err(anyhow::Error::msg)?;
        Ok(Self {
            observer,
            active: true,
        })
    }

    fn finish(mut self) -> anyhow::Result<ManagedSqliteShmTestLockReceipt> {
        let receipt = self
            .observer
            .finish_lock_action_observation()
            .map_err(anyhow::Error::msg)?;
        self.active = false;
        Ok(receipt)
    }
}

impl Drop for ArmedLockObservation<'_> {
    fn drop(&mut self) {
        if self.active {
            let _ = self.observer.cancel_lock_action_observation();
        }
    }
}

pub(super) fn validate_binding(binding: LockRunnerLifecycleBindingV1) -> anyhow::Result<()> {
    let end = binding
        .first
        .checked_add(binding.count)
        .ok_or_else(|| anyhow!("Lock lifecycle range overflow"))?;
    if binding.count == 0 || binding.first >= 8 || end > 8 {
        return Err(anyhow!(
            "Lock lifecycle range is outside the eight-slot authority"
        ));
    }
    let mask = (((1_u16 << binding.count) - 1) << binding.first) as u8;
    if binding.mask != mask {
        return Err(anyhow!("Lock lifecycle range mask mismatch"));
    }
    let supported = match (binding.path, binding.action) {
        (LockRunnerLifecyclePathV1::NativeAcquire, LockRunnerActionV1::LockShared)
        | (LockRunnerLifecyclePathV1::NativeRelease, LockRunnerActionV1::UnlockShared)
        | (LockRunnerLifecyclePathV1::SharedLocalAcquire, LockRunnerActionV1::LockShared)
        | (LockRunnerLifecyclePathV1::SharedLocalRelease, LockRunnerActionV1::UnlockShared) => {
            binding.count == 1
        }
        (LockRunnerLifecyclePathV1::NativeAcquire, LockRunnerActionV1::LockExclusive)
        | (LockRunnerLifecyclePathV1::NativeRelease, LockRunnerActionV1::UnlockExclusive) => true,
        _ => false,
    };
    if !supported {
        return Err(anyhow!("Lock lifecycle action/path tuple is unsupported"));
    }
    Ok(())
}

pub(super) fn exercise_child(
    root: &Path,
    binding: LockRunnerLifecycleBindingV1,
) -> anyhow::Result<()> {
    validate_binding(binding)?;
    let nonce = std::env::var(A2_DYNAMIC_CHILD_NONCE_ENV)
        .context("read parent-created Lock lifecycle child nonce")?;
    SanitizedChildReport::validate_root_before_exercise(&nonce, root)
        .map_err(anyhow::Error::msg)?;

    let fixture = fixture::prepare(root, binding.path)?;
    let selected_binding = fixture
        .route(SELECTED)?
        .installed_shm_fault_witness()
        .map_err(anyhow::Error::msg)?;
    let selected_observer = selected_binding.observer().map_err(anyhow::Error::msg)?;
    let target = selected_binding
        .target_witness()
        .map_err(anyhow::Error::msg)?;
    let sibling_observer = if binding.path.is_local() {
        Some(
            fixture
                .route(SIBLING)?
                .installed_shm_fault_witness()
                .map_err(anyhow::Error::msg)?
                .observer()
                .map_err(anyhow::Error::msg)?,
        )
    } else {
        None
    };

    fixture::install_prestate(&fixture, binding)?;
    let selected_before = selected_observer.snapshot()?;
    let sibling_before = sibling_observer
        .as_ref()
        .map(|observer| observer.snapshot())
        .transpose()?
        .map(fixture::sibling_values)
        .unwrap_or([0; 3]);
    fixture::validate_prestate(binding, selected_before, sibling_before)?;

    let lock_observation =
        ArmedLockObservation::begin(&selected_observer, lock_expectation(binding))?;
    let callback = fixture
        .route(SELECTED)?
        .observe_main_shm_lock_raw(
            i32::from(binding.first),
            i32::from(binding.count),
            raw_flags(binding.action),
        )
        .map_err(anyhow::Error::msg)?;
    let selected_after = selected_observer.snapshot()?;
    let sibling_after = sibling_observer
        .as_ref()
        .map(|observer| observer.snapshot())
        .transpose()?
        .map(fixture::sibling_values)
        .unwrap_or([0; 3]);
    let receipt = lock_observation.finish()?;
    let pending_count = selected_binding
        .pending_count()
        .map_err(anyhow::Error::msg)?;
    fixture::validate_action(
        binding,
        callback,
        selected_before,
        selected_after,
        sibling_before,
        sibling_after,
        receipt,
        pending_count,
    )?;

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
    let autocommit = fixture.connection(SELECTED)?.is_autocommit();
    let liveness: i64 = fixture
        .connection(SELECTED)?
        .query_row("SELECT 1", [], |row| row.get(0))?;

    fixture::cleanup_locks(&fixture, binding)?;
    fixture.close()?;
    let terminal_values = [
        u64::from(autocommit),
        u64::from(liveness == 1),
        1,
        u64::from(root.is_dir()),
    ];
    let payload = payload::encode(
        binding,
        target.registration_id(),
        target.route_ordinal(),
        target.runtime_generation(),
        target.shm_connection_id(),
        callback,
        selected_before,
        selected_after,
        sibling_before,
        sibling_after,
        receipt,
        pending_count,
        registration_values,
        route_values,
        terminal_values,
    );
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

pub(super) fn lock_expectation(
    binding: LockRunnerLifecycleBindingV1,
) -> ManagedSqliteShmTestLockExpectation {
    ManagedSqliteShmTestLockExpectation {
        action: managed_action(binding.action),
        first: binding.first,
        count: binding.count,
        mask: binding.mask,
        path: match binding.path {
            LockRunnerLifecyclePathV1::NativeAcquire => ManagedSqliteShmTestLockPath::NativeAcquire,
            LockRunnerLifecyclePathV1::NativeRelease => ManagedSqliteShmTestLockPath::NativeRelease,
            LockRunnerLifecyclePathV1::SharedLocalAcquire
            | LockRunnerLifecyclePathV1::SharedLocalRelease => ManagedSqliteShmTestLockPath::Local,
        },
    }
}

pub(super) fn managed_action(action: LockRunnerActionV1) -> ManagedSqliteShmLockAction {
    match action {
        LockRunnerActionV1::LockShared => ManagedSqliteShmLockAction::LockShared,
        LockRunnerActionV1::LockExclusive => ManagedSqliteShmLockAction::LockExclusive,
        LockRunnerActionV1::UnlockShared => ManagedSqliteShmLockAction::UnlockShared,
        LockRunnerActionV1::UnlockExclusive => ManagedSqliteShmLockAction::UnlockExclusive,
    }
}

pub(super) fn raw_flags(action: LockRunnerActionV1) -> i32 {
    match action {
        LockRunnerActionV1::LockShared => ffi::SQLITE_SHM_LOCK | ffi::SQLITE_SHM_SHARED,
        LockRunnerActionV1::LockExclusive => ffi::SQLITE_SHM_LOCK | ffi::SQLITE_SHM_EXCLUSIVE,
        LockRunnerActionV1::UnlockShared => ffi::SQLITE_SHM_UNLOCK | ffi::SQLITE_SHM_SHARED,
        LockRunnerActionV1::UnlockExclusive => ffi::SQLITE_SHM_UNLOCK | ffi::SQLITE_SHM_EXCLUSIVE,
    }
}

pub(super) fn action_tag(action: LockRunnerActionV1) -> u64 {
    match action {
        LockRunnerActionV1::LockShared => 1,
        LockRunnerActionV1::LockExclusive => 2,
        LockRunnerActionV1::UnlockShared => 3,
        LockRunnerActionV1::UnlockExclusive => 4,
    }
}

pub(super) fn path_tag(path: LockRunnerLifecyclePathV1) -> u64 {
    match path {
        LockRunnerLifecyclePathV1::NativeAcquire => 1,
        LockRunnerLifecyclePathV1::NativeRelease => 2,
        LockRunnerLifecyclePathV1::SharedLocalAcquire => 3,
        LockRunnerLifecyclePathV1::SharedLocalRelease => 4,
    }
}

impl LockRunnerLifecyclePathV1 {
    pub(super) fn is_local(self) -> bool {
        matches!(self, Self::SharedLocalAcquire | Self::SharedLocalRelease)
    }

    pub(super) fn connection_count(self) -> u8 {
        if self.is_local() {
            2
        } else {
            1
        }
    }
}

impl LockRunnerActionV1 {
    pub(super) fn acquire_pair(self) -> Self {
        match self {
            Self::UnlockShared => Self::LockShared,
            Self::UnlockExclusive => Self::LockExclusive,
            other => other,
        }
    }

    pub(super) fn release_pair(self) -> Self {
        match self {
            Self::LockShared => Self::UnlockShared,
            Self::LockExclusive => Self::UnlockExclusive,
            other => other,
        }
    }
}
