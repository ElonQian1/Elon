//! Real q2/q5/q6 fixtures composed with the ordinary Lock completion preemption seam.

use std::path::Path;

use crate::node_agent_managed_fs::{
    ManagedSqliteShmLockAttempt, ManagedSqliteShmTestLockExpectation, ManagedSqliteShmTestLockPath,
    ManagedSqliteShmTestLockReceipt, ManagedSqliteShmTestNativeContentionReceipt,
    ManagedSqliteShmTestTargetObserver, ManagedSqliteShmTestTargetSnapshot,
};
use anyhow::{anyhow, Context};

use super::super::super::super::{
    connection::ManagedTestShmLockCallbackObservation, ManagedSqliteMultiConnectionFixture,
};
use super::super::super::{SanitizedChildReport, A2_DYNAMIC_CHILD_NONCE_ENV};
use super::super::{
    lifecycle, local_sibling_contention, native_acquire_busy, LockRunnerActionV1,
    LockRunnerLifecycleBindingV1, LockRunnerLifecyclePathV1,
    LockRunnerLocalSiblingContentionBindingV1,
};
use super::{
    payload, validate_binding, LockRunnerCallbackRouteUnknownBindingV1,
    LockRunnerCallbackRouteUnknownPathV1,
};

const SELECTED: usize = 0;
const SIBLING: usize = 1;

mod validation;
pub(super) use validation::{snapshot_values, target_values};
use validation::{terminal_values, validate_cleaned, validate_lower, validate_snapshots};

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

pub(super) fn exercise_child(
    root: &Path,
    binding: LockRunnerCallbackRouteUnknownBindingV1,
) -> anyhow::Result<()> {
    validate_binding(binding)?;
    let nonce = std::env::var(A2_DYNAMIC_CHILD_NONCE_ENV)
        .context("read parent-created Lock callback RouteUnknown child nonce")?;
    SanitizedChildReport::validate_root_before_exercise(&nonce, root)
        .map_err(anyhow::Error::msg)?;

    let fixture = prepare(root, binding)?;
    let selected_binding = fixture
        .route(SELECTED)?
        .installed_shm_fault_witness()
        .map_err(anyhow::Error::msg)?;
    let selected_observer = selected_binding.observer().map_err(anyhow::Error::msg)?;
    let target = selected_binding
        .target_witness()
        .map_err(anyhow::Error::msg)?;
    let sibling_observer = if binding.path.connection_count() == 2 {
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

    install_prestate(&fixture, binding)?;
    let selected_before = selected_observer.snapshot()?;
    let sibling_before = snapshot_optional(sibling_observer.as_ref())?;
    validate_snapshots(binding, selected_before, sibling_before, false)?;

    let expectation = lock_expectation(binding);
    let lower_observation = ArmedLockObservation::begin(&selected_observer, expectation)?;
    fixture
        .arm_ordinary_shm_lock_route_preemption(
            SELECTED,
            expectation,
            expected_attempt(binding.path),
        )
        .map_err(anyhow::Error::msg)?;
    let (callback, holder) = invoke_real_lower(&fixture, &selected_observer, binding)?;
    let selected_after = selected_observer.snapshot()?;
    let sibling_after = snapshot_optional(sibling_observer.as_ref())?;
    let lower = lower_observation.finish()?;
    let pending_count = selected_binding
        .pending_count()
        .map_err(anyhow::Error::msg)?;
    validate_lower(
        binding,
        callback,
        selected_before,
        selected_after,
        sibling_before,
        sibling_after,
        lower,
        holder,
        pending_count,
    )?;

    let preemption = fixture
        .ordinary_shm_lock_route_preemption_snapshot(SELECTED)
        .map_err(anyhow::Error::msg)?
        .ordered_values();
    if preemption != [1; 6] {
        return Err(anyhow!(
            "Lock callback RouteUnknown ordered preemption receipt mismatch"
        ));
    }
    let terminal = fixture
        .route(SELECTED)?
        .terminal_custody_test_snapshot()
        .map_err(anyhow::Error::msg)?;
    let terminal = terminal_values(terminal)?;

    cleanup_sibling(&fixture, binding)?;
    let selected_cleaned = selected_observer.snapshot()?;
    let sibling_cleaned = snapshot_optional(sibling_observer.as_ref())?;
    validate_cleaned(binding, selected_cleaned, sibling_cleaned)?;

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
    if registration_values != [1; 4]
        || target.route_ordinal() != 1
        || route_values
            != if binding.path.connection_count() == 1 {
                [1, 1, 3]
            } else {
                [2, 2, 6]
            }
        || !root_shape_present
    {
        return Err(anyhow!(
            "Lock callback RouteUnknown retained registration/root shape mismatch"
        ));
    }

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
        selected_cleaned,
        sibling_cleaned,
        holder,
        lower,
        pending_count,
        preemption,
        terminal,
        registration_values,
        route_values,
        u64::from(root_shape_present),
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

fn prepare(
    root: &Path,
    binding: LockRunnerCallbackRouteUnknownBindingV1,
) -> anyhow::Result<ManagedSqliteMultiConnectionFixture> {
    match binding.path {
        LockRunnerCallbackRouteUnknownPathV1::NativeAcquireBusy => {
            native_acquire_busy::fixture::prepare(root)
        }
        LockRunnerCallbackRouteUnknownPathV1::LocalSiblingContention => {
            local_sibling_contention::fixture::prepare(root)
        }
        _ => lifecycle::fixture::prepare(root, lifecycle_binding(binding)?.path),
    }
}

fn install_prestate(
    fixture: &ManagedSqliteMultiConnectionFixture,
    binding: LockRunnerCallbackRouteUnknownBindingV1,
) -> anyhow::Result<()> {
    if binding.path == LockRunnerCallbackRouteUnknownPathV1::LocalSiblingContention {
        local_sibling_contention::fixture::install_prestate(fixture, sibling_binding(binding)?)
    } else if binding.path == LockRunnerCallbackRouteUnknownPathV1::NativeAcquireBusy {
        Ok(())
    } else {
        lifecycle::fixture::install_prestate(fixture, lifecycle_binding(binding)?)
    }
}

fn cleanup_sibling(
    fixture: &ManagedSqliteMultiConnectionFixture,
    binding: LockRunnerCallbackRouteUnknownBindingV1,
) -> anyhow::Result<()> {
    if binding.path == LockRunnerCallbackRouteUnknownPathV1::LocalSiblingContention {
        local_sibling_contention::fixture::cleanup_locks(fixture, sibling_binding(binding)?)
    } else if matches!(
        binding.path,
        LockRunnerCallbackRouteUnknownPathV1::SharedLocalAcquire
            | LockRunnerCallbackRouteUnknownPathV1::SharedLocalRelease
    ) {
        lifecycle::fixture::cleanup_route_unknown_sibling(fixture, lifecycle_binding(binding)?)
    } else {
        Ok(())
    }
}

fn invoke_real_lower(
    fixture: &ManagedSqliteMultiConnectionFixture,
    observer: &ManagedSqliteShmTestTargetObserver,
    binding: LockRunnerCallbackRouteUnknownBindingV1,
) -> anyhow::Result<(
    ManagedTestShmLockCallbackObservation,
    Option<ManagedSqliteShmTestNativeContentionReceipt>,
)> {
    let selected_route = fixture.route(SELECTED)?;
    let invoke = || {
        selected_route.observe_main_shm_lock_raw(
            i32::from(binding.first),
            i32::from(binding.count),
            lifecycle::raw_flags(binding.action),
        )
    };
    if binding.path == LockRunnerCallbackRouteUnknownPathV1::NativeAcquireBusy {
        let (callback, holder) = observer
            .with_native_lock_contention(binding.first, binding.count, invoke)
            .map_err(anyhow::Error::msg)?;
        Ok((callback, Some(holder)))
    } else {
        Ok((invoke().map_err(anyhow::Error::msg)?, None))
    }
}

pub(super) fn lock_expectation(
    binding: LockRunnerCallbackRouteUnknownBindingV1,
) -> ManagedSqliteShmTestLockExpectation {
    ManagedSqliteShmTestLockExpectation {
        action: lifecycle::managed_action(binding.action),
        first: binding.first,
        count: binding.count,
        mask: binding.mask,
        path: match binding.path {
            LockRunnerCallbackRouteUnknownPathV1::NativeAcquireAcquired
            | LockRunnerCallbackRouteUnknownPathV1::NativeAcquireBusy => {
                ManagedSqliteShmTestLockPath::NativeAcquire
            }
            LockRunnerCallbackRouteUnknownPathV1::NativeRelease => {
                ManagedSqliteShmTestLockPath::NativeRelease
            }
            LockRunnerCallbackRouteUnknownPathV1::SharedLocalAcquire
            | LockRunnerCallbackRouteUnknownPathV1::SharedLocalRelease => {
                ManagedSqliteShmTestLockPath::Local
            }
            LockRunnerCallbackRouteUnknownPathV1::LocalSiblingContention => {
                ManagedSqliteShmTestLockPath::SiblingContention
            }
        },
    }
}

fn expected_attempt(path: LockRunnerCallbackRouteUnknownPathV1) -> ManagedSqliteShmLockAttempt {
    if path.is_contended() {
        ManagedSqliteShmLockAttempt::Contended
    } else {
        ManagedSqliteShmLockAttempt::Acquired
    }
}

fn lifecycle_binding(
    binding: LockRunnerCallbackRouteUnknownBindingV1,
) -> anyhow::Result<LockRunnerLifecycleBindingV1> {
    let path = match binding.path {
        LockRunnerCallbackRouteUnknownPathV1::NativeAcquireAcquired => {
            LockRunnerLifecyclePathV1::NativeAcquire
        }
        LockRunnerCallbackRouteUnknownPathV1::NativeRelease => {
            LockRunnerLifecyclePathV1::NativeRelease
        }
        LockRunnerCallbackRouteUnknownPathV1::SharedLocalAcquire => {
            LockRunnerLifecyclePathV1::SharedLocalAcquire
        }
        LockRunnerCallbackRouteUnknownPathV1::SharedLocalRelease => {
            LockRunnerLifecyclePathV1::SharedLocalRelease
        }
        _ => return Err(anyhow!("path does not use the q2 lifecycle fixture")),
    };
    Ok(LockRunnerLifecycleBindingV1 {
        path,
        action: binding.action,
        first: binding.first,
        count: binding.count,
        mask: binding.mask,
        normalized_descriptor_sha256: binding.normalized_descriptor_sha256,
        case_key_sha256: binding.case_key_sha256,
        full_record_sha256: binding.full_record_sha256,
        plan_sha256: binding.plan_sha256,
        implementation_sha256: binding.implementation_sha256,
    })
}

fn sibling_binding(
    binding: LockRunnerCallbackRouteUnknownBindingV1,
) -> anyhow::Result<LockRunnerLocalSiblingContentionBindingV1> {
    if binding.path != LockRunnerCallbackRouteUnknownPathV1::LocalSiblingContention {
        return Err(anyhow!("path does not use the q6 sibling fixture"));
    }
    Ok(LockRunnerLocalSiblingContentionBindingV1 {
        action: binding.action,
        first: binding.first,
        count: binding.count,
        mask: binding.mask,
        normalized_descriptor_sha256: binding.normalized_descriptor_sha256,
        case_key_sha256: binding.case_key_sha256,
        full_record_sha256: binding.full_record_sha256,
        plan_sha256: binding.plan_sha256,
        implementation_sha256: binding.implementation_sha256,
    })
}

fn snapshot_optional(
    observer: Option<&ManagedSqliteShmTestTargetObserver>,
) -> anyhow::Result<Option<ManagedSqliteShmTestTargetSnapshot>> {
    observer.map(|observer| observer.snapshot()).transpose()
}
