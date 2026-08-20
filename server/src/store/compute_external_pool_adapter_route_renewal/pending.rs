//! Connection-local, ordered, one-shot V278 route-renewal write plan.

use std::{
    collections::HashMap,
    marker::PhantomData,
    rc::Rc,
    sync::{Arc, Mutex, OnceLock, Weak},
};

use anyhow::{anyhow, bail, ensure, Result};
use rusqlite::{
    functions::{Context, FunctionFlags},
    types::{Value, ValueRef},
    Connection,
};

const UDF: &str = "elon_v278_external_pool_adapter_route_renewal_pending_plan_matches";
static CUSTODY: OnceLock<Mutex<HashMap<usize, Weak<Mutex<Registry>>>>> = OnceLock::new();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Kind {
    ServiceActor,
    CredentialVersion,
    CredentialRoot,
    Authorization,
    Capability,
    Seal,
    Receipt,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Write {
    kind: Kind,
    values: Vec<Cell>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Cell {
    Null,
    Integer(i64),
    Real(u64),
    Text(Vec<u8>),
    Blob(Vec<u8>),
}

pub(super) struct ExternalPoolAdapterRouteRenewalPendingPlan {
    writes: Vec<Write>,
    _same_thread: PhantomData<Rc<()>>,
}

pub(super) struct ExternalPoolAdapterRouteRenewalPendingPlanGuard {
    registry: Arc<Mutex<Registry>>,
    connection_key: usize,
    generation: u64,
    armed: bool,
    _same_thread: PhantomData<Rc<()>>,
}

struct Registry {
    generation: u64,
    active: Option<Active>,
}

struct Active {
    generation: u64,
    writes: Vec<Write>,
    next: usize,
}

impl Kind {
    fn sql(self) -> &'static str {
        match self {
            Self::ServiceActor => "service_actor_authorization",
            Self::CredentialVersion => "route_credential",
            Self::CredentialRoot => "route_credential_root_cas",
            Self::Authorization => "route_authorization",
            Self::Capability => "route_capability",
            Self::Seal => "route_seal",
            Self::Receipt => "route_renewal_receipt",
        }
    }

    fn from_sql(value: &str) -> Option<Self> {
        [
            Self::ServiceActor,
            Self::CredentialVersion,
            Self::CredentialRoot,
            Self::Authorization,
            Self::Capability,
            Self::Seal,
            Self::Receipt,
        ]
        .into_iter()
        .find(|kind| kind.sql() == value)
    }
}

impl ExternalPoolAdapterRouteRenewalPendingPlan {
    pub(super) fn new(writes: Vec<(Kind, Vec<Value>)>) -> Result<Self> {
        let writes = writes
            .into_iter()
            .map(|(kind, values)| Write {
                kind,
                values: values.into_iter().map(Cell::from_owned).collect(),
            })
            .collect::<Vec<_>>();
        ensure!(writes.len() == 12, "V278 route plan is not twelve-step");
        let expected = [
            Kind::ServiceActor,
            Kind::CredentialVersion,
            Kind::CredentialRoot,
            Kind::Authorization,
            Kind::Capability,
            Kind::Capability,
            Kind::Capability,
            Kind::Capability,
            Kind::Capability,
            Kind::Capability,
            Kind::Seal,
            Kind::Receipt,
        ];
        ensure!(
            writes.iter().map(|write| write.kind).eq(expected),
            "V278 route plan order is not exact"
        );
        Ok(Self {
            writes,
            _same_thread: PhantomData,
        })
    }
}

impl ExternalPoolAdapterRouteRenewalPendingPlanGuard {
    pub(super) fn ensure_fully_consumed(&self) -> Result<()> {
        let registry = self
            .registry
            .lock()
            .map_err(|_| anyhow!("V278 plan poisoned"))?;
        let active = registry
            .active
            .as_ref()
            .filter(|active| active.generation == self.generation)
            .ok_or_else(|| anyhow!("V278 route plan is no longer registered"))?;
        ensure!(
            active.next == active.writes.len(),
            "V278 route plan is partial"
        );
        Ok(())
    }

    pub(super) fn ensure_same_connection(&self, connection: &Connection) -> Result<()> {
        ensure!(
            self.connection_key == connection_key(connection),
            "V278 connection changed"
        );
        let registry = custody()
            .lock()
            .map_err(|_| anyhow!("V278 custody poisoned"))?
            .get(&self.connection_key)
            .and_then(Weak::upgrade)
            .ok_or_else(|| anyhow!("V278 route plan UDF is not registered"))?;
        ensure!(
            Arc::ptr_eq(&registry, &self.registry),
            "V278 registry changed"
        );
        Ok(())
    }

    pub(super) fn discard(mut self) -> Result<()> {
        self.clear()?;
        self.armed = false;
        Ok(())
    }

    fn clear(&self) -> Result<()> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| anyhow!("V278 plan poisoned"))?;
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

impl Drop for ExternalPoolAdapterRouteRenewalPendingPlanGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.clear();
        }
    }
}

impl Registry {
    fn matches(&mut self, context: &Context<'_>) -> bool {
        if context.len() < 2 {
            return false;
        }
        let Ok(sql_kind) = context.get_raw(0).as_str() else {
            return false;
        };
        let Some(kind) = Kind::from_sql(sql_kind) else {
            return false;
        };
        let actual = (1..context.len())
            .map(|index| Cell::from_ref(context.get_raw(index)))
            .collect::<Vec<_>>();
        let Some(active) = self.active.as_mut() else {
            return false;
        };
        let Some(expected) = active.writes.get(active.next) else {
            return false;
        };
        if expected.kind != kind || expected.values != actual {
            return false;
        }
        active.next += 1;
        true
    }
}

pub(crate) fn register_external_pool_adapter_route_renewal_pending_plan_function(
    connection: &Connection,
) -> Result<()> {
    let registry = Arc::new(Mutex::new(Registry {
        generation: 0,
        active: None,
    }));
    let udf = Arc::clone(&registry);
    connection.create_scalar_function(
        UDF,
        -1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_INNOCUOUS,
        move |context| {
            Ok(i64::from(
                udf.lock()
                    .map(|mut registry| registry.matches(context))
                    .unwrap_or(false),
            ))
        },
    )?;
    let mut map = custody()
        .lock()
        .map_err(|_| anyhow!("V278 custody poisoned"))?;
    map.retain(|_, registry| registry.strong_count() != 0);
    map.insert(connection_key(connection), Arc::downgrade(&registry));
    Ok(())
}

pub(super) fn install(
    connection: &Connection,
    plan: ExternalPoolAdapterRouteRenewalPendingPlan,
) -> Result<ExternalPoolAdapterRouteRenewalPendingPlanGuard> {
    let registry = custody()
        .lock()
        .map_err(|_| anyhow!("V278 custody poisoned"))?
        .get(&connection_key(connection))
        .and_then(Weak::upgrade)
        .ok_or_else(|| anyhow!("V278 route plan UDF is not registered"))?;
    let generation = {
        let mut registry = registry.lock().map_err(|_| anyhow!("V278 plan poisoned"))?;
        if registry.active.is_some() {
            bail!("this connection already has a V278 route plan")
        }
        registry.generation = registry
            .generation
            .checked_add(1)
            .ok_or_else(|| anyhow!("V278 route plan generation exhausted"))?;
        let generation = registry.generation;
        registry.active = Some(Active {
            generation,
            writes: plan.writes,
            next: 0,
        });
        generation
    };
    Ok(ExternalPoolAdapterRouteRenewalPendingPlanGuard {
        registry,
        connection_key: connection_key(connection),
        generation,
        armed: true,
        _same_thread: PhantomData,
    })
}

impl Cell {
    fn from_owned(value: Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Integer(value) => Self::Integer(value),
            Value::Real(value) => Self::Real(value.to_bits()),
            Value::Text(value) => Self::Text(value.into_bytes()),
            Value::Blob(value) => Self::Blob(value),
        }
    }

    fn from_ref(value: ValueRef<'_>) -> Self {
        match value {
            ValueRef::Null => Self::Null,
            ValueRef::Integer(value) => Self::Integer(value),
            ValueRef::Real(value) => Self::Real(value.to_bits()),
            ValueRef::Text(value) => Self::Text(value.to_vec()),
            ValueRef::Blob(value) => Self::Blob(value.to_vec()),
        }
    }
}

fn custody() -> &'static Mutex<HashMap<usize, Weak<Mutex<Registry>>>> {
    CUSTODY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn connection_key(connection: &Connection) -> usize {
    // SAFETY: used only as a borrowed connection identity; weak registries are replaced on reuse.
    unsafe { connection.handle() as usize }
}
