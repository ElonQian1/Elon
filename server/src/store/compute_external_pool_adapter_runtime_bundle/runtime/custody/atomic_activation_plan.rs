//! Connection-local, one-shot custody for the V277 atomic activation write plan.

use std::{
    collections::HashMap,
    marker::PhantomData,
    rc::Rc,
    sync::{Arc, Mutex, OnceLock, Weak},
};

use anyhow::{anyhow, bail, ensure, Result};
use rusqlite::{
    functions::{Context, FunctionFlags},
    types::Value,
    Connection,
};

mod fingerprint;

use fingerprint::{PendingColumnFingerprint, PendingWriteFingerprint};

const PENDING_PLAN_MATCHES: &str =
    "elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches";
const EXPECTED_WRITE_COUNT: usize = 15;
const EXPECTED_CAPABILITY_COUNT: usize = 6;

static CONNECTION_CUSTODY: OnceLock<Mutex<HashMap<usize, Weak<Mutex<PendingPlanRegistry>>>>> =
    OnceLock::new();

/// The ten trigger call shapes covered by one V277 plan. Route capability is consumed six times.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(in crate::store) enum ExternalPoolAdapterAtomicActivationPendingWriteKind {
    ProviderUpdate,
    ProviderVersion,
    ProjectionAdapter,
    ProjectionAdapterVersion,
    ServiceActorAuthorization,
    RouteCredential,
    RouteAuthorization,
    RouteCapability,
    RouteSeal,
    ActivationReceipt,
}

/// One expected trigger call. Values are converted immediately to type-preserving fingerprints.
pub(in crate::store) struct ExternalPoolAdapterAtomicActivationPendingWrite {
    fingerprint: PendingWriteFingerprint,
}

/// Complete one-shot plan. It is intentionally non-Clone, non-Serde, !Send, and !Sync.
pub(in crate::store) struct ExternalPoolAdapterAtomicActivationPendingPlan {
    writes: Vec<PendingWriteFingerprint>,
    _same_thread: PhantomData<Rc<()>>,
}

/// RAII registration on exactly one SQLite connection. Drop always clears an unpromoted plan.
pub(in crate::store) struct ExternalPoolAdapterAtomicActivationPendingPlanGuard {
    registry: Arc<Mutex<PendingPlanRegistry>>,
    connection_key: usize,
    generation: u64,
    armed: bool,
    _same_thread: PhantomData<Rc<()>>,
}

struct PendingPlanRegistry {
    next_generation: u64,
    active: Option<RegisteredPendingPlan>,
}

struct RegisteredPendingPlan {
    generation: u64,
    writes: Vec<PendingWriteFingerprint>,
    next_index: usize,
}

impl ExternalPoolAdapterAtomicActivationPendingWriteKind {
    fn from_sql_kind(value: &str) -> Option<Self> {
        Some(match value {
            "provider_update" => Self::ProviderUpdate,
            "provider_version" => Self::ProviderVersion,
            "projection_adapter" => Self::ProjectionAdapter,
            "projection_adapter_version" => Self::ProjectionAdapterVersion,
            "service_actor_authorization" => Self::ServiceActorAuthorization,
            "route_credential" => Self::RouteCredential,
            "route_authorization" => Self::RouteAuthorization,
            "route_capability" => Self::RouteCapability,
            "route_seal" => Self::RouteSeal,
            "activation_receipt" => Self::ActivationReceipt,
            _ => return None,
        })
    }

    fn table_and_event(self) -> (&'static str, &'static str) {
        match self {
            Self::ProviderUpdate => ("compute_providers", "UPDATE"),
            Self::ProviderVersion => ("compute_provider_versions", "INSERT"),
            Self::ProjectionAdapter => ("compute_route_adapters", "INSERT"),
            Self::ProjectionAdapterVersion => ("compute_route_adapter_versions", "INSERT"),
            Self::ServiceActorAuthorization => ("compute_service_actor_authorizations", "INSERT"),
            Self::RouteCredential => ("compute_route_credential_versions", "INSERT"),
            Self::RouteAuthorization => ("compute_route_authorization_receipts", "INSERT"),
            Self::RouteCapability => ("compute_route_authorization_capabilities", "INSERT"),
            Self::RouteSeal => ("compute_route_authorization_seals", "INSERT"),
            Self::ActivationReceipt => (
                "compute_external_pool_adapter_atomic_activation_receipts",
                "INSERT",
            ),
        }
    }

    fn arity(self) -> usize {
        match self {
            Self::ProviderUpdate => 24,
            Self::ProviderVersion => 5,
            Self::ProjectionAdapter => 6,
            Self::ProjectionAdapterVersion => 4,
            Self::ServiceActorAuthorization => 3,
            Self::RouteCredential | Self::RouteAuthorization | Self::RouteCapability => 4,
            Self::RouteSeal => 3,
            Self::ActivationReceipt => 4,
        }
    }
}

impl ExternalPoolAdapterAtomicActivationPendingWrite {
    pub(in crate::store) fn new(
        kind: ExternalPoolAdapterAtomicActivationPendingWriteKind,
        values: Vec<Value>,
    ) -> Result<Self> {
        ensure!(
            values.len() == kind.arity(),
            "V277 pending write has the wrong trigger arity"
        );
        let (table, event) = kind.table_and_event();
        let columns = values
            .into_iter()
            .enumerate()
            .map(|(ordinal, value)| PendingColumnFingerprint::from_value(ordinal, value))
            .collect();
        Ok(Self {
            fingerprint: PendingWriteFingerprint {
                kind,
                table,
                event,
                columns,
            },
        })
    }
}

impl ExternalPoolAdapterAtomicActivationPendingPlan {
    pub(in crate::store) fn new(
        writes: Vec<ExternalPoolAdapterAtomicActivationPendingWrite>,
    ) -> Result<Self> {
        ensure!(
            writes.len() == EXPECTED_WRITE_COUNT,
            "V277 pending plan must bind exactly fifteen trigger calls"
        );
        for kind in [
            ExternalPoolAdapterAtomicActivationPendingWriteKind::ProviderUpdate,
            ExternalPoolAdapterAtomicActivationPendingWriteKind::ProviderVersion,
            ExternalPoolAdapterAtomicActivationPendingWriteKind::ProjectionAdapter,
            ExternalPoolAdapterAtomicActivationPendingWriteKind::ProjectionAdapterVersion,
            ExternalPoolAdapterAtomicActivationPendingWriteKind::ServiceActorAuthorization,
            ExternalPoolAdapterAtomicActivationPendingWriteKind::RouteCredential,
            ExternalPoolAdapterAtomicActivationPendingWriteKind::RouteAuthorization,
            ExternalPoolAdapterAtomicActivationPendingWriteKind::RouteSeal,
            ExternalPoolAdapterAtomicActivationPendingWriteKind::ActivationReceipt,
        ] {
            ensure!(
                writes
                    .iter()
                    .filter(|write| write.fingerprint.kind == kind)
                    .count()
                    == 1,
                "V277 pending plan trigger inventory is incomplete"
            );
        }
        ensure!(
            writes
                .iter()
                .filter(|write| {
                    write.fingerprint.kind
                        == ExternalPoolAdapterAtomicActivationPendingWriteKind::RouteCapability
                })
                .count()
                == EXPECTED_CAPABILITY_COUNT,
            "V277 pending plan must bind exactly six route capabilities"
        );
        Ok(Self {
            writes: writes.into_iter().map(|write| write.fingerprint).collect(),
            _same_thread: PhantomData,
        })
    }
}

impl ExternalPoolAdapterAtomicActivationPendingPlanGuard {
    pub(in crate::store) fn ensure_fully_consumed(&self) -> Result<()> {
        let registry = self
            .registry
            .lock()
            .map_err(|_| anyhow!("V277 pending-plan registry lock was poisoned"))?;
        let active = registry
            .active
            .as_ref()
            .filter(|active| active.generation == self.generation)
            .ok_or_else(|| anyhow!("V277 pending plan is no longer registered"))?;
        ensure!(
            active.next_index == active.writes.len(),
            "V277 pending plan was not fully consumed"
        );
        Ok(())
    }

    pub(in crate::store) fn ensure_same_connection(&self, connection: &Connection) -> Result<()> {
        let connection_key = connection_key(connection);
        ensure!(
            self.connection_key == connection_key,
            "V277 postcommit readback changed SQLite connection"
        );
        let registered = {
            let custody = connection_custody()
                .lock()
                .map_err(|_| anyhow!("V277 connection-custody registry lock was poisoned"))?;
            custody
                .get(&connection_key)
                .and_then(Weak::upgrade)
                .ok_or_else(|| {
                    anyhow!("V277 pending-plan UDF is not registered on this connection")
                })?
        };
        ensure!(
            Arc::ptr_eq(&registered, &self.registry),
            "V277 postcommit readback changed SQLite connection registration"
        );
        Ok(())
    }

    pub(in crate::store) fn discard(mut self) -> Result<()> {
        self.clear()?;
        self.armed = false;
        Ok(())
    }

    fn clear(&self) -> Result<()> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| anyhow!("V277 pending-plan registry lock was poisoned"))?;
        if registry
            .active
            .as_ref()
            .is_some_and(|active| active.generation == self.generation)
        {
            registry.active = None;
        }
        Ok(())
    }
}

impl Drop for ExternalPoolAdapterAtomicActivationPendingPlanGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.clear();
        }
    }
}

impl PendingPlanRegistry {
    fn empty() -> Self {
        Self {
            next_generation: 0,
            active: None,
        }
    }

    fn matches(&mut self, context: &Context<'_>) -> bool {
        if context.len() < 1 {
            return false;
        }
        let Ok(sql_kind) = context.get_raw(0).as_str() else {
            return false;
        };
        let Some(kind) =
            ExternalPoolAdapterAtomicActivationPendingWriteKind::from_sql_kind(sql_kind)
        else {
            return false;
        };
        if context.len() != kind.arity() + 1 {
            return false;
        }
        let (table, event) = kind.table_and_event();
        let actual = PendingWriteFingerprint {
            kind,
            table,
            event,
            columns: (1..context.len())
                .map(|index| PendingColumnFingerprint::from_ref(index - 1, context.get_raw(index)))
                .collect(),
        };
        let Some(active) = self.active.as_mut() else {
            return false;
        };
        let Some(expected) = active.writes.get(active.next_index) else {
            return false;
        };
        if expected != &actual {
            return false;
        }
        active.next_index += 1;
        true
    }
}

pub(crate) fn register_external_pool_adapter_atomic_activation_pending_plan_udf(
    connection: &Connection,
) -> Result<()> {
    let registry = Arc::new(Mutex::new(PendingPlanRegistry::empty()));
    let udf_registry = Arc::clone(&registry);
    let flags = FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_INNOCUOUS;
    connection.create_scalar_function(PENDING_PLAN_MATCHES, -1, flags, move |context| {
        Ok(i64::from(
            udf_registry
                .lock()
                .map(|mut registry| registry.matches(context))
                .unwrap_or(false),
        ))
    })?;

    let mut custody = connection_custody()
        .lock()
        .map_err(|_| anyhow!("V277 connection-custody registry lock was poisoned"))?;
    custody.retain(|_, registry| registry.strong_count() != 0);
    custody.insert(connection_key(connection), Arc::downgrade(&registry));
    Ok(())
}

pub(in crate::store) fn install_external_pool_adapter_atomic_activation_pending_plan_on(
    connection: &Connection,
    plan: ExternalPoolAdapterAtomicActivationPendingPlan,
) -> Result<ExternalPoolAdapterAtomicActivationPendingPlanGuard> {
    let registry = {
        let custody = connection_custody()
            .lock()
            .map_err(|_| anyhow!("V277 connection-custody registry lock was poisoned"))?;
        custody
            .get(&connection_key(connection))
            .and_then(Weak::upgrade)
            .ok_or_else(|| anyhow!("V277 pending-plan UDF is not registered on this connection"))?
    };
    let generation = {
        let mut registry_guard = registry
            .lock()
            .map_err(|_| anyhow!("V277 pending-plan registry lock was poisoned"))?;
        if registry_guard.active.is_some() {
            bail!("this SQLite connection already has a V277 pending plan");
        }
        registry_guard.next_generation = registry_guard
            .next_generation
            .checked_add(1)
            .ok_or_else(|| anyhow!("V277 pending-plan generation exhausted"))?;
        let generation = registry_guard.next_generation;
        registry_guard.active = Some(RegisteredPendingPlan {
            generation,
            writes: plan.writes,
            next_index: 0,
        });
        generation
    };
    Ok(ExternalPoolAdapterAtomicActivationPendingPlanGuard {
        registry,
        connection_key: connection_key(connection),
        generation,
        armed: true,
        _same_thread: PhantomData,
    })
}

fn connection_custody() -> &'static Mutex<HashMap<usize, Weak<Mutex<PendingPlanRegistry>>>> {
    CONNECTION_CUSTODY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn connection_key(connection: &Connection) -> usize {
    // SAFETY: the handle is observed only as an identity while `connection` is borrowed. The map
    // stores a Weak registry, and every new registration at a reused SQLite address replaces it.
    unsafe { connection.handle() as usize }
}
