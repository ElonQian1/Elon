//! Exact installed-ABI execution for six positive single-region Map lifecycles.

mod fixture;
mod payload;

use std::path::Path;

use anyhow::Context;

use crate::node_agent_managed_fs::{
    ManagedSqliteShmMapMode, ManagedSqliteShmTestMapDmsPath, ManagedSqliteShmTestMapExpectation,
    ManagedSqliteShmTestMapPath, ManagedSqliteShmTestMapReceipt,
    ManagedSqliteShmTestTargetObserver,
};

use super::super::{SanitizedChildReport, A2_DYNAMIC_CHILD_NONCE_ENV};
use super::{MapRunnerLifecycleBindingV1, MapRunnerLifecyclePathV1};

pub(super) use payload::{validate_payload, ValidatedLifecyclePayloadV1};

pub(super) const SELECTED: usize = 0;
pub(super) const REGION_SIZE: u32 = 32_768;

struct ArmedMapObservation<'a> {
    observer: &'a ManagedSqliteShmTestTargetObserver,
    active: bool,
}

impl<'a> ArmedMapObservation<'a> {
    fn begin(
        observer: &'a ManagedSqliteShmTestTargetObserver,
        expectation: ManagedSqliteShmTestMapExpectation,
    ) -> anyhow::Result<Self> {
        observer
            .begin_map_action_observation(expectation)
            .map_err(anyhow::Error::msg)?;
        Ok(Self {
            observer,
            active: true,
        })
    }

    fn finish(mut self) -> anyhow::Result<ManagedSqliteShmTestMapReceipt> {
        let receipt = self
            .observer
            .finish_map_action_observation()
            .map_err(anyhow::Error::msg)?;
        self.active = false;
        Ok(receipt)
    }
}

impl Drop for ArmedMapObservation<'_> {
    fn drop(&mut self) {
        if self.active {
            let _ = self.observer.cancel_map_action_observation();
        }
    }
}

pub(super) fn validate_binding(_binding: MapRunnerLifecycleBindingV1) -> anyhow::Result<()> {
    Ok(())
}

pub(super) fn exercise_child(
    root: &Path,
    binding: MapRunnerLifecycleBindingV1,
) -> anyhow::Result<()> {
    validate_binding(binding)?;
    let nonce = std::env::var(A2_DYNAMIC_CHILD_NONCE_ENV)
        .context("read parent-created Map lifecycle child nonce")?;
    SanitizedChildReport::validate_root_before_exercise(&nonce, root)
        .map_err(anyhow::Error::msg)?;

    let fixture = fixture::prepare(root)?;
    let prestate = fixture::install_prestate(&fixture, binding.path)?;
    let target_binding = fixture
        .route(SELECTED)?
        .installed_shm_fault_witness()
        .map_err(anyhow::Error::msg)?;
    let observer = target_binding.observer().map_err(anyhow::Error::msg)?;
    let target = target_binding
        .target_witness()
        .map_err(anyhow::Error::msg)?;
    let before = observer.snapshot()?;
    fixture::validate_prestate(binding.path, before)?;

    let armed = ArmedMapObservation::begin(&observer, map_expectation(binding.path))?;
    let callback = fixture
        .route(SELECTED)?
        .call_main_shm_map_raw(
            binding.path.region() as i32,
            REGION_SIZE as i32,
            binding.path.raw_extend(),
        )
        .map_err(anyhow::Error::msg)?;
    let after = observer.snapshot()?;
    let receipt = armed.finish()?;
    let relation =
        fixture::validate_action(binding.path, prestate, callback, before, after, receipt)?;
    let pending_count = target_binding.pending_count().map_err(anyhow::Error::msg)?;
    if pending_count != 0 {
        return Err(anyhow::anyhow!(
            "Map lifecycle exact target retained a fault token"
        ));
    }

    let registration = fixture.live_registration_snapshot()?;
    let registration_values = [
        u64::from(registration.registered()),
        u64::from(registration.table_present()),
        u64::from(registration.name_present()),
        u64::from(registration.context_present()),
    ];
    let (routes, logical_names) = fixture.logical_route_counts()?;
    let route_values = [
        fixture.live_connection_count() as u64,
        routes as u64,
        logical_names as u64,
    ];
    let autocommit = fixture.connection(SELECTED)?.is_autocommit();
    let liveness: i64 = fixture
        .connection(SELECTED)?
        .query_row("SELECT 1", [], |row| row.get(0))?;
    fixture.close()?;
    let terminal_values = [
        u64::from(autocommit),
        u64::from(liveness == 1),
        1,
        u64::from(root.is_dir()),
    ];
    let payload = payload::encode(
        binding,
        target.registration_id(),
        target.route_ordinal(),
        target.runtime_generation(),
        target.shm_connection_id(),
        callback,
        before,
        after,
        receipt,
        relation,
        registration_values,
        route_values,
        terminal_values,
    );
    let report = SanitizedChildReport::encode_for_current_child(
        &nonce,
        root,
        target.registration_id(),
        &payload,
    )
    .map_err(anyhow::Error::msg)?;
    println!("{report}");
    Ok(())
}

pub(super) fn map_expectation(
    path: MapRunnerLifecyclePathV1,
) -> ManagedSqliteShmTestMapExpectation {
    ManagedSqliteShmTestMapExpectation {
        region: path.region(),
        region_size: REGION_SIZE,
        mode: path.managed_mode(),
        path: path.managed_path(),
        dms_path: path.managed_dms_path(),
    }
}

impl MapRunnerLifecyclePathV1 {
    pub(super) const fn region(self) -> u32 {
        match self {
            Self::MissingObserveNotPresent | Self::MissingExtendMapped => 1,
            _ => 0,
        }
    }

    pub(super) const fn managed_mode(self) -> ManagedSqliteShmMapMode {
        match self {
            Self::EmptyObserveNotPresent
            | Self::ReuseObserveMapped
            | Self::MissingObserveNotPresent => ManagedSqliteShmMapMode::Observe,
            Self::EmptyExtendMapped | Self::ReuseExtendMapped | Self::MissingExtendMapped => {
                ManagedSqliteShmMapMode::Extend
            }
        }
    }

    pub(super) const fn raw_extend(self) -> i32 {
        mode_tag(self) as i32
    }

    pub(super) const fn managed_path(self) -> ManagedSqliteShmTestMapPath {
        match self {
            Self::EmptyObserveNotPresent | Self::MissingObserveNotPresent => {
                ManagedSqliteShmTestMapPath::NotPresent
            }
            Self::EmptyExtendMapped | Self::MissingExtendMapped => {
                ManagedSqliteShmTestMapPath::MappedNew
            }
            Self::ReuseObserveMapped | Self::ReuseExtendMapped => {
                ManagedSqliteShmTestMapPath::MappedReuse
            }
        }
    }

    pub(super) const fn managed_dms_path(self) -> ManagedSqliteShmTestMapDmsPath {
        if self.is_empty() {
            ManagedSqliteShmTestMapDmsPath::CreatedFirstShared
        } else {
            ManagedSqliteShmTestMapDmsPath::NodeLive
        }
    }

    pub(super) const fn is_empty(self) -> bool {
        matches!(self, Self::EmptyObserveNotPresent | Self::EmptyExtendMapped)
    }

    pub(super) const fn is_missing(self) -> bool {
        matches!(
            self,
            Self::MissingObserveNotPresent | Self::MissingExtendMapped
        )
    }

    pub(super) const fn is_reuse(self) -> bool {
        matches!(self, Self::ReuseObserveMapped | Self::ReuseExtendMapped)
    }

    pub(super) const fn is_mapped(self) -> bool {
        !matches!(
            self,
            Self::EmptyObserveNotPresent | Self::MissingObserveNotPresent
        )
    }

    pub(super) const fn is_new_mapping(self) -> bool {
        matches!(self, Self::EmptyExtendMapped | Self::MissingExtendMapped)
    }
}

pub(super) const fn mode_tag(path: MapRunnerLifecyclePathV1) -> u64 {
    match path.managed_mode() {
        ManagedSqliteShmMapMode::Observe => 0,
        ManagedSqliteShmMapMode::Extend => 1,
    }
}

pub(super) const fn path_tag(path: MapRunnerLifecyclePathV1) -> u64 {
    match path {
        MapRunnerLifecyclePathV1::EmptyObserveNotPresent => 1,
        MapRunnerLifecyclePathV1::EmptyExtendMapped => 2,
        MapRunnerLifecyclePathV1::ReuseObserveMapped => 3,
        MapRunnerLifecyclePathV1::ReuseExtendMapped => 4,
        MapRunnerLifecyclePathV1::MissingObserveNotPresent => 5,
        MapRunnerLifecyclePathV1::MissingExtendMapped => 6,
    }
}
