use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard, OnceLock, Weak},
};

use super::ManagedSqliteObservedLock;

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct ManagedSqliteLockDomainKey {
    volume_serial: u64,
    file_id: [u8; 16],
}

#[derive(Debug, Default)]
pub(super) struct ManagedSqliteHeldLocks {
    pub(super) shared: bool,
    pub(super) reserved: bool,
    pub(super) pending: bool,
    pub(super) exclusive: bool,
    pub(super) terminal: bool,
}

impl ManagedSqliteHeldLocks {
    pub(super) fn level(&self) -> ManagedSqliteObservedLock {
        if self.exclusive {
            ManagedSqliteObservedLock::Exclusive
        } else if self.pending {
            ManagedSqliteObservedLock::Pending
        } else if self.reserved {
            ManagedSqliteObservedLock::Reserved
        } else if self.shared {
            ManagedSqliteObservedLock::Shared
        } else {
            ManagedSqliteObservedLock::None
        }
    }

    fn is_writer(&self) -> bool {
        self.reserved || self.pending || self.exclusive
    }
}

const RETIRED_TERMINAL_LOCKS: ManagedSqliteHeldLocks = ManagedSqliteHeldLocks {
    shared: false,
    reserved: false,
    pending: false,
    exclusive: false,
    terminal: true,
};

struct ManagedSqliteLockDomain {
    state: Mutex<ManagedSqliteLockDomainState>,
}

#[derive(Default)]
struct ManagedSqliteLockDomainState {
    terminal: bool,
    next_owner_id: u64,
    owners: HashMap<u64, ManagedSqliteHeldLocks>,
}

pub(super) struct ManagedSqliteLockOwner {
    domain: Arc<ManagedSqliteLockDomain>,
    owner_id: u64,
}

pub(super) struct ManagedSqliteLockDomainGuard<'a> {
    state: MutexGuard<'a, ManagedSqliteLockDomainState>,
    owner_id: u64,
}

static SQLITE_LOCK_DOMAINS: OnceLock<
    Mutex<HashMap<ManagedSqliteLockDomainKey, Weak<ManagedSqliteLockDomain>>>,
> = OnceLock::new();

pub(super) fn register_lock_owner(
    volume_serial: u64,
    file_id: [u8; 16],
) -> std::io::Result<ManagedSqliteLockOwner> {
    let key = ManagedSqliteLockDomainKey {
        volume_serial,
        file_id,
    };
    let registry = SQLITE_LOCK_DOMAINS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut registry = registry
        .lock()
        .map_err(|_| std::io::Error::other("NODE_MANAGED_SQLITE_LOCK_REGISTRY_POISONED"))?;
    registry.retain(|_, domain| domain.upgrade().is_some());
    let domain = registry
        .get(&key)
        .and_then(Weak::upgrade)
        .unwrap_or_else(|| {
            let domain = Arc::new(ManagedSqliteLockDomain {
                state: Mutex::new(ManagedSqliteLockDomainState::default()),
            });
            registry.insert(key, Arc::downgrade(&domain));
            domain
        });
    let mut state = domain
        .state
        .lock()
        .map_err(|_| std::io::Error::other("NODE_MANAGED_SQLITE_LOCK_DOMAIN_POISONED"))?;
    if state.terminal {
        return Err(std::io::Error::other(
            "NODE_MANAGED_SQLITE_LOCK_DOMAIN_TERMINAL",
        ));
    }
    let owner_id = state
        .next_owner_id
        .checked_add(1)
        .filter(|owner_id| *owner_id != 0)
        .ok_or_else(|| std::io::Error::other("NODE_MANAGED_SQLITE_LOCK_OWNER_EXHAUSTED"))?;
    state.next_owner_id = owner_id;
    state
        .owners
        .insert(owner_id, ManagedSqliteHeldLocks::default());
    drop(state);
    drop(registry);
    Ok(ManagedSqliteLockOwner { domain, owner_id })
}

impl ManagedSqliteLockOwner {
    pub(super) fn lock(&self) -> std::io::Result<ManagedSqliteLockDomainGuard<'_>> {
        let state = self
            .domain
            .state
            .lock()
            .map_err(|_| std::io::Error::other("NODE_MANAGED_SQLITE_LOCK_DOMAIN_POISONED"))?;
        if !state.owners.contains_key(&self.owner_id) {
            return Err(std::io::Error::other(
                "NODE_MANAGED_SQLITE_LOCK_OWNER_RETIRED",
            ));
        }
        Ok(ManagedSqliteLockDomainGuard {
            state,
            owner_id: self.owner_id,
        })
    }
}

impl ManagedSqliteLockDomainGuard<'_> {
    pub(super) fn held(&self) -> &ManagedSqliteHeldLocks {
        self.state
            .owners
            .get(&self.owner_id)
            .unwrap_or(&RETIRED_TERMINAL_LOCKS)
    }

    pub(super) fn held_mut(&mut self) -> &mut ManagedSqliteHeldLocks {
        if !self.state.owners.contains_key(&self.owner_id) {
            self.state.terminal = true;
        }
        self.state
            .owners
            .entry(self.owner_id)
            .or_insert(RETIRED_TERMINAL_LOCKS)
    }

    pub(super) fn is_terminal(&self) -> bool {
        self.state.terminal || self.held().terminal
    }

    pub(super) fn another_owner_is_writer(&self) -> bool {
        self.state
            .owners
            .iter()
            .any(|(owner_id, held)| *owner_id != self.owner_id && held.is_writer())
    }

    pub(super) fn another_owner_has_pending_or_exclusive(&self) -> bool {
        self.state
            .owners
            .iter()
            .any(|(owner_id, held)| *owner_id != self.owner_id && (held.pending || held.exclusive))
    }

    pub(super) fn another_owner_holds_lock(&self) -> bool {
        self.state
            .owners
            .iter()
            .any(|(owner_id, held)| *owner_id != self.owner_id && held.level().rank() != 0)
    }

    pub(super) fn poison(&mut self) {
        self.state.terminal = true;
        self.held_mut().terminal = true;
    }
}

impl Drop for ManagedSqliteLockOwner {
    fn drop(&mut self) {
        if let Ok(mut state) = self.domain.state.lock() {
            state.owners.remove(&self.owner_id);
        }
    }
}
