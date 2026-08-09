use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    num::NonZeroU64,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
};

use super::super::{
    PinnedManagedSqliteFile, PinnedManagedSqliteMainFile, PinnedManagedSqliteNamespace,
};
use super::{
    mapping::ManagedSqliteShmRegionMapping,
    types::{
        ManagedSqliteShmBudget, ManagedSqliteShmFailure, ManagedSqliteShmFailureClass,
        ManagedSqliteShmFailurePhase,
    },
};

static NEXT_SHM_RUNTIME_GENERATION: AtomicU64 = AtomicU64::new(1);
static SHM_DOMAINS: OnceLock<
    Mutex<HashMap<ManagedSqliteShmDomainKey, ManagedSqliteShmDomainEntry>>,
> = OnceLock::new();

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub(super) struct ManagedSqliteShmDomainKey {
    volume_serial: u64,
    directory_file_id: [u8; 16],
}

struct ManagedSqliteShmDomainEntry {
    terminal: bool,
}

#[must_use = "dropping the WAL runtime releases its exact namespace and SHM coordinator"]
pub(crate) struct PinnedManagedSqliteWalRuntime {
    pub(super) coordinator: Arc<ManagedSqliteShmCoordinator>,
}

pub(super) struct ManagedSqliteShmCoordinator {
    pub(super) state: Mutex<ManagedSqliteShmCoordinatorState>,
    pub(super) namespace: PinnedManagedSqliteNamespace,
    pub(super) domain_key: ManagedSqliteShmDomainKey,
    pub(super) generation: NonZeroU64,
    pub(super) budget: ManagedSqliteShmBudget,
}

pub(super) struct ManagedSqliteShmCoordinatorState {
    pub(super) next_connection_id: u64,
    pub(super) connections: BTreeMap<u64, ManagedSqliteShmConnectionState>,
    pub(super) main_identity_digest: Option<String>,
    pub(super) node: Option<ManagedSqliteShmNode>,
    pub(super) poisoned: Option<ManagedSqliteShmPoison>,
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct ManagedSqliteShmConnectionState {
    pub(super) shared_mask: u8,
    pub(super) exclusive_mask: u8,
    pub(super) exclusive_ranges: [u8; 8],
}

pub(super) struct ManagedSqliteShmNode {
    pub(super) regions: Vec<ManagedSqliteShmRegionMapping>,
    pub(super) file: PinnedManagedSqliteFile,
    pub(super) dms: ManagedSqliteShmDmsCustody,
    pub(super) initialization_mutated: bool,
    pub(super) region_size: Option<std::num::NonZeroU32>,
    pub(super) mapped_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedSqliteShmDmsCustody {
    Shared,
    ExclusiveOutcomeUncertain,
    Released,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ManagedSqliteShmPoison {
    pub(super) phase: ManagedSqliteShmFailurePhase,
    pub(super) mutation_may_have_occurred: bool,
    pub(super) lock_outcome_uncertain: bool,
}

#[must_use = "dropping a SHM connection invokes only best-effort teardown"]
pub(crate) struct PinnedManagedSqliteShmConnection {
    pub(super) coordinator: Arc<ManagedSqliteShmCoordinator>,
    pub(super) connection_id: u64,
    pub(super) active: bool,
}

/// Future VFS file custody: the SHM connection is declared first so fallback field destruction
/// detaches it before the main file releases database-byte locks.
#[must_use = "dropping a WAL main file releases SHM custody before its main-file locks"]
pub(crate) struct PinnedManagedSqliteWalMainFile {
    pub(super) shm: Option<PinnedManagedSqliteShmConnection>,
    pub(super) main: PinnedManagedSqliteMainFile,
    pub(super) runtime_generation: NonZeroU64,
}

impl PinnedManagedSqliteNamespace {
    /// Consumes the only public namespace owner. Raw SHM open/delete operations remain private to
    /// the resulting coordinator, preventing a second in-process WAL lock domain.
    pub(crate) fn into_wal_runtime(
        self,
        budget: ManagedSqliteShmBudget,
    ) -> Result<PinnedManagedSqliteWalRuntime, ManagedSqliteShmFailure> {
        let generation = next_runtime_generation()?;
        let domain_key = ManagedSqliteShmDomainKey {
            volume_serial: self.inner.directory_identity.volume_serial,
            directory_file_id: self.inner.directory_identity.file_id,
        };
        let coordinator = Arc::new(ManagedSqliteShmCoordinator {
            state: Mutex::new(ManagedSqliteShmCoordinatorState {
                next_connection_id: 1,
                connections: BTreeMap::new(),
                main_identity_digest: None,
                node: None,
                poisoned: None,
            }),
            namespace: self,
            domain_key,
            generation,
            budget,
        });
        register_shm_domain(&coordinator)?;
        Ok(PinnedManagedSqliteWalRuntime { coordinator })
    }
}

impl PinnedManagedSqliteWalRuntime {
    /// Binds an exact pinned main-file identity to this coordinator. Every later main-file bind
    /// must present the same identity before it receives a local SHM connection id.
    pub(super) fn bind_main_file(
        &self,
        main: PinnedManagedSqliteMainFile,
    ) -> Result<PinnedManagedSqliteWalMainFile, ManagedSqliteShmFailure> {
        if !Arc::ptr_eq(&main.file.namespace, &self.coordinator.namespace.inner) {
            return Err(ManagedSqliteShmFailure::code(
                ManagedSqliteShmFailurePhase::RequestValidation,
                ManagedSqliteShmFailureClass::ProtocolViolation,
                "NODE_MANAGED_SQLITE_SHM_MAIN_NAMESPACE_MISMATCH",
            ));
        }
        let identity = main.identity_digest().to_owned();
        let shm = self.coordinator.attach(&identity)?;
        Ok(PinnedManagedSqliteWalMainFile {
            shm: Some(shm),
            main,
            runtime_generation: self.coordinator.generation,
        })
    }

    pub(super) fn generation(&self) -> NonZeroU64 {
        self.coordinator.generation
    }
}

impl ManagedSqliteShmCoordinator {
    fn attach(
        self: &Arc<Self>,
        main_identity_digest: &str,
    ) -> Result<PinnedManagedSqliteShmConnection, ManagedSqliteShmFailure> {
        let mut state = self.state.lock().map_err(|_| self.poisoned_failure())?;
        if let Some(poison) = state.poisoned {
            return Err(poison.failure());
        }
        match state.main_identity_digest.as_deref() {
            Some(expected) if expected != main_identity_digest => {
                return Err(ManagedSqliteShmFailure::code(
                    ManagedSqliteShmFailurePhase::RequestValidation,
                    ManagedSqliteShmFailureClass::ProtocolViolation,
                    "NODE_MANAGED_SQLITE_SHM_MAIN_IDENTITY_MISMATCH",
                ));
            }
            None => state.main_identity_digest = Some(main_identity_digest.to_owned()),
            Some(_) => {}
        }
        let connection_id = state.next_connection_id;
        let Some(next_connection_id) = connection_id.checked_add(1) else {
            self.mark_poisoned(&mut state, ManagedSqliteShmFailurePhase::Gate, false, false);
            return Err(ManagedSqliteShmFailure::poisoned_code(
                ManagedSqliteShmFailurePhase::Gate,
                "NODE_MANAGED_SQLITE_SHM_CONNECTION_ID_EXHAUSTED",
                false,
                false,
            ));
        };
        state.next_connection_id = next_connection_id;
        state
            .connections
            .insert(connection_id, ManagedSqliteShmConnectionState::default());
        Ok(PinnedManagedSqliteShmConnection {
            coordinator: Arc::clone(self),
            connection_id,
            active: true,
        })
    }

    pub(super) fn poisoned_failure(&self) -> ManagedSqliteShmFailure {
        self.mark_domain_terminal();
        ManagedSqliteShmFailure::poisoned_code(
            ManagedSqliteShmFailurePhase::Gate,
            "NODE_MANAGED_SQLITE_SHM_COORDINATOR_POISONED",
            true,
            true,
        )
    }

    pub(super) fn mark_poisoned(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        phase: ManagedSqliteShmFailurePhase,
        mutation_may_have_occurred: bool,
        lock_outcome_uncertain: bool,
    ) {
        state.poisoned.get_or_insert(ManagedSqliteShmPoison {
            phase,
            mutation_may_have_occurred,
            lock_outcome_uncertain,
        });
        self.mark_domain_terminal();
    }

    pub(super) fn mark_domain_terminal(&self) {
        let registry = SHM_DOMAINS.get_or_init(|| Mutex::new(HashMap::new()));
        let Ok(mut registry) = registry.lock() else {
            return;
        };
        registry
            .entry(self.domain_key)
            .and_modify(|entry| entry.terminal = true)
            .or_insert_with(|| ManagedSqliteShmDomainEntry { terminal: true });
    }
}

impl ManagedSqliteShmPoison {
    pub(super) fn failure(self) -> ManagedSqliteShmFailure {
        ManagedSqliteShmFailure::poisoned_code(
            self.phase,
            if self.lock_outcome_uncertain {
                "NODE_MANAGED_SQLITE_SHM_LOCK_OUTCOME_UNCERTAIN"
            } else {
                "NODE_MANAGED_SQLITE_SHM_CUSTODY_QUARANTINED"
            },
            self.mutation_may_have_occurred,
            self.lock_outcome_uncertain,
        )
    }
}

impl PinnedManagedSqliteWalMainFile {
    pub(super) fn main_mut(&mut self) -> &mut PinnedManagedSqliteMainFile {
        &mut self.main
    }

    pub(super) fn shm_mut(&mut self) -> Option<&mut PinnedManagedSqliteShmConnection> {
        self.shm.as_mut()
    }
}

impl Drop for PinnedManagedSqliteShmConnection {
    fn drop(&mut self) {
        if self.active {
            self.coordinator
                .best_effort_drop_connection(self.connection_id);
            self.active = false;
        }
    }
}

impl Drop for ManagedSqliteShmCoordinator {
    fn drop(&mut self) {
        let terminal = match self.state.get_mut() {
            Ok(state) => {
                state.poisoned.is_some() || state.node.is_some() || !state.connections.is_empty()
            }
            Err(_) => true,
        };
        if terminal {
            self.mark_domain_terminal();
        }
    }
}

impl fmt::Debug for PinnedManagedSqliteWalRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedManagedSqliteWalRuntime")
            .field("namespace", &"<retained>")
            .field("generation", &"<process-local>")
            .finish()
    }
}

impl fmt::Debug for PinnedManagedSqliteShmConnection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedManagedSqliteShmConnection")
            .field("connection", &"<process-local>")
            .field("active", &self.active)
            .finish()
    }
}

fn next_runtime_generation() -> Result<NonZeroU64, ManagedSqliteShmFailure> {
    let value = NEXT_SHM_RUNTIME_GENERATION
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| {
            ManagedSqliteShmFailure::poisoned_code(
                ManagedSqliteShmFailurePhase::Gate,
                "NODE_MANAGED_SQLITE_SHM_RUNTIME_GENERATION_EXHAUSTED",
                false,
                false,
            )
        })?;
    NonZeroU64::new(value).ok_or_else(|| {
        ManagedSqliteShmFailure::poisoned_code(
            ManagedSqliteShmFailurePhase::Gate,
            "NODE_MANAGED_SQLITE_SHM_RUNTIME_GENERATION_ZERO",
            false,
            false,
        )
    })
}

fn register_shm_domain(
    coordinator: &Arc<ManagedSqliteShmCoordinator>,
) -> Result<(), ManagedSqliteShmFailure> {
    let registry = SHM_DOMAINS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut registry = registry.lock().map_err(|_| {
        ManagedSqliteShmFailure::poisoned_code(
            ManagedSqliteShmFailurePhase::Gate,
            "NODE_MANAGED_SQLITE_SHM_DOMAIN_REGISTRY_POISONED",
            false,
            false,
        )
    })?;
    if let Some(existing) = registry.get(&coordinator.domain_key) {
        if existing.terminal {
            return Err(ManagedSqliteShmFailure::poisoned_code(
                ManagedSqliteShmFailurePhase::Gate,
                "NODE_MANAGED_SQLITE_SHM_DOMAIN_TERMINAL",
                false,
                false,
            ));
        }
        return Err(ManagedSqliteShmFailure::code(
            ManagedSqliteShmFailurePhase::Gate,
            ManagedSqliteShmFailureClass::BusyNoMutation,
            "NODE_MANAGED_SQLITE_SHM_DOMAIN_ALREADY_ISSUED",
        ));
    }
    registry.insert(
        coordinator.domain_key,
        ManagedSqliteShmDomainEntry { terminal: false },
    );
    Ok(())
}
