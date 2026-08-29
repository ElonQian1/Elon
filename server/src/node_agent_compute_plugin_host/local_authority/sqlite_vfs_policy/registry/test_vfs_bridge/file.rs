use std::{
    num::{NonZeroU32, NonZeroU8},
    sync::Arc,
};

use super::super::file_custody::{
    ManagedSqliteRegistryCloseLifecycleFaults, ManagedSqliteRegistryCloseLifecyclePhase,
    ManagedSqliteRegistryLifecycleStage,
};
use super::super::process_owner::ManagedSqliteRegistryTerminalCustodyTestSnapshot;
use super::*;
use crate::node_agent_compute_plugin_host::local_authority::{
    sqlite_vfs_abi::HandleBoundSqliteFileOperations,
    sqlite_vfs_policy::{
        HandleBoundSqliteAbiAttempt, HandleBoundSqliteAbiLockLevel,
        HandleBoundSqliteAbiShmLockAction, HandleBoundSqliteAbiShmMap,
        HandleBoundSqliteAbiUnlockLevel,
    },
};
use crate::node_agent_managed_fs::{
    ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase, ManagedSqliteShmTestFaultProbe,
    ManagedSqliteShmTestTargetObserver,
};

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) struct ManagedSqliteTestVfsFile<
    Custody,
    NonceSource,
> where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    file: HandleBoundSqliteAbiFile<Custody, NonceSource>,
    owner: &'static ManagedSqliteRegistryProcessOwner<Custody, NonceSource>,
    route: ManagedSqliteRegistryRouteHandle,
    role: ManagedSqliteLogicalFileRole,
    wal_runtime: Option<Arc<PinnedManagedSqliteWalRuntime>>,
    close_faults: Option<Arc<dyn ManagedSqliteRegistryCloseLifecycleFaults>>,
}

impl<Custody, NonceSource> ManagedSqliteTestVfsFile<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(super) fn new(
        file: HandleBoundSqliteAbiFile<Custody, NonceSource>,
        owner: &'static ManagedSqliteRegistryProcessOwner<Custody, NonceSource>,
        route: ManagedSqliteRegistryRouteHandle,
        role: ManagedSqliteLogicalFileRole,
        wal_runtime: Option<Arc<PinnedManagedSqliteWalRuntime>>,
        close_faults: Option<Arc<dyn ManagedSqliteRegistryCloseLifecycleFaults>>,
    ) -> Self {
        Self {
            file,
            owner,
            route,
            role,
            wal_runtime,
            close_faults,
        }
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn prepare_wal_main_shm_test_fault_script(
        &mut self,
        before_call: &[(ManagedSqliteShmFailurePhase, u32)],
        after_success: &[(
            ManagedSqliteShmFailurePhase,
            u32,
            ManagedSqliteShmFailureClass,
        )],
    ) -> Result<ManagedSqliteShmTestFaultProbe, ()> {
        if self.role != ManagedSqliteLogicalFileRole::Main {
            return Err(());
        }
        let runtime = self.wal_runtime.as_deref().ok_or(())?;
        self.file.promote_main_to_wal(runtime)?;
        self.file
            .install_exact_wal_main_shm_test_fault_script(before_call, after_success)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn promote_main_to_wal_for_shm(
        &mut self,
    ) -> Result<ManagedSqliteShmTestTargetObserver, ()> {
        if self.role != ManagedSqliteLogicalFileRole::Main {
            return Err(());
        }
        let runtime = self.wal_runtime.as_deref().ok_or(())?;
        self.file.promote_main_to_wal(runtime)?;
        self.file.exact_wal_main_shm_test_target_observer()
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn retain_test_fault_bridge_failure(
        &self,
        code: &'static str,
    ) -> Result<(), ()> {
        self.owner
            .retain_terminal_custody(
                self.route,
                ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                code,
            )
            .map_err(drop)
    }

    /// Returns only redacted counts and route-presence state for this file's exact private route.
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn terminal_custody_test_snapshot(
        &self,
    ) -> Result<ManagedSqliteRegistryTerminalCustodyTestSnapshot, ()> {
        self.owner
            .terminal_custody_test_snapshot(self.route)
            .map_err(drop)
    }
}

impl<Custody, NonceSource> HandleBoundSqliteFileOperations
    for ManagedSqliteTestVfsFile<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    fn read_at_zero_filled(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize, ()> {
        self.file.read_at_zero_filled(offset, buffer)
    }

    fn write_all_at(&mut self, offset: u64, bytes: &[u8]) -> Result<(), ()> {
        self.file.write_all_at(offset, bytes)
    }

    fn truncate(&mut self, size: u64) -> Result<(), ()> {
        self.file.truncate(size)
    }

    fn size(&mut self) -> Result<u64, ()> {
        self.file.size()
    }

    fn full_sync(&mut self) -> Result<(), ()> {
        self.file.full_sync()
    }

    fn lock_to(
        &mut self,
        level: HandleBoundSqliteAbiLockLevel,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()> {
        self.file.lock_to(level)
    }

    fn unlock_to(&mut self, level: HandleBoundSqliteAbiUnlockLevel) -> Result<(), ()> {
        self.file.unlock_to(level)
    }

    fn check_reserved_lock(&mut self) -> Result<bool, ()> {
        self.file.check_reserved_lock()
    }

    fn shm_map(
        &mut self,
        region: u32,
        region_size: NonZeroU32,
        extend: bool,
    ) -> Result<HandleBoundSqliteAbiShmMap, ()> {
        self.file.shm_map(region, region_size, extend)
    }

    fn shm_lock(
        &mut self,
        first: u8,
        count: NonZeroU8,
        action: HandleBoundSqliteAbiShmLockAction,
    ) -> Result<HandleBoundSqliteAbiAttempt, ()> {
        self.file.shm_lock(first, count, action)
    }

    fn shm_barrier(&mut self) -> Result<(), ()> {
        self.file.shm_barrier()
    }

    fn shm_unmap(&mut self, delete: bool) -> Result<(), ()> {
        self.file.shm_unmap(delete)
    }

    fn close(self: Box<Self>) -> Result<(), ()> {
        let Self {
            file,
            owner,
            route,
            role,
            wal_runtime: _,
            close_faults,
        } = *self;
        if role != ManagedSqliteLogicalFileRole::Main {
            return file.close();
        }
        let close_faults = close_faults.ok_or(())?;
        close_faults.observe_registry_lifecycle_stage(
            ManagedSqliteRegistryLifecycleStage::RawCloseEntered,
        )?;
        owner.begin_connection_close(route).map_err(drop)?;
        let callback = file.close_with_callback_receipt()?;
        let outstanding_sidecar = match close_faults.take_connection_observation_sidecar() {
            Ok(Some(file)) => {
                let lease = match owner.claim_connection_observation_sidecar(
                    route,
                    ManagedSqliteLogicalFileRole::Journal,
                ) {
                    Ok(lease) => lease,
                    Err(_) => {
                        let _ = owner.retain_terminal_custody(
                            route,
                            ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                            (callback, file),
                        );
                        return Err(());
                    }
                };
                let sidecar = match ManagedSqliteRegistryPinnedFile::bind_sidecar(
                    owner, route, file, lease,
                ) {
                    Ok(sidecar) => sidecar,
                    Err(_) => {
                        let _ = owner.retain_terminal_custody(
                            route,
                            ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                            callback,
                        );
                        return Err(());
                    }
                };
                if close_faults
                    .observe_registry_lifecycle_stage(
                        ManagedSqliteRegistryLifecycleStage::OutstandingSidecarRetained,
                    )
                    .is_err()
                {
                    let _ = owner.retain_terminal_custody(
                        route,
                        ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                        (callback, sidecar),
                    );
                    return Err(());
                }
                Some(sidecar)
            }
            Ok(None) => None,
            Err(()) => {
                let _ = owner.retain_terminal_custody(
                    route,
                    ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                    callback,
                );
                return Err(());
            }
        };
        if close_faults
            .before(ManagedSqliteRegistryCloseLifecyclePhase::ConnectionObservation)
            .unwrap_or(true)
        {
            match outstanding_sidecar {
                Some(sidecar) => {
                    let _ = owner.retain_terminal_custody(
                        route,
                        ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                        (callback, sidecar),
                    );
                }
                None => {
                    let _ = owner.retain_terminal_custody(
                        route,
                        ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                        callback,
                    );
                }
            }
            return Err(());
        }
        if close_faults
            .observe_registry_lifecycle_stage(
                ManagedSqliteRegistryLifecycleStage::ConnectionObservationAttempt,
            )
            .is_err()
        {
            match outstanding_sidecar {
                Some(sidecar) => {
                    let _ = owner.retain_terminal_custody(
                        route,
                        ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                        (callback, sidecar),
                    );
                }
                None => {
                    let _ = owner.retain_terminal_custody(
                        route,
                        ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                        callback,
                    );
                }
            }
            return Err(());
        }
        let observed = match outstanding_sidecar {
            Some(sidecar) => match owner
                .observe_connection_closed_after_callback_with_sidecar(route, callback, sidecar)
            {
                Ok((observed, sidecar)) => {
                    let _ = owner.retain_terminal_custody(
                        route,
                        ManagedSqliteRegistryTerminalReason::StateInvariantViolated,
                        (observed, sidecar),
                    );
                    return Err(());
                }
                Err(_rejection) => {
                    close_faults.native_failure(
                        ManagedSqliteRegistryCloseLifecyclePhase::ConnectionObservation,
                    );
                    return Err(());
                }
            },
            None => match owner.observe_connection_closed_after_callback(route, callback) {
                Ok(observed) => observed,
                Err(_rejection) => {
                    close_faults.native_failure(
                        ManagedSqliteRegistryCloseLifecyclePhase::ConnectionObservation,
                    );
                    return Err(());
                }
            },
        };
        if close_faults
            .observe_registry_lifecycle_stage(
                ManagedSqliteRegistryLifecycleStage::ConnectionObservationSucceeded,
            )
            .is_err()
        {
            let _ = owner.retain_terminal_custody(
                route,
                ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                observed,
            );
            return Err(());
        }
        if close_faults
            .after_success(ManagedSqliteRegistryCloseLifecyclePhase::ConnectionObservation)
            .unwrap_or(true)
        {
            let _ = owner.retain_terminal_custody(
                route,
                ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                observed,
            );
            return Err(());
        }
        if close_faults
            .before(ManagedSqliteRegistryCloseLifecyclePhase::RouteRetirement)
            .unwrap_or(true)
        {
            let _ = owner.retain_terminal_custody(
                route,
                ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                observed,
            );
            return Err(());
        }
        if close_faults
            .observe_registry_lifecycle_stage(
                ManagedSqliteRegistryLifecycleStage::RouteRetirementAttempt,
            )
            .is_err()
        {
            let _ = owner.retain_terminal_custody(
                route,
                ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                observed,
            );
            return Err(());
        }
        let reject_retirement = match close_faults
            .claim_native_failure_gate(ManagedSqliteRegistryCloseLifecyclePhase::RouteRetirement)
        {
            Ok(reject) => reject,
            Err(()) => {
                let _ = owner.retain_terminal_custody(
                    route,
                    ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                    observed,
                );
                return Err(());
            }
        };
        if reject_retirement
            && owner
                .arm_route_retirement_native_rejection(route, &observed)
                .is_err()
        {
            let _ = owner.retain_terminal_custody(
                route,
                ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                observed,
            );
            return Err(());
        }
        let retirement = match owner.retire_closed_after_observation(route, observed) {
            Ok(retirement) => retirement,
            Err(_rejection) => {
                close_faults
                    .native_failure(ManagedSqliteRegistryCloseLifecyclePhase::RouteRetirement);
                return Err(());
            }
        };
        if close_faults
            .observe_registry_lifecycle_stage(
                ManagedSqliteRegistryLifecycleStage::RouteRetirementSucceeded,
            )
            .is_err()
        {
            close_faults.retain_retirement_failure(retirement);
            return Err(());
        }
        if close_faults
            .after_success(ManagedSqliteRegistryCloseLifecyclePhase::RouteRetirement)
            .unwrap_or(true)
        {
            close_faults.retain_retirement_failure(retirement);
            return Err(());
        }
        close_faults.publish_retirement(retirement)?;
        Ok(())
    }
}
