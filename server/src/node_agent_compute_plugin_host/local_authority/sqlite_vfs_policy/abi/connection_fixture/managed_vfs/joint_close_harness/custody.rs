//! Exact physical/topology observation and retained-custody projection.

use anyhow::{anyhow, Context};

use super::{
    super::a2b2_cases::{
        JointCloseActualCustody, JointCloseActualTopology, JointCloseCause, JointCloseDmsCustody,
        JointClosePhase, JointCloseSelector as S, JointCloseTiming,
    },
    boundary::SealedJointCloseBoundary,
    outcome,
    prepare::JointCloseFixture,
};
use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::{
        ManagedSqliteRegistryTerminalCustodyTestSnapshot, ManagedSqliteTestVfsRoutePhase,
    },
    node_agent_managed_fs::{
        ManagedSqliteShmTestDmsCustody as Dms, ManagedSqliteShmTestTargetSnapshot,
    },
};

pub(super) struct ObservedCustody {
    pub(super) post: JointCloseActualTopology,
    pub(super) retained: JointCloseActualCustody,
}

pub(super) fn validate_and_project(
    fixture: &JointCloseFixture,
    boundary: SealedJointCloseBoundary,
    physical: ManagedSqliteShmTestTargetSnapshot,
    terminal: ManagedSqliteRegistryTerminalCustodyTestSnapshot,
) -> anyhow::Result<ObservedCustody> {
    validate_registration_shape(fixture)?;
    validate_physical(boundary, physical)?;
    let (main_lease, shm_lease, callback_leases) = registry_leases(fixture, boundary, terminal)?;
    let main_completed = boundary.phase() == JointClosePhase::Success
        || boundary.phase() == JointClosePhase::RegistryWalMainClose
        || (boundary.phase() == JointClosePhase::MainFileClose
            && boundary.variant() == 0
            && boundary.timing() == JointCloseTiming::AfterSuccessKnown);
    Ok(ObservedCustody {
        post: JointCloseActualTopology {
            sqlite_connections: 1,
            shm_connections: physical.topology.shm_connections,
            registry_routes: 1,
            logical_names: 3,
        },
        retained: JointCloseActualCustody {
            node: physical.topology.node_present,
            views: checked(physical.topology.views, "views")?,
            mappings: checked(physical.topology.mappings, "mappings")?,
            dms: project_dms(physical.topology.dms)?,
            shm_file: physical.topology.shm_file_present,
            main_file: !main_completed,
            main_lock_owner: !main_completed,
            main_lease,
            shm_lease,
            callback_leases,
            registry_entry: true,
            logical_names: 3,
            vfs_table: true,
            vfs_name: true,
            vfs_context: true,
            root_deletable: false,
        },
    })
}

fn validate_registration_shape(fixture: &JointCloseFixture) -> anyhow::Result<()> {
    let registration = fixture.owner().live_registration_snapshot()?;
    // The terminal close paths intentionally remove process-owner route custody while
    // retaining the VFS logical-name index.  Observe that index directly: the broader
    // shutdown snapshot also asks the removed owner route for custody and is therefore
    // not a valid post-terminal observer.
    let logical = fixture
        .owner()
        .route(super::prepare::SELECTED)?
        .barrier_logical_route_snapshot()?;
    let routes = logical.live_routes();
    let names = logical.logical_names();
    let exact_names = logical.exact_route_names();
    if fixture.owner().live_connection_count() != 1
        || routes != 1
        || names != 3
        || exact_names != fixture.prepared_names
        || exact_names != 3
        || !registration.registered()
        || !registration.table_present()
        || !registration.name_present()
        || !registration.context_present()
    {
        return Err(anyhow!(
            "JointClose retained registration shape is not exact"
        ));
    }
    Ok(())
}

fn validate_physical(
    boundary: SealedJointCloseBoundary,
    actual: ManagedSqliteShmTestTargetSnapshot,
) -> anyhow::Result<()> {
    let expected = expected_physical(boundary)?;
    let topology = actual.topology;
    if actual.target_attached != expected.target_attached
        || topology.shm_connections != u8::from(expected.target_attached)
        || topology.node_present != expected.node
        || topology.views != expected.views
        || topology.mappings != expected.mappings
        || topology.dms != expected.dms
        || topology.shm_file_present != expected.shm_file
        || topology.poisoned != expected.poisoned
        || topology.mutation_may_have_occurred != expected.mutation
        || topology.lock_outcome_uncertain != expected.lock_uncertain
        || topology.domain_terminal != expected.domain_terminal
        || topology.quarantined_file_closes != expected.quarantined_file_closes
        || actual.shared_mask != 0
        || actual.exclusive_mask != 0
    {
        return Err(anyhow!(
            "JointClose observed physical custody differs from its sealed boundary"
        ));
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct PhysicalShape {
    target_attached: bool,
    node: bool,
    views: u16,
    mappings: u16,
    dms: Dms,
    shm_file: bool,
    poisoned: bool,
    mutation: bool,
    lock_uncertain: bool,
    domain_terminal: bool,
    quarantined_file_closes: u16,
}

fn expected_physical(boundary: SealedJointCloseBoundary) -> anyhow::Result<PhysicalShape> {
    let live = PhysicalShape {
        target_attached: true,
        node: true,
        views: 1,
        mappings: 1,
        dms: Dms::Shared,
        shm_file: true,
        poisoned: false,
        mutation: false,
        lock_uncertain: false,
        domain_terminal: false,
        quarantined_file_closes: 0,
    };
    if boundary.phase() != JointClosePhase::ShmUnmapLift {
        return if matches!(
            boundary.phase(),
            JointClosePhase::MainLockRelease
                | JointClosePhase::MainFileClose
                | JointClosePhase::RegistryWalMainClose
                | JointClosePhase::Success
        ) && !(boundary.phase() == JointClosePhase::MainFileClose
            && boundary.variant() == 1)
        {
            Ok(absent())
        } else {
            Ok(live)
        };
    }
    let success = matches!(
        boundary.timing(),
        JointCloseTiming::AfterSuccessKnown | JointCloseTiming::AfterSuccessUncertain
    );
    let mut value = live;
    match boundary.cause() {
        JointCloseCause::ViewUnmap => value.views = u16::from(!success),
        JointCloseCause::MappingClose => {
            value.views = 0;
            value.mappings = u16::from(!success);
        }
        JointCloseCause::DmsSharedRelease => {
            value.views = 0;
            value.mappings = 0;
            value.dms = if boundary.timing() == JointCloseTiming::NativeUncertain {
                Dms::SharedOutcomeUncertain
            } else if success {
                Dms::Released
            } else {
                Dms::Shared
            };
        }
        JointCloseCause::ShmFileClose => {
            value.views = 0;
            value.mappings = 0;
            value.dms = Dms::Released;
            if boundary.timing() != JointCloseTiming::BeforeCall {
                value.node = false;
                value.dms = Dms::Absent;
            }
            value.shm_file = !success;
            value.quarantined_file_closes = u16::from(!success && !value.node);
        }
        JointCloseCause::ConnectionDetach => {
            value = absent();
            value.target_attached = !success;
        }
        JointCloseCause::None => return Err(anyhow!("JointClose SHM cause is absent")),
    }
    value.poisoned = boundary.domain_terminal();
    value.mutation = boundary.mutation_may_have_occurred();
    value.lock_uncertain = boundary.lock_outcome_uncertain();
    value.domain_terminal = boundary.domain_terminal();
    Ok(value)
}

fn absent() -> PhysicalShape {
    PhysicalShape {
        target_attached: false,
        node: false,
        views: 0,
        mappings: 0,
        dms: Dms::Absent,
        shm_file: false,
        poisoned: false,
        mutation: false,
        lock_uncertain: false,
        domain_terminal: false,
        quarantined_file_closes: 0,
    }
}

fn registry_leases(
    fixture: &JointCloseFixture,
    boundary: SealedJointCloseBoundary,
    custody: ManagedSqliteRegistryTerminalCustodyTestSnapshot,
) -> anyhow::Result<(bool, bool, u8)> {
    if boundary.phase() == JointClosePhase::RawStateTake {
        let route = fixture
            .owner()
            .route(super::prepare::SELECTED)
            .map_err(anyhow::Error::msg)?
            .route_custody_snapshot()
            .map_err(anyhow::Error::msg)?;
        if route.phase() != ManagedSqliteTestVfsRoutePhase::Active
            || !route.connection_owner()
            || !route.main_file_lock_owner_lease()
            || !route.shm_lease()
            || route.callbacks_in_flight() != 0
            || !route.access_callback_allowed()
        {
            return Err(anyhow!(
                "JointClose RawState retained route custody is not exact"
            ));
        }
        return Ok((
            route.main_file_lock_owner_lease(),
            route.shm_lease(),
            u8::try_from(route.callbacks_in_flight())?,
        ));
    }
    if boundary.phase() == JointClosePhase::Success {
        let (_, main, shm, callbacks) = custody.physical_success_handoff_shape();
        return Ok((main, shm, u8::try_from(callbacks)?));
    }
    let terminal = custody
        .terminal_route()
        .ok_or_else(|| anyhow!("JointClose terminal registry lease receipt is absent"))?;
    Ok((
        terminal.main_file_lock_owner_lease(),
        terminal.shm_lease(),
        u8::try_from(terminal.callbacks_in_flight())?,
    ))
}

fn project_dms(value: Dms) -> anyhow::Result<JointCloseDmsCustody> {
    match value {
        Dms::Absent => Ok(JointCloseDmsCustody::Absent),
        Dms::Shared => Ok(JointCloseDmsCustody::Shared),
        Dms::SharedOutcomeUncertain => Ok(JointCloseDmsCustody::OutcomeUncertain),
        Dms::Released => Ok(JointCloseDmsCustody::Released),
        Dms::ExclusiveKnown | Dms::ExclusiveOutcomeUncertain => Err(anyhow!(
            "JointClose retained an impossible exclusive DMS custody"
        )),
    }
}

fn checked(value: u16, label: &'static str) -> anyhow::Result<u8> {
    u8::try_from(value).with_context(|| format!("JointClose {label} exceed u8"))
}
