//! Exact installed-ABI execution for successful multi-region Map loops.

mod fixture;
mod payload;

use std::path::Path;

use anyhow::{anyhow, Context};

use crate::node_agent_managed_fs::{
    ManagedSqliteShmMapMode, ManagedSqliteShmTestMapDmsPath, ManagedSqliteShmTestMapExpectation,
    ManagedSqliteShmTestMapPath, ManagedSqliteShmTestMapReceipt,
    ManagedSqliteShmTestTargetObserver,
};

use super::super::{SanitizedChildReport, A2_DYNAMIC_CHILD_NONCE_ENV};

pub(super) use payload::{validate_payload, ValidatedRegionLoopPayloadV1};

pub(super) const SELECTED: usize = 0;
pub(super) const REGION_SIZE: u32 = 32_768;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) enum MapRunnerRegionLoopFamilyV1 {
    CreatedFirstEmptyExtendMapped,
    NodeLiveMissingExtendMapped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct MapRunnerRegionLoopBindingV1 {
    pub(in super::super::super) family: MapRunnerRegionLoopFamilyV1,
    pub(in super::super::super) target_region: u32,
    pub(in super::super::super) regions_to_create: u16,
    pub(in super::super::super) normalized_descriptor_sha256: [u8; 32],
    pub(in super::super::super) case_key_sha256: [u8; 32],
    pub(in super::super::super) full_record_sha256: [u8; 32],
    pub(in super::super::super) plan_sha256: [u8; 32],
    pub(in super::super::super) implementation_sha256: [u8; 32],
}

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

pub(super) fn validate_binding(binding: MapRunnerRegionLoopBindingV1) -> anyhow::Result<()> {
    let valid = match binding.family {
        MapRunnerRegionLoopFamilyV1::CreatedFirstEmptyExtendMapped => {
            binding.target_region <= 255
                && binding.regions_to_create == binding.target_region as u16 + 1
        }
        MapRunnerRegionLoopFamilyV1::NodeLiveMissingExtendMapped => {
            (1..=255).contains(&binding.target_region)
                && binding.regions_to_create == binding.target_region as u16
        }
    };
    if !valid {
        return Err(anyhow!("Map region-loop binding axes mismatch"));
    }
    Ok(())
}

pub(super) fn exact_selector(binding: MapRunnerRegionLoopBindingV1) -> anyhow::Result<String> {
    validate_binding(binding)?;
    super::super::child::map_region_loop::selector(
        binding.family.tag(),
        binding.target_region,
        binding.regions_to_create,
    )
    .map_err(anyhow::Error::msg)
}

pub(super) fn exercise_child(
    root: &Path,
    binding: MapRunnerRegionLoopBindingV1,
) -> anyhow::Result<()> {
    validate_binding(binding)?;
    let nonce = std::env::var(A2_DYNAMIC_CHILD_NONCE_ENV)
        .context("read parent-created Map region-loop child nonce")?;
    SanitizedChildReport::validate_root_before_exercise(&nonce, root)
        .map_err(anyhow::Error::msg)?;

    let fixture = fixture::prepare(root)?;
    fixture::install_prestate(&fixture, binding)?;
    let target_binding = fixture
        .route(SELECTED)?
        .installed_shm_fault_witness()
        .map_err(anyhow::Error::msg)?;
    let observer = target_binding.observer().map_err(anyhow::Error::msg)?;
    let target = target_binding
        .target_witness()
        .map_err(anyhow::Error::msg)?;
    let before = observer.snapshot()?;
    fixture::validate_prestate(binding, before)?;

    let armed = ArmedMapObservation::begin(&observer, map_expectation(binding))?;
    let callback = fixture
        .route(SELECTED)?
        .call_main_shm_map_raw(binding.target_region as i32, REGION_SIZE as i32, 1)
        .map_err(anyhow::Error::msg)?;
    let after = observer.snapshot()?;
    let receipt = armed.finish()?;
    let relation = fixture::validate_action(binding, callback, before, after, receipt)?;
    if target_binding.pending_count().map_err(anyhow::Error::msg)? != 0 {
        return Err(anyhow!(
            "Map region-loop exact target retained a fault token"
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

pub(super) const fn map_expectation(
    binding: MapRunnerRegionLoopBindingV1,
) -> ManagedSqliteShmTestMapExpectation {
    ManagedSqliteShmTestMapExpectation {
        region: binding.target_region,
        region_size: REGION_SIZE,
        regions_to_create: binding.regions_to_create,
        mode: ManagedSqliteShmMapMode::Extend,
        path: ManagedSqliteShmTestMapPath::MappedNew,
        dms_path: binding.family.managed_dms_path(),
    }
}

impl MapRunnerRegionLoopFamilyV1 {
    pub(super) const fn tag(self) -> u64 {
        match self {
            Self::CreatedFirstEmptyExtendMapped => 1,
            Self::NodeLiveMissingExtendMapped => 2,
        }
    }

    pub(super) const fn is_empty(self) -> bool {
        matches!(self, Self::CreatedFirstEmptyExtendMapped)
    }

    pub(super) const fn managed_dms_path(self) -> ManagedSqliteShmTestMapDmsPath {
        match self {
            Self::CreatedFirstEmptyExtendMapped => {
                ManagedSqliteShmTestMapDmsPath::CreatedFirstShared
            }
            Self::NodeLiveMissingExtendMapped => ManagedSqliteShmTestMapDmsPath::NodeLive,
        }
    }
}
