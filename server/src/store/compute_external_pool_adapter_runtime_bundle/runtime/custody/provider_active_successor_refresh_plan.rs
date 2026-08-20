//! Connection-local, ordered one-shot custody for a V274 sequence-greater-than-one refresh.

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

use fingerprint::RefreshPendingPlanFingerprint;

const REFRESH_PENDING_PLAN_MATCHES: &str =
    "elon_v278_external_pool_adapter_provider_active_successor_refresh_pending_plan_matches";
const REFRESH_PENDING_PLAN_ARITY: usize = 17;
const REFRESH_PURPOSE: &str = "provider_active_successor_refresh";

static CONNECTION_CUSTODY: OnceLock<Mutex<HashMap<usize, Weak<Mutex<RefreshPendingRegistry>>>>> =
    OnceLock::new();

/// Exact trigger tuple in DDL argument order. Values retain their SQLite storage class.
pub(in crate::store) struct ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlanInput {
    values: Vec<Value>,
}

/// One exact V274 refresh INSERT. It is non-Clone/non-Serde and thread-affine.
pub(in crate::store) struct ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlan {
    fingerprint: RefreshPendingPlanFingerprint,
    _same_thread: PhantomData<Rc<()>>,
}

/// RAII registration on one exact SQLite connection; Drop clears every unpromoted plan.
pub(in crate::store) struct ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlanGuard {
    registry: Arc<Mutex<RefreshPendingRegistry>>,
    connection_key: usize,
    generation: u64,
    armed: bool,
    _same_thread: PhantomData<Rc<()>>,
}

struct RefreshPendingRegistry {
    next_generation: u64,
    active: Option<RegisteredRefreshPendingPlan>,
}

struct RegisteredRefreshPendingPlan {
    generation: u64,
    fingerprint: RefreshPendingPlanFingerprint,
    consumed: bool,
}

impl ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlanInput {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::store) fn new(
        purpose: Value,
        active_successor_receipt_id: Value,
        receipt_digest: Value,
        receipt_json: Value,
        provider_binding_id: Value,
        activation_root_digest: Value,
        successor_sequence: Value,
        predecessor_receipt_id: Value,
        predecessor_receipt_digest: Value,
        activation_target_updated_at: Value,
        evidence_checked_at: Value,
        created_at: Value,
        observation_expires_at: Value,
        process_custody_epoch_digest: Value,
        process_custody_nonce_digest: Value,
        process_custody_seal_digest: Value,
        receipt_integrity_digest: Value,
    ) -> Result<Self> {
        ensure!(
            matches!(&purpose, Value::Text(value) if value == REFRESH_PURPOSE),
            "V274 refresh pending plan has the wrong purpose"
        );
        ensure!(
            matches!(successor_sequence, Value::Integer(value) if value > 1),
            "V274 refresh pending plan requires successor_sequence greater than one"
        );
        ensure!(
            matches!(&predecessor_receipt_id, Value::Text(value) if !value.is_empty())
                && matches!(&predecessor_receipt_digest, Value::Text(value) if !value.is_empty()),
            "V274 refresh pending plan requires an exact predecessor pair"
        );
        Ok(Self {
            values: vec![
                purpose,
                active_successor_receipt_id,
                receipt_digest,
                receipt_json,
                provider_binding_id,
                activation_root_digest,
                successor_sequence,
                predecessor_receipt_id,
                predecessor_receipt_digest,
                activation_target_updated_at,
                evidence_checked_at,
                created_at,
                observation_expires_at,
                process_custody_epoch_digest,
                process_custody_nonce_digest,
                process_custody_seal_digest,
                receipt_integrity_digest,
            ],
        })
    }
}

impl ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlan {
    pub(in crate::store) fn new(
        input: ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlanInput,
    ) -> Result<Self> {
        ensure!(
            input.values.len() == REFRESH_PENDING_PLAN_ARITY,
            "V274 refresh pending plan has the wrong trigger arity"
        );
        Ok(Self {
            fingerprint: RefreshPendingPlanFingerprint::from_values(input.values),
            _same_thread: PhantomData,
        })
    }
}

impl ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlanGuard {
    pub(in crate::store) fn ensure_fully_consumed(&self) -> Result<()> {
        let registry = self.registry()?;
        let active = registry
            .active
            .as_ref()
            .filter(|active| active.generation == self.generation)
            .ok_or_else(|| anyhow!("V274 refresh pending plan is no longer registered"))?;
        ensure!(
            active.consumed,
            "V274 refresh pending plan was not consumed"
        );
        Ok(())
    }

    pub(in crate::store) fn ensure_same_connection(&self, connection: &Connection) -> Result<()> {
        let key = connection_key(connection);
        ensure!(
            key == self.connection_key,
            "V274 refresh postcommit changed SQLite connection"
        );
        let registered = connection_custody()
            .lock()
            .map_err(|_| anyhow!("V274 refresh connection-custody lock was poisoned"))?
            .get(&key)
            .and_then(Weak::upgrade)
            .ok_or_else(|| anyhow!("V274 refresh UDF is not registered on this connection"))?;
        ensure!(
            Arc::ptr_eq(&registered, &self.registry),
            "V274 refresh postcommit changed connection registration"
        );
        Ok(())
    }

    pub(in crate::store) fn discard(mut self) -> Result<()> {
        self.clear()?;
        self.armed = false;
        Ok(())
    }

    fn registry(&self) -> Result<std::sync::MutexGuard<'_, RefreshPendingRegistry>> {
        self.registry
            .lock()
            .map_err(|_| anyhow!("V274 refresh pending-plan registry lock was poisoned"))
    }

    fn clear(&self) -> Result<()> {
        let mut registry = self.registry()?;
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

impl Drop for ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlanGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.clear();
        }
    }
}

impl RefreshPendingRegistry {
    fn empty() -> Self {
        Self {
            next_generation: 0,
            active: None,
        }
    }

    fn matches(&mut self, context: &Context<'_>) -> bool {
        if context.len() != REFRESH_PENDING_PLAN_ARITY {
            return false;
        }
        let Some(active) = self.active.as_mut() else {
            return false;
        };
        if active.consumed
            || active.fingerprint != RefreshPendingPlanFingerprint::from_context(context)
        {
            return false;
        }
        active.consumed = true;
        true
    }
}

pub(crate) fn register_external_pool_adapter_provider_active_successor_refresh_pending_plan_udf(
    connection: &Connection,
) -> Result<()> {
    let registry = Arc::new(Mutex::new(RefreshPendingRegistry::empty()));
    let udf_registry = Arc::clone(&registry);
    let flags = FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_INNOCUOUS;
    connection.create_scalar_function(
        REFRESH_PENDING_PLAN_MATCHES,
        REFRESH_PENDING_PLAN_ARITY as i32,
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
    let mut custody = connection_custody()
        .lock()
        .map_err(|_| anyhow!("V274 refresh connection-custody lock was poisoned"))?;
    custody.retain(|_, registry| registry.strong_count() != 0);
    custody.insert(connection_key(connection), Arc::downgrade(&registry));
    Ok(())
}

pub(in crate::store) fn install_external_pool_adapter_provider_active_successor_refresh_pending_plan_on(
    connection: &Connection,
    plan: ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlan,
) -> Result<ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlanGuard> {
    let registry = connection_custody()
        .lock()
        .map_err(|_| anyhow!("V274 refresh connection-custody lock was poisoned"))?
        .get(&connection_key(connection))
        .and_then(Weak::upgrade)
        .ok_or_else(|| anyhow!("V274 refresh UDF is not registered on this connection"))?;
    let generation = {
        let mut state = registry
            .lock()
            .map_err(|_| anyhow!("V274 refresh pending-plan registry lock was poisoned"))?;
        if state.active.is_some() {
            bail!("this SQLite connection already has a V274 refresh pending plan");
        }
        state.next_generation = state
            .next_generation
            .checked_add(1)
            .ok_or_else(|| anyhow!("V274 refresh pending-plan generation exhausted"))?;
        let generation = state.next_generation;
        state.active = Some(RegisteredRefreshPendingPlan {
            generation,
            fingerprint: plan.fingerprint,
            consumed: false,
        });
        generation
    };
    Ok(
        ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlanGuard {
            registry,
            connection_key: connection_key(connection),
            generation,
            armed: true,
            _same_thread: PhantomData,
        },
    )
}

fn connection_custody() -> &'static Mutex<HashMap<usize, Weak<Mutex<RefreshPendingRegistry>>>> {
    CONNECTION_CUSTODY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn connection_key(connection: &Connection) -> usize {
    // SAFETY: the borrowed handle is used only as an identity; the map retains a Weak registry.
    unsafe { connection.handle() as usize }
}
