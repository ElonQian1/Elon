//! Test-only ownership for one managed namespace, runtime and exact logical-name route index.

use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    fs,
    path::Path,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use anyhow::{anyhow, Context};

use super::*;
use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::{
        ManagedSqliteLogicalFileRole, ManagedSqliteRegistryNonceSource,
    },
    node_agent_managed_fs::{
        ManagedSqliteShmBudget, PinnedManagedRoot, PinnedManagedSqliteWalRuntime,
    },
};

pub(super) struct ManagedTestNonceSource {
    prefix: [u8; 8],
    next: AtomicU64,
}

impl ManagedTestNonceSource {
    fn new(registration_id: ManagedTestRegistrationId) -> Self {
        Self {
            prefix: registration_id.counter_value().to_be_bytes(),
            next: AtomicU64::new(1),
        }
    }
}

impl ManagedSqliteRegistryNonceSource for ManagedTestNonceSource {
    fn fill_nonce(&self, output: &mut [u8; 16]) -> Result<(), ()> {
        let ordinal = self
            .next
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                current.checked_add(1)
            })
            .map_err(|_| ())?;
        output[..8].copy_from_slice(&self.prefix);
        output[8..].copy_from_slice(&ordinal.to_be_bytes());
        Ok(())
    }
}

pub(super) struct ManagedTestSharedNamespace {
    pub(super) routes: Arc<ManagedTestVfsRouteCollection>,
    pub(super) runtime: Arc<PinnedManagedSqliteWalRuntime>,
}

impl ManagedTestSharedNamespace {
    pub(super) fn pin(
        root: &Path,
        registration_id: ManagedTestRegistrationId,
        lifecycle: Arc<ManagedTestLifecycleFaultController>,
    ) -> anyhow::Result<Self> {
        fs::create_dir_all(root.join("db"))
            .with_context(|| format!("create managed VFS fixture root at {}", root.display()))?;
        let pinned_root = PinnedManagedRoot::pin(root, &"b".repeat(64))
            .with_context(|| format!("pin managed VFS fixture root at {}", root.display()))?;
        let directory = pinned_root
            .pin_existing_directory(Path::new("db"))
            .context("pin managed VFS fixture database directory")?;
        let namespace = directory
            .into_sqlite_namespace()
            .map_err(|failure| anyhow!("bind managed SQLite namespace: {failure:?}"))?;
        let runtime = Arc::new(
            namespace
                .into_wal_runtime(ManagedSqliteShmBudget::authority_default())
                .map_err(|failure| anyhow!("bind managed SQLite WAL runtime: {failure:?}"))?,
        );
        drop(pinned_root);
        let owner = TestProcessOwner::leak(ManagedTestNonceSource::new(registration_id));
        Ok(Self {
            routes: Arc::new(ManagedTestVfsRouteCollection {
                owner,
                registration_id,
                by_name: Mutex::new(HashMap::new()),
                live_routes: AtomicUsize::new(0),
                next_route_ordinal: AtomicU64::new(1),
                lifecycle,
            }),
            runtime,
        })
    }
}

pub(super) struct ManagedTestVfsRouteEntry {
    route: Arc<TestRoute>,
    registration_id: ManagedTestRegistrationId,
    ordinal: ManagedTestRouteOrdinal,
    shm_faults: Arc<ManagedTestShmFaultPlanSlot>,
    main_name: CString,
    exact_names: [Vec<u8>; 3],
    custody_drops: Arc<AtomicUsize>,
    lifecycle: ManagedTestLifecycleFaultBinding,
}

impl ManagedTestVfsRouteEntry {
    pub(super) fn route(&self) -> &Arc<TestRoute> {
        &self.route
    }

    pub(super) fn ordinal(&self) -> ManagedTestRouteOrdinal {
        self.ordinal
    }

    pub(super) fn main_name(&self) -> &CStr {
        &self.main_name
    }

    pub(super) fn custody_drops(&self) -> usize {
        self.custody_drops.load(Ordering::SeqCst)
    }

    pub(super) fn lifecycle(&self) -> ManagedTestLifecycleFaultBinding {
        self.lifecycle.clone()
    }

    pub(super) fn install_shm_fault_script(
        &self,
        before_call: &[(
            crate::node_agent_managed_fs::ManagedSqliteShmFailurePhase,
            u32,
        )],
        after_success: &[(
            crate::node_agent_managed_fs::ManagedSqliteShmFailurePhase,
            u32,
            crate::node_agent_managed_fs::ManagedSqliteShmFailureClass,
        )],
    ) -> Result<(), &'static str> {
        let binding = self.main_shm_fault_binding()?;
        binding.install(before_call, after_success).map_err(|code| {
            let _ = self.route.retain_failure(code);
            code
        })
    }

    pub(super) fn pending_shm_fault_count(&self) -> Result<usize, &'static str> {
        let binding = self.main_shm_fault_binding()?;
        binding.pending_count().map_err(|code| {
            let _ = self.route.retain_failure(code);
            code
        })
    }

    pub(super) fn shm_fault_was_triggered(
        &self,
        phase: crate::node_agent_managed_fs::ManagedSqliteShmFailurePhase,
        occurrence: u32,
    ) -> Result<bool, &'static str> {
        let binding = self.main_shm_fault_binding()?;
        binding.was_triggered(phase, occurrence).map_err(|code| {
            let _ = self.route.retain_failure(code);
            code
        })
    }

    fn main_shm_fault_binding(&self) -> Result<ManagedTestShmFaultPlanBinding, &'static str> {
        self.shm_faults
            .binding(
                self.registration_id,
                self.ordinal,
                ManagedSqliteLogicalFileRole::Main,
            )
            .map_err(|code| {
                let _ = self.route.retain_failure(code);
                code
            })
    }
}

#[derive(Clone)]
pub(super) struct ManagedTestVfsResolvedRoute {
    entry: Arc<ManagedTestVfsRouteEntry>,
    role: ManagedSqliteLogicalFileRole,
}

impl ManagedTestVfsResolvedRoute {
    pub(super) fn route(&self) -> &Arc<TestRoute> {
        self.entry.route()
    }

    pub(super) fn role(&self) -> ManagedSqliteLogicalFileRole {
        self.role
    }

    pub(super) fn route_ordinal(&self) -> ManagedTestRouteOrdinal {
        self.entry.ordinal()
    }

    pub(super) fn shm_fault_binding(&self) -> Result<ManagedTestShmFaultPlanBinding, ()> {
        self.entry
            .shm_faults
            .binding(self.entry.registration_id, self.entry.ordinal, self.role)
            .map_err(drop)
    }

    pub(super) fn lifecycle(&self) -> ManagedTestLifecycleFaultBinding {
        self.entry.lifecycle()
    }
}

pub(super) struct ManagedTestVfsRouteCollection {
    owner: &'static TestProcessOwner,
    registration_id: ManagedTestRegistrationId,
    by_name: Mutex<HashMap<Vec<u8>, ManagedTestVfsResolvedRoute>>,
    live_routes: AtomicUsize,
    next_route_ordinal: AtomicU64,
    lifecycle: Arc<ManagedTestLifecycleFaultController>,
}

#[must_use = "logical route removal must be consumed before registration shutdown"]
pub(super) struct ManagedTestLogicalRouteRemovalReceipt {
    _registry: crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryRetirementReceipt,
    _removed: [ManagedTestVfsResolvedRoute; 3],
    _live_before: usize,
    _live_after: usize,
}

impl ManagedTestVfsRouteCollection {
    pub(super) fn register_route(
        &self,
        custody_drops: Arc<AtomicUsize>,
    ) -> anyhow::Result<Arc<ManagedTestVfsRouteEntry>> {
        let ordinal = self
            .next_route_ordinal
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                current.checked_add(1)
            })
            .map_err(|_| anyhow!("managed VFS route ordinal exhausted"))?;
        let ordinal = ManagedTestRouteOrdinal::from_counter(ordinal)
            .map_err(|()| anyhow!("managed VFS route ordinal was zero"))?;
        let shm_faults = ManagedTestShmFaultPlanSlot::new(self.registration_id, ordinal);
        let route = Arc::new(
            TestRoute::register(self.owner, TestCustody::tracked(Arc::clone(&custody_drops)))
                .map_err(|()| anyhow!("register managed VFS route"))?,
        );
        let main_name = match route.main_logical_name() {
            Ok(main_name) => main_name,
            Err(()) => {
                route.abort_unopened_for_test();
                return Err(anyhow!("read managed VFS logical main name"));
            }
        };
        let main = main_name.as_bytes().to_vec();
        let mut journal = main.clone();
        journal.extend_from_slice(b"-journal");
        let mut wal = main.clone();
        wal.extend_from_slice(b"-wal");
        let entry = Arc::new(ManagedTestVfsRouteEntry {
            route,
            registration_id: self.registration_id,
            ordinal,
            shm_faults,
            main_name,
            exact_names: [main, journal, wal],
            custody_drops,
            lifecycle: self.lifecycle.binding(ordinal),
        });

        let insert = self.insert_exact_names(&entry);
        if let Err(error) = insert {
            entry.route.abort_unopened_for_test();
            return Err(error);
        }
        Ok(entry)
    }

    pub(super) fn resolve(
        &self,
        candidate: Option<&[u8]>,
    ) -> Result<ManagedTestVfsResolvedRoute, ()> {
        let candidate = candidate.ok_or(())?;
        self.by_name
            .lock()
            .map_err(|_| ())?
            .get(candidate)
            .cloned()
            .ok_or(())
    }

    pub(super) fn retire_route(&self, entry: &Arc<ManagedTestVfsRouteEntry>) -> anyhow::Result<()> {
        if entry.custody_drops() != 1 {
            return Err(anyhow!(
                "managed VFS route custody was not retired exactly once"
            ));
        }
        let mut routes = self
            .by_name
            .lock()
            .map_err(|_| anyhow!("managed VFS logical route index poisoned"))?;
        for name in &entry.exact_names {
            let Some(candidate) = routes.get(name.as_slice()) else {
                return Err(anyhow!("managed VFS exact route name already retired"));
            };
            if !Arc::ptr_eq(&candidate.entry, entry) {
                return Err(anyhow!("managed VFS exact route identity mismatch"));
            }
        }
        let live_routes = self.live_routes.load(Ordering::SeqCst);
        let next_live_routes = live_routes
            .checked_sub(1)
            .ok_or_else(|| anyhow!("managed VFS live route count underflow"))?;
        for name in &entry.exact_names {
            routes.remove(name.as_slice());
        }
        self.live_routes.store(next_live_routes, Ordering::SeqCst);
        Ok(())
    }

    pub(super) fn retire_closed_route(
        &self,
        entry: &Arc<ManagedTestVfsRouteEntry>,
    ) -> anyhow::Result<ManagedTestLogicalRouteRemovalReceipt> {
        let lifecycle = entry.lifecycle();
        let retirement = lifecycle
            .claim_retirement()
            .map_err(|()| anyhow!("managed VFS registry retirement receipt missing"))?;
        if lifecycle
            .before(ManagedTestLifecycleFaultPhase::LogicalRouteRemoval)
            .unwrap_or(true)
        {
            lifecycle.retain_terminal(retirement);
            return Err(anyhow!("injected before managed VFS logical route removal"));
        }
        if entry.custody_drops() != 1 {
            lifecycle.native_failure(ManagedTestLifecycleFaultPhase::LogicalRouteRemoval);
            lifecycle.retain_terminal(retirement);
            return Err(anyhow!(
                "managed VFS route custody was not retired exactly once"
            ));
        }
        let mut routes = match self.by_name.lock() {
            Ok(routes) => routes,
            Err(_) => {
                lifecycle.native_failure(ManagedTestLifecycleFaultPhase::LogicalRouteRemoval);
                lifecycle.retain_terminal(retirement);
                return Err(anyhow!("managed VFS logical route index poisoned"));
            }
        };
        for name in &entry.exact_names {
            let Some(candidate) = routes.get(name.as_slice()) else {
                drop(routes);
                lifecycle.native_failure(ManagedTestLifecycleFaultPhase::LogicalRouteRemoval);
                lifecycle.retain_terminal(retirement);
                return Err(anyhow!("managed VFS exact route name already retired"));
            };
            if !Arc::ptr_eq(&candidate.entry, entry) {
                drop(routes);
                lifecycle.native_failure(ManagedTestLifecycleFaultPhase::LogicalRouteRemoval);
                lifecycle.retain_terminal(retirement);
                return Err(anyhow!("managed VFS exact route identity mismatch"));
            }
        }
        let live_before = self.live_routes.load(Ordering::SeqCst);
        let Some(live_after) = live_before.checked_sub(1) else {
            drop(routes);
            lifecycle.native_failure(ManagedTestLifecycleFaultPhase::LogicalRouteRemoval);
            lifecycle.retain_terminal(retirement);
            return Err(anyhow!("managed VFS live route count underflow"));
        };
        let removed = [
            routes
                .remove(entry.exact_names[0].as_slice())
                .expect("validated main route remains present"),
            routes
                .remove(entry.exact_names[1].as_slice())
                .expect("validated journal route remains present"),
            routes
                .remove(entry.exact_names[2].as_slice())
                .expect("validated WAL route remains present"),
        ];
        self.live_routes.store(live_after, Ordering::SeqCst);
        drop(routes);
        let receipt = ManagedTestLogicalRouteRemovalReceipt {
            _registry: retirement,
            _removed: removed,
            _live_before: live_before,
            _live_after: live_after,
        };
        if lifecycle
            .after_success(ManagedTestLifecycleFaultPhase::LogicalRouteRemoval)
            .unwrap_or(true)
        {
            lifecycle.retain_terminal(receipt);
            return Err(anyhow!("injected after managed VFS logical route removal"));
        }
        Ok(receipt)
    }

    pub(super) fn live_route_count(&self) -> anyhow::Result<usize> {
        let routes = self
            .by_name
            .lock()
            .map_err(|_| anyhow!("managed VFS logical route index poisoned"))?;
        let live_routes = self.live_routes.load(Ordering::SeqCst);
        let expected_names = live_routes
            .checked_mul(3)
            .ok_or_else(|| anyhow!("managed VFS logical route count overflow"))?;
        if routes.len() != expected_names {
            return Err(anyhow!("managed VFS logical route index count mismatch"));
        }
        Ok(live_routes)
    }

    fn insert_exact_names(&self, entry: &Arc<ManagedTestVfsRouteEntry>) -> anyhow::Result<()> {
        let mut routes = self
            .by_name
            .lock()
            .map_err(|_| anyhow!("managed VFS logical route index poisoned"))?;
        if entry
            .exact_names
            .iter()
            .any(|name| routes.contains_key(name.as_slice()))
        {
            return Err(anyhow!("managed VFS exact logical route collision"));
        }
        let live_routes = self.live_routes.load(Ordering::SeqCst);
        let next_live_routes = live_routes
            .checked_add(1)
            .ok_or_else(|| anyhow!("managed VFS live route count overflow"))?;
        for (name, role) in entry.exact_names.iter().zip([
            ManagedSqliteLogicalFileRole::Main,
            ManagedSqliteLogicalFileRole::Journal,
            ManagedSqliteLogicalFileRole::Wal,
        ]) {
            routes.insert(
                name.clone(),
                ManagedTestVfsResolvedRoute {
                    entry: Arc::clone(entry),
                    role,
                },
            );
        }
        self.live_routes.store(next_live_routes, Ordering::SeqCst);
        Ok(())
    }
}
