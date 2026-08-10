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
    fn new(registration_id: u64) -> Self {
        Self {
            prefix: registration_id.to_be_bytes(),
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
    pub(super) fn pin(root: &Path, registration_id: u64) -> anyhow::Result<Self> {
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
                by_name: Mutex::new(HashMap::new()),
                live_routes: AtomicUsize::new(0),
                next_route_ordinal: AtomicU64::new(1),
            }),
            runtime,
        })
    }
}

pub(super) struct ManagedTestVfsRouteEntry {
    route: Arc<TestRoute>,
    ordinal: ManagedTestRouteOrdinal,
    main_name: CString,
    exact_names: [Vec<u8>; 3],
    custody_drops: Arc<AtomicUsize>,
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
}

pub(super) struct ManagedTestVfsRouteCollection {
    owner: &'static TestProcessOwner,
    by_name: Mutex<HashMap<Vec<u8>, ManagedTestVfsResolvedRoute>>,
    live_routes: AtomicUsize,
    next_route_ordinal: AtomicU64,
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
            ordinal,
            main_name,
            exact_names: [main, journal, wal],
            custody_drops,
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
