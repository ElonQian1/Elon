//! Real installed-ABI prestates and observations for the six q9 source profiles.

use std::{num::NonZeroU8, path::Path};

use anyhow::{anyhow, Context};
use rusqlite::ffi;

use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryTerminalCustodyTestSnapshot;
use crate::node_agent_managed_fs::ManagedSqliteShmLockRequest;

use super::super::super::super::{
    lifecycle_faults::{ManagedTestPreManagedLockPath, ManagedTestPreManagedLockSnapshot},
    ManagedSqliteRoutedConnectionFixture, ManagedSqliteTestVfsRouteCustodySnapshot,
    ManagedSqliteTestVfsRoutePhase,
};
use super::super::super::{SanitizedChildReport, A2_DYNAMIC_CHILD_NONCE_ENV};
use super::super::lifecycle;
use super::{
    payload, validate_binding, LockRunnerPreManagedCompletionV1,
    LockRunnerPreManagedRejectionBindingV1, LockRunnerPreManagedRejectionV1,
};

pub(super) fn exercise_child(
    root: &Path,
    binding: LockRunnerPreManagedRejectionBindingV1,
) -> anyhow::Result<()> {
    validate_binding(binding)?;
    let nonce = std::env::var(A2_DYNAMIC_CHILD_NONCE_ENV)
        .context("read parent-created q9 Lock child nonce")?;
    SanitizedChildReport::validate_root_before_exercise(&nonce, root)
        .map_err(anyhow::Error::msg)?;

    let fixture = ManagedSqliteRoutedConnectionFixture::open(root, [0xa9; 16])?;
    let registration_id = fixture.registration_id_for_test();
    let route_ordinal = fixture.route_ordinal().counter_value();
    if registration_id == 0 || route_ordinal == 0 {
        return Err(anyhow!("q9 Lock registration/route identity is zero"));
    }
    let setup = prepare_prestate(&fixture, binding)?;
    let request = lock_request(binding)?;
    fixture
        .arm_pre_managed_lock_observation(observation_path(binding)?, request)
        .map_err(anyhow::Error::msg)?;

    let mut prime = [0; 4];
    let mut admission_quarantine = 0;
    match binding.rejection {
        LockRunnerPreManagedRejectionV1::AdmissionRouteUnknown => {
            fixture
                .quarantine_for_lock_admission_test()
                .map_err(anyhow::Error::msg)?;
            admission_quarantine = 1;
        }
        LockRunnerPreManagedRejectionV1::AdmissionCounterOverflow => {
            prime = fixture
                .prime_lock_callback_counter_overflow()
                .map_err(anyhow::Error::msg)?
                .ordered_values();
        }
        LockRunnerPreManagedRejectionV1::UnsupportedFileRole
        | LockRunnerPreManagedRejectionV1::ShmDetached => {}
    }

    let raw = fixture
        .observe_main_shm_lock_raw(
            i32::from(binding.first),
            i32::from(binding.count),
            lifecycle::raw_flags(binding.action),
        )
        .map_err(anyhow::Error::msg)?;
    if raw.result_code() != ffi::SQLITE_IOERR_SHMLOCK
        || !raw.before().methods_installed
        || !raw.before().state_installed
        || !raw.after().methods_installed
        || !raw.after().state_installed
    {
        return Err(anyhow!("q9 installed xShmLock receipt mismatch"));
    }
    let observation = fixture
        .pre_managed_lock_snapshot()
        .map_err(anyhow::Error::msg)?;
    let counter_terminal =
        if binding.rejection == LockRunnerPreManagedRejectionV1::AdmissionCounterOverflow {
            fixture
                .lock_callback_counter_overflow_terminal()
                .map_err(anyhow::Error::msg)?
        } else {
            false
        };
    let terminal = terminal_values(
        fixture
            .terminal_custody_test_snapshot()
            .map_err(anyhow::Error::msg)?,
        counter_terminal,
    );
    let route = route_values(fixture.route_custody_snapshot().ok());
    let registration = fixture.live_registration_snapshot_for_test()?;
    let registration = [
        u64::from(registration.registered()),
        u64::from(registration.table_present()),
        u64::from(registration.name_present()),
        u64::from(registration.context_present()),
    ];
    if registration != [1; 4] || !root.is_dir() {
        return Err(anyhow!("q9 Lock live registration/root witness mismatch"));
    }

    let completed = binding.completion == LockRunnerPreManagedCompletionV1::Completed;
    let cleanup = if completed {
        fixture.close()?;
        [1, 0, 1, 1, 1]
    } else {
        std::mem::forget(fixture);
        [0, 1, 1, 1, 1]
    };
    let payload = payload::encode(
        binding,
        registration_id,
        route_ordinal,
        raw,
        setup,
        prime,
        admission_quarantine,
        observation,
        terminal,
        route,
        registration,
        cleanup,
    );
    let report =
        SanitizedChildReport::encode_for_current_child(&nonce, root, registration_id, &payload)
            .map_err(anyhow::Error::msg)?;
    println!("{report}");
    Ok(())
}

fn prepare_prestate(
    fixture: &ManagedSqliteRoutedConnectionFixture,
    binding: LockRunnerPreManagedRejectionBindingV1,
) -> anyhow::Result<[u64; 19]> {
    if binding.rejection != LockRunnerPreManagedRejectionV1::ShmDetached {
        if fixture
            .exact_main_shm_target_presence()
            .map_err(anyhow::Error::msg)?
        {
            return Err(anyhow!("q9 Main prestate unexpectedly had SHM"));
        }
        return Ok([0; 19]);
    }
    let map = fixture
        .call_main_shm_map_raw(0, 32_768, 1)
        .map_err(anyhow::Error::msg)?;
    let present_after_map = fixture
        .exact_main_shm_target_presence()
        .map_err(anyhow::Error::msg)?;
    if map.result_code() != ffi::SQLITE_OK || map.output_pointer().is_null() || !present_after_map {
        return Err(anyhow!("q9 detached prestate real xShmMap mismatch"));
    }
    let unmap = fixture
        .call_main_shm_unmap_raw(0)
        .map_err(anyhow::Error::msg)?;
    let present_after_unmap = fixture
        .exact_main_shm_target_presence()
        .map_err(anyhow::Error::msg)?;
    if unmap.result_code() != ffi::SQLITE_OK || present_after_unmap {
        return Err(anyhow!("q9 detached prestate real xShmUnmap mismatch"));
    }
    Ok([
        1,
        map.region() as u64,
        map.region_size() as u64,
        map.raw_extend() as u64,
        map.result_code() as u64,
        u64::from(!map.output_pointer().is_null()),
        u64::from(map.before().methods_installed),
        u64::from(map.before().state_installed),
        u64::from(map.after().methods_installed),
        u64::from(map.after().state_installed),
        u64::from(present_after_map),
        1,
        unmap.raw_delete() as u64,
        unmap.result_code() as u64,
        u64::from(unmap.before().methods_installed),
        u64::from(unmap.before().state_installed),
        u64::from(unmap.after().methods_installed),
        u64::from(unmap.after().state_installed),
        u64::from(present_after_unmap),
    ])
}

fn lock_request(
    binding: LockRunnerPreManagedRejectionBindingV1,
) -> anyhow::Result<ManagedSqliteShmLockRequest> {
    ManagedSqliteShmLockRequest::new(
        binding.first,
        NonZeroU8::new(binding.count).ok_or_else(|| anyhow!("q9 zero Lock count"))?,
        lifecycle::managed_action(binding.action),
    )
    .map_err(anyhow::Error::msg)
}

fn observation_path(
    binding: LockRunnerPreManagedRejectionBindingV1,
) -> anyhow::Result<ManagedTestPreManagedLockPath> {
    use LockRunnerPreManagedCompletionV1 as C;
    use LockRunnerPreManagedRejectionV1 as R;
    match (binding.rejection, binding.completion) {
        (R::AdmissionRouteUnknown, C::Direct) => {
            Ok(ManagedTestPreManagedLockPath::AdmissionRouteUnknown)
        }
        (R::AdmissionCounterOverflow, C::Direct) => {
            Ok(ManagedTestPreManagedLockPath::AdmissionCounterOverflow)
        }
        (R::UnsupportedFileRole, C::Completed) => {
            Ok(ManagedTestPreManagedLockPath::UnsupportedCompleted)
        }
        (R::UnsupportedFileRole, C::RouteUnknown) => {
            Ok(ManagedTestPreManagedLockPath::UnsupportedRouteUnknown)
        }
        (R::ShmDetached, C::Completed) => Ok(ManagedTestPreManagedLockPath::ShmDetachedCompleted),
        (R::ShmDetached, C::RouteUnknown) => {
            Ok(ManagedTestPreManagedLockPath::ShmDetachedRouteUnknown)
        }
        _ => Err(anyhow!("q9 Lock observation path mismatch")),
    }
}

fn terminal_values(
    snapshot: ManagedSqliteRegistryTerminalCustodyTestSnapshot,
    counter_terminal: bool,
) -> [u64; 17] {
    let terminal = snapshot.terminal_route();
    [
        snapshot.retention_count() as u64,
        snapshot.callback_lease_retention_count() as u64,
        snapshot.completion_evidence_retention_count() as u64,
        snapshot.wal_main_physical_custody_retention_count() as u64,
        snapshot.other_terminal_custody_retention_count() as u64,
        snapshot.explicit_failure_custody_retained_count() as u64,
        snapshot.terminal_route_observation_count() as u64,
        snapshot.route_removal_count() as u64,
        u64::from(snapshot.active_route_present()),
        u64::from(counter_terminal),
        u64::from(
            terminal.is_some_and(|route| route.terminal_reason_is_failure_custody_retained()),
        ),
        terminal.map_or(0, |route| route.callbacks_in_flight() as u64),
        u64::from(terminal.is_some_and(|route| route.access_callback_allowed())),
        u64::from(terminal.is_some_and(|route| route.connection_owner())),
        u64::from(terminal.is_some_and(|route| route.main_file_lock_owner_lease())),
        u64::from(terminal.is_some_and(|route| route.shm_lease())),
        u64::from(snapshot.active_access_callback_allowed()),
    ]
}

fn route_values(snapshot: Option<ManagedSqliteTestVfsRouteCustodySnapshot>) -> [u64; 7] {
    let Some(snapshot) = snapshot else {
        return [0; 7];
    };
    [
        1,
        phase_tag(snapshot.phase()),
        u64::from(snapshot.connection_owner()),
        u64::from(snapshot.main_file_lock_owner_lease()),
        u64::from(snapshot.shm_lease()),
        snapshot.callbacks_in_flight() as u64,
        u64::from(snapshot.access_callback_allowed()),
    ]
}

const fn phase_tag(phase: ManagedSqliteTestVfsRoutePhase) -> u64 {
    match phase {
        ManagedSqliteTestVfsRoutePhase::PendingMain => 1,
        ManagedSqliteTestVfsRoutePhase::Opening => 2,
        ManagedSqliteTestVfsRoutePhase::Active => 3,
        ManagedSqliteTestVfsRoutePhase::Closing => 4,
        ManagedSqliteTestVfsRoutePhase::AwaitingRouteRetirement => 5,
        ManagedSqliteTestVfsRoutePhase::Retired => 6,
        ManagedSqliteTestVfsRoutePhase::TerminalQuarantine => 7,
    }
}

pub(super) const fn lock_effect(binding: LockRunnerPreManagedRejectionBindingV1) -> u64 {
    match binding.rejection {
        LockRunnerPreManagedRejectionV1::AdmissionRouteUnknown
        | LockRunnerPreManagedRejectionV1::AdmissionCounterOverflow => 1,
        LockRunnerPreManagedRejectionV1::UnsupportedFileRole
        | LockRunnerPreManagedRejectionV1::ShmDetached => 2,
    }
}

pub(super) const fn lifecycle_values(snapshot: ManagedTestPreManagedLockSnapshot) -> [u64; 18] {
    snapshot.ordered_values()
}
