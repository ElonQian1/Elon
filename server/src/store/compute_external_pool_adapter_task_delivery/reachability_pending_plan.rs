//! Connection-local, ordered, one-shot custody for V278 task-delivery writes.

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

pub(crate) const REACHABILITY_PENDING_PLAN_MATCHES: &str =
    "elon_v278_external_pool_adapter_task_reachability_pending_plan_matches";
const MAX_ORDERED_WRITES: usize = 263;

static CONNECTION_CUSTODY: OnceLock<Mutex<HashMap<usize, Weak<Mutex<PendingPlanRegistry>>>>> =
    OnceLock::new();

#[derive(Clone, Copy, Eq, PartialEq)]
pub(in crate::store) enum ExternalPoolAdapterTaskReachabilityPendingWriteKind {
    StartSendAttempt,
    ExchangeAttempt,
    StartOutboxCas,
    ExchangeReceipt,
    ReconcilePoll,
    EventPoll,
    EventBatch,
    Event,
    ReconcilePollCas,
    EventPollCas,
    HistoricalAcceptedActor,
    HistoricalAcceptedLeaseAuthority,
    HistoricalAcceptedCommit,
    HistoricalAcceptedApplication,
}

pub(in crate::store) struct ExternalPoolAdapterTaskReachabilityPendingWrite {
    fingerprint: PendingWriteFingerprint,
}

/// Complete ordered plan. Deliberately non-Clone, non-Serde, !Send and !Sync.
pub(in crate::store) struct ExternalPoolAdapterTaskReachabilityPendingPlan {
    writes: Vec<PendingWriteFingerprint>,
    _same_thread: PhantomData<Rc<()>>,
}

/// Drop clears every unconsumed or unpromoted plan on the exact SQLite connection.
pub(in crate::store) struct ExternalPoolAdapterTaskReachabilityPendingPlanGuard {
    registry: Arc<Mutex<PendingPlanRegistry>>,
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

impl ExternalPoolAdapterTaskReachabilityPendingWriteKind {
    fn from_sql_kind(value: &str) -> Option<Self> {
        Some(match value {
            "start_send_attempt" => Self::StartSendAttempt,
            "exchange_attempt" => Self::ExchangeAttempt,
            "start_outbox_cas" => Self::StartOutboxCas,
            "exchange_receipt" => Self::ExchangeReceipt,
            "reconcile_poll" => Self::ReconcilePoll,
            "event_poll" => Self::EventPoll,
            "event_batch" => Self::EventBatch,
            "event" => Self::Event,
            "reconcile_poll_cas" => Self::ReconcilePollCas,
            "event_poll_cas" => Self::EventPollCas,
            "historical_accepted_actor" => Self::HistoricalAcceptedActor,
            "historical_accepted_lease_authority" => Self::HistoricalAcceptedLeaseAuthority,
            "historical_accepted_commit" => Self::HistoricalAcceptedCommit,
            "historical_accepted_application" => Self::HistoricalAcceptedApplication,
            _ => return None,
        })
    }
}

impl ExternalPoolAdapterTaskReachabilityPendingWrite {
    pub(in crate::store) fn new(
        kind: ExternalPoolAdapterTaskReachabilityPendingWriteKind,
        values: Vec<Value>,
    ) -> Result<Self> {
        ensure!(!values.is_empty(), "V278 pending write is empty");
        Ok(Self {
            fingerprint: PendingWriteFingerprint {
                kind,
                columns: values
                    .into_iter()
                    .enumerate()
                    .map(|(ordinal, value)| PendingColumnFingerprint::from_value(ordinal, value))
                    .collect(),
            },
        })
    }
}

impl ExternalPoolAdapterTaskReachabilityPendingPlan {
    pub(in crate::store) fn new(
        writes: Vec<ExternalPoolAdapterTaskReachabilityPendingWrite>,
    ) -> Result<Self> {
        ensure!(
            !writes.is_empty() && writes.len() <= MAX_ORDERED_WRITES,
            "V278 pending plan inventory is outside its fixed bound"
        );
        Ok(Self {
            writes: writes.into_iter().map(|write| write.fingerprint).collect(),
            _same_thread: PhantomData,
        })
    }
}

impl ExternalPoolAdapterTaskReachabilityPendingPlanGuard {
    pub(in crate::store) fn ensure_fully_consumed(&self) -> Result<()> {
        let registry = self.registry.lock().map_err(|_| registry_poisoned())?;
        let active = registry
            .active
            .as_ref()
            .filter(|active| active.generation == self.generation)
            .ok_or_else(|| anyhow!("V278 pending plan is no longer registered"))?;
        ensure!(
            active.next_index == active.writes.len(),
            "V278 pending plan was not fully consumed"
        );
        Ok(())
    }

    fn clear(&self) -> Result<()> {
        let mut registry = self.registry.lock().map_err(|_| registry_poisoned())?;
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

impl Drop for ExternalPoolAdapterTaskReachabilityPendingPlanGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.clear();
        }
    }
}

impl PendingPlanRegistry {
    fn matches(&mut self, context: &Context<'_>) -> bool {
        if context.len() < 2 {
            return false;
        }
        let Ok(sql_kind) = context.get_raw(0).as_str() else {
            return false;
        };
        let Some(kind) =
            ExternalPoolAdapterTaskReachabilityPendingWriteKind::from_sql_kind(sql_kind)
        else {
            return false;
        };
        let actual = PendingWriteFingerprint {
            kind,
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

pub(crate) fn register_external_pool_adapter_task_reachability_pending_plan_function(
    connection: &Connection,
) -> Result<()> {
    let registry = Arc::new(Mutex::new(PendingPlanRegistry {
        next_generation: 0,
        active: None,
    }));
    let udf_registry = Arc::clone(&registry);
    let flags = FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_INNOCUOUS;
    connection.create_scalar_function(
        REACHABILITY_PENDING_PLAN_MATCHES,
        -1,
        flags,
        move |context| {
            Ok(i64::from(
                udf_registry
                    .lock()
                    .map(|mut registry| registry.matches(context))
                    .unwrap_or(false),
            ))
        },
    )?;
    let key = connection_key(connection);
    let mut custody = connection_custody()
        .lock()
        .map_err(|_| registry_poisoned())?;
    custody.retain(|_, registry| registry.strong_count() != 0);
    custody.insert(key, Arc::downgrade(&registry));
    Ok(())
}

pub(in crate::store) fn install_external_pool_adapter_task_reachability_pending_plan_on(
    connection: &Connection,
    plan: ExternalPoolAdapterTaskReachabilityPendingPlan,
) -> Result<ExternalPoolAdapterTaskReachabilityPendingPlanGuard> {
    let key = connection_key(connection);
    let registry = connection_custody()
        .lock()
        .map_err(|_| registry_poisoned())?
        .get(&key)
        .and_then(Weak::upgrade)
        .ok_or_else(|| anyhow!("V278 pending-plan UDF is not registered on this connection"))?;
    let generation = {
        let mut registered = registry.lock().map_err(|_| registry_poisoned())?;
        if registered.active.is_some() {
            bail!("this SQLite connection already has a V278 pending plan");
        }
        registered.next_generation = registered
            .next_generation
            .checked_add(1)
            .ok_or_else(|| anyhow!("V278 pending-plan generation exhausted"))?;
        let generation = registered.next_generation;
        registered.active = Some(RegisteredPendingPlan {
            generation,
            writes: plan.writes,
            next_index: 0,
        });
        generation
    };
    Ok(ExternalPoolAdapterTaskReachabilityPendingPlanGuard {
        registry,
        generation,
        armed: true,
        _same_thread: PhantomData,
    })
}

fn connection_custody() -> &'static Mutex<HashMap<usize, Weak<Mutex<PendingPlanRegistry>>>> {
    CONNECTION_CUSTODY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn registry_poisoned() -> anyhow::Error {
    anyhow!("V278 pending-plan registry lock was poisoned")
}

fn connection_key(connection: &Connection) -> usize {
    // SAFETY: the handle is observed only as identity while borrowed. Every new registration at a
    // reused SQLite address replaces the Weak registry, so no stale plan is recovered by address.
    unsafe { connection.handle() as usize }
}
