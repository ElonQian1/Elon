//! Process-isolated native receipts for Map request guards and positive region lifecycles.

mod lifecycle;
mod region_loop;
mod request_budget;

pub(in super::super) use region_loop::{MapRunnerRegionLoopBindingV1, MapRunnerRegionLoopFamilyV1};
pub(in super::super) use request_budget::MapRunnerRequestBudgetV1;

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{anyhow, Context};
use rusqlite::ffi;
use sha2::{Digest, Sha256};

use super::child::SanitizedPayloadFamily;
use super::{
    ChildLaunchIdentity, DynamicChildFailure, SanitizedChildReport, ValidatedChildProcessReceipt,
    ValidatedParentCleanupReceipt, WindowsDynamicEnvironment, A2_DYNAMIC_CHILD_NONCE_ENV,
};
use crate::node_agent_managed_fs::{
    ManagedSqliteShmTestDmsCustody, ManagedSqliteShmTestTargetSnapshot,
};

use super::super::ManagedSqliteRoutedConnectionFixture;

const CHILD_ROOT_ENV: &str = "ELON_SQLITE_A2_MAP_QUOTIENT_CHILD_ROOT";
const REGION_LOOP_SELECTOR_ENV: &str = "ELON_SQLITE_A2_MAP_REGION_LOOP_SELECTOR";
const PAYLOAD_VERSION: &str = "a2mapq2";
const PAYLOAD_VALUE_COUNT: usize = 67;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum MapRunnerModeV1 {
    Observe,
    Extend,
}

impl MapRunnerModeV1 {
    const fn tag(self) -> u64 {
        match self {
            Self::Observe => 0,
            Self::Extend => 1,
        }
    }

    const fn raw_extend(self) -> i32 {
        self.tag() as i32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct MapRunnerProgramBindingV1 {
    pub(in super::super) mode: MapRunnerModeV1,
    pub(in super::super) request_budget: MapRunnerRequestBudgetV1,
    pub(in super::super) normalized_descriptor_sha256: [u8; 32],
    pub(in super::super) case_key_sha256: [u8; 32],
    pub(in super::super) full_record_sha256: [u8; 32],
    pub(in super::super) plan_sha256: [u8; 32],
    pub(in super::super) implementation_sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum MapRunnerLifecyclePathV1 {
    EmptyObserveNotPresent,
    EmptyExtendMapped,
    ReuseObserveMapped,
    ReuseExtendMapped,
    MissingObserveNotPresent,
    MissingExtendMapped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct MapRunnerLifecycleBindingV1 {
    pub(in super::super) path: MapRunnerLifecyclePathV1,
    pub(in super::super) normalized_descriptor_sha256: [u8; 32],
    pub(in super::super) case_key_sha256: [u8; 32],
    pub(in super::super) full_record_sha256: [u8; 32],
    pub(in super::super) plan_sha256: [u8; 32],
    pub(in super::super) implementation_sha256: [u8; 32],
}

pub(in super::super) enum MapRunnerIsolatedEvidenceV1 {
    ParentReceipt(MapRunnerEvidenceReceiptV1),
    ChildReported,
}

/// Opaque components produced only after exact-child exit, root rebinding and parent deletion.
pub(in super::super) struct MapRunnerEvidenceReceiptV1 {
    root_commitment_sha256: [u8; 32],
    child_fingerprint_sha256: [u8; 32],
    registration_commitment_sha256: [u8; 32],
    payload_commitment_sha256: [u8; 32],
    environment_sha256: [u8; 32],
    cleanup_sha256: [u8; 32],
    native_receipt_sha256: [u8; 32],
    child_exit_code: i32,
}

impl MapRunnerEvidenceReceiptV1 {
    pub(in super::super) fn into_bindings(
        self,
    ) -> (
        [u8; 32],
        [u8; 32],
        [u8; 32],
        [u8; 32],
        [u8; 32],
        [u8; 32],
        [u8; 32],
        i32,
    ) {
        (
            self.root_commitment_sha256,
            self.child_fingerprint_sha256,
            self.registration_commitment_sha256,
            self.payload_commitment_sha256,
            self.environment_sha256,
            self.cleanup_sha256,
            self.native_receipt_sha256,
            self.child_exit_code,
        )
    }
}

pub(in super::super) fn run_map_program_isolated(
    exact_test: &str,
    binding: MapRunnerProgramBindingV1,
) -> anyhow::Result<MapRunnerIsolatedEvidenceV1> {
    if let Some(root) = selected_child_root()? {
        exercise_child(&root, binding)?;
        return Ok(MapRunnerIsolatedEvidenceV1::ChildReported);
    }
    run_parent(exact_test, binding)
}

pub(in super::super) fn run_map_lifecycle_program_isolated(
    exact_test: &str,
    binding: MapRunnerLifecycleBindingV1,
) -> anyhow::Result<MapRunnerIsolatedEvidenceV1> {
    lifecycle::validate_binding(binding)?;
    if let Some(root) = selected_child_root()? {
        lifecycle::exercise_child(&root, binding)?;
        return Ok(MapRunnerIsolatedEvidenceV1::ChildReported);
    }
    run_lifecycle_parent(exact_test, binding)
}

pub(in super::super) fn run_map_region_loop_program_isolated(
    exact_test: &str,
    binding: MapRunnerRegionLoopBindingV1,
) -> anyhow::Result<MapRunnerIsolatedEvidenceV1> {
    region_loop::validate_binding(binding)?;
    if let Some(root) = selected_child_root()? {
        let selected = std::env::var(REGION_LOOP_SELECTOR_ENV)
            .context("read parent-selected Map region-loop program")?;
        if selected == region_loop::exact_selector(binding)? {
            region_loop::exercise_child(&root, binding)?;
        }
        return Ok(MapRunnerIsolatedEvidenceV1::ChildReported);
    }
    run_region_loop_parent(exact_test, binding)
}

fn run_parent(
    exact_test: &str,
    binding: MapRunnerProgramBindingV1,
) -> anyhow::Result<MapRunnerIsolatedEvidenceV1> {
    if exact_test.is_empty() {
        return Err(anyhow!("Map quotient exact test name is empty"));
    }
    let executable = std::env::current_exe().context("resolve current Map test executable")?;
    let root = create_private_child_root()?;
    let launch = ChildLaunchIdentity::new();
    let spawned = match Command::new(executable)
        .args(["--exact", exact_test, "--nocapture"])
        .env(CHILD_ROOT_ENV, &root)
        .env(A2_DYNAMIC_CHILD_NONCE_ENV, launch.env_value())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return Err(cleanup_failed_root(&root, anyhow!(error))),
    };
    let bound = launch
        .bind(spawned)
        .map_err(|failure| handle_child_failure(&root, failure))?;
    let child = bound
        .wait_for_successful_report()
        .map_err(|failure| handle_child_failure(&root, failure))?;
    let result = validate_parent_receipt(&root, binding, child);
    result.map_err(|error| cleanup_failed_root(&root, error))
}

fn run_lifecycle_parent(
    exact_test: &str,
    binding: MapRunnerLifecycleBindingV1,
) -> anyhow::Result<MapRunnerIsolatedEvidenceV1> {
    if exact_test.is_empty() {
        return Err(anyhow!("Map lifecycle exact test name is empty"));
    }
    let executable = std::env::current_exe().context("resolve current Map test executable")?;
    let root = create_private_child_root()?;
    let launch = ChildLaunchIdentity::new();
    let spawned = match Command::new(executable)
        .args(["--exact", exact_test, "--nocapture"])
        .env(CHILD_ROOT_ENV, &root)
        .env(A2_DYNAMIC_CHILD_NONCE_ENV, launch.env_value())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return Err(cleanup_failed_root(&root, anyhow!(error))),
    };
    let bound = launch
        .bind(spawned)
        .map_err(|failure| handle_child_failure(&root, failure))?;
    let child = bound
        .wait_for_successful_report()
        .map_err(|failure| handle_child_failure(&root, failure))?;
    validate_lifecycle_parent_receipt(&root, binding, child)
        .map_err(|error| cleanup_failed_root(&root, error))
}

fn run_region_loop_parent(
    exact_test: &str,
    binding: MapRunnerRegionLoopBindingV1,
) -> anyhow::Result<MapRunnerIsolatedEvidenceV1> {
    if exact_test.is_empty() {
        return Err(anyhow!("Map region-loop exact test name is empty"));
    }
    let executable = std::env::current_exe().context("resolve current Map test executable")?;
    let root = create_private_child_root()?;
    let launch = ChildLaunchIdentity::new();
    let selector = region_loop::exact_selector(binding)?;
    let spawned = match Command::new(executable)
        .args(["--exact", exact_test, "--nocapture"])
        .env(CHILD_ROOT_ENV, &root)
        .env(REGION_LOOP_SELECTOR_ENV, selector)
        .env(A2_DYNAMIC_CHILD_NONCE_ENV, launch.env_value())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return Err(cleanup_failed_root(&root, anyhow!(error))),
    };
    let bound = launch
        .bind(spawned)
        .map_err(|failure| handle_child_failure(&root, failure))?;
    let child = bound
        .wait_for_successful_report()
        .map_err(|failure| handle_child_failure(&root, failure))?;
    validate_region_loop_parent_receipt(&root, binding, child)
        .map_err(|error| cleanup_failed_root(&root, error))
}

fn validate_parent_receipt(
    root: &Path,
    binding: MapRunnerProgramBindingV1,
    child: ValidatedChildProcessReceipt,
) -> anyhow::Result<MapRunnerIsolatedEvidenceV1> {
    if !child.matches_family(SanitizedPayloadFamily::MapQuotient) {
        return Err(anyhow!("Map quotient child payload family mismatch"));
    }
    let registration_id = validate_payload(child.actual_payload(), binding)?;
    if !child.matches_registration_id(registration_id) {
        return Err(anyhow!("Map quotient child registration binding mismatch"));
    }
    let environment =
        WindowsDynamicEnvironment::capture(root, &child).map_err(anyhow::Error::msg)?;
    let cleanup = ValidatedParentCleanupReceipt::remove_after_child_exit(&child, &environment)
        .map_err(anyhow::Error::msg)?;
    let child_fingerprint = child.fingerprint();
    if child_fingerprint != cleanup.child_fingerprint
        || child.root_commitment != cleanup.root_commitment
        || child.registration_commitment != cleanup.registration_commitment
    {
        return Err(anyhow!("Map quotient parent cleanup binding mismatch"));
    }
    Ok(MapRunnerIsolatedEvidenceV1::ParentReceipt(
        MapRunnerEvidenceReceiptV1 {
            root_commitment_sha256: child.root_commitment.0,
            child_fingerprint_sha256: child_fingerprint.0,
            registration_commitment_sha256: child.registration_commitment.0,
            payload_commitment_sha256: child.payload_commitment.0,
            environment_sha256: digest_environment(&environment),
            cleanup_sha256: digest_cleanup(&cleanup),
            native_receipt_sha256: digest_native_receipt(child.actual_payload()),
            child_exit_code: child.exit_code,
        },
    ))
}

fn validate_lifecycle_parent_receipt(
    root: &Path,
    binding: MapRunnerLifecycleBindingV1,
    child: ValidatedChildProcessReceipt,
) -> anyhow::Result<MapRunnerIsolatedEvidenceV1> {
    if !child.matches_family(SanitizedPayloadFamily::MapQuotient) {
        return Err(anyhow!("Map lifecycle child payload family mismatch"));
    }
    let payload = lifecycle::validate_payload(child.actual_payload(), binding)?;
    if !child.matches_registration_id(payload.registration_id) {
        return Err(anyhow!("Map lifecycle child registration binding mismatch"));
    }
    let environment =
        WindowsDynamicEnvironment::capture(root, &child).map_err(anyhow::Error::msg)?;
    let cleanup = ValidatedParentCleanupReceipt::remove_after_child_exit(&child, &environment)
        .map_err(anyhow::Error::msg)?;
    let child_fingerprint = child.fingerprint();
    if child_fingerprint != cleanup.child_fingerprint
        || child.root_commitment != cleanup.root_commitment
        || child.registration_commitment != cleanup.registration_commitment
    {
        return Err(anyhow!("Map lifecycle parent cleanup binding mismatch"));
    }
    Ok(MapRunnerIsolatedEvidenceV1::ParentReceipt(
        MapRunnerEvidenceReceiptV1 {
            root_commitment_sha256: child.root_commitment.0,
            child_fingerprint_sha256: child_fingerprint.0,
            registration_commitment_sha256: child.registration_commitment.0,
            payload_commitment_sha256: child.payload_commitment.0,
            environment_sha256: digest_environment(&environment),
            cleanup_sha256: digest_cleanup(&cleanup),
            native_receipt_sha256: payload.native_receipt_sha256,
            child_exit_code: child.exit_code,
        },
    ))
}

fn validate_region_loop_parent_receipt(
    root: &Path,
    binding: MapRunnerRegionLoopBindingV1,
    child: ValidatedChildProcessReceipt,
) -> anyhow::Result<MapRunnerIsolatedEvidenceV1> {
    if !child.matches_family(SanitizedPayloadFamily::MapQuotient) {
        return Err(anyhow!("Map region-loop child payload family mismatch"));
    }
    let payload = region_loop::validate_payload(child.actual_payload(), binding)?;
    if !child.matches_registration_id(payload.registration_id) {
        return Err(anyhow!(
            "Map region-loop child registration binding mismatch"
        ));
    }
    let environment =
        WindowsDynamicEnvironment::capture(root, &child).map_err(anyhow::Error::msg)?;
    let cleanup = ValidatedParentCleanupReceipt::remove_after_child_exit(&child, &environment)
        .map_err(anyhow::Error::msg)?;
    let child_fingerprint = child.fingerprint();
    if child_fingerprint != cleanup.child_fingerprint
        || child.root_commitment != cleanup.root_commitment
        || child.registration_commitment != cleanup.registration_commitment
    {
        return Err(anyhow!("Map region-loop parent cleanup binding mismatch"));
    }
    Ok(MapRunnerIsolatedEvidenceV1::ParentReceipt(
        MapRunnerEvidenceReceiptV1 {
            root_commitment_sha256: child.root_commitment.0,
            child_fingerprint_sha256: child_fingerprint.0,
            registration_commitment_sha256: child.registration_commitment.0,
            payload_commitment_sha256: child.payload_commitment.0,
            environment_sha256: digest_environment(&environment),
            cleanup_sha256: digest_cleanup(&cleanup),
            native_receipt_sha256: payload.native_receipt_sha256,
            child_exit_code: child.exit_code,
        },
    ))
}

fn selected_child_root() -> anyhow::Result<Option<PathBuf>> {
    let Some(root) = std::env::var_os(CHILD_ROOT_ENV).map(PathBuf::from) else {
        return Ok(None);
    };
    if !root.is_absolute() {
        return Err(anyhow!("Map quotient child root is not absolute"));
    }
    Ok(Some(root))
}

fn exercise_child(root: &Path, binding: MapRunnerProgramBindingV1) -> anyhow::Result<()> {
    let nonce = std::env::var(A2_DYNAMIC_CHILD_NONCE_ENV)
        .context("read parent-created Map quotient child nonce")?;
    SanitizedChildReport::validate_root_before_exercise(&nonce, root)
        .map_err(anyhow::Error::msg)?;
    let fixture = ManagedSqliteRoutedConnectionFixture::open(root, [0xa2; 16])?;
    let journal_mode: String =
        fixture
            .connection()
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    if !journal_mode.eq_ignore_ascii_case("wal") {
        return Err(anyhow!("Map quotient fixture did not enter WAL mode"));
    }
    fixture.into_schema_migration()?;
    fixture.into_runtime()?;
    if fixture
        .exact_main_shm_target_presence()
        .map_err(anyhow::Error::msg)?
    {
        return Err(anyhow!(
            "Map quotient target existed before its first xShmMap"
        ));
    }
    let map = fixture
        .call_main_shm_map_raw(
            binding.request_budget.region(),
            binding.request_budget.region_size(),
            binding.mode.raw_extend(),
        )
        .map_err(anyhow::Error::msg)?;
    let witness = fixture
        .installed_shm_fault_witness()
        .map_err(anyhow::Error::msg)?;
    let target = witness.target_witness().map_err(anyhow::Error::msg)?;
    let after = witness.observer().map_err(anyhow::Error::msg)?.snapshot()?;
    validate_native_map(binding.mode, binding.request_budget, map, after)?;
    let autocommit = fixture.connection().is_autocommit();
    let liveness: i64 = fixture
        .connection()
        .query_row("SELECT 1", [], |row| row.get(0))?;
    fixture.close()?;
    if !autocommit || liveness != 1 || !root.is_dir() {
        return Err(anyhow!("Map quotient fixture terminal state mismatch"));
    }
    let payload = encode_payload(binding, target, map, after);
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

fn validate_native_map(
    mode: MapRunnerModeV1,
    request_budget: MapRunnerRequestBudgetV1,
    map: super::super::connection::ManagedTestShmMapCallbackObservation,
    after: ManagedSqliteShmTestTargetSnapshot,
) -> anyhow::Result<()> {
    let before_slots = map.before();
    let after_slots = map.after();
    if map.region() != request_budget.region()
        || map.region_size() != request_budget.region_size()
        || map.raw_extend() != mode.raw_extend()
        || map.result_code() != ffi::SQLITE_IOERR_SHMMAP
        || !map.output_was_cleared()
        || !before_slots.methods_installed
        || !before_slots.state_installed
        || !after_slots.methods_installed
        || !after_slots.state_installed
        || !clean_post_topology(after)
    {
        return Err(anyhow!("Map quotient native xShmMap receipt mismatch"));
    }
    Ok(())
}

fn clean_post_topology(value: ManagedSqliteShmTestTargetSnapshot) -> bool {
    let topology = value.topology;
    value.target_attached
        && value.shared_mask == 0
        && value.exclusive_mask == 0
        && topology.shm_connections == 1
        && !topology.node_present
        && topology.views == 0
        && topology.mappings == 0
        && topology.dms == ManagedSqliteShmTestDmsCustody::Absent
        && !topology.shm_file_present
        && !topology.poisoned
        && !topology.mutation_may_have_occurred
        && !topology.lock_outcome_uncertain
        && !topology.domain_terminal
        && topology.quarantined_file_closes == 0
}

fn encode_payload(
    binding: MapRunnerProgramBindingV1,
    target: super::super::shm_fault_script::ManagedTestShmTargetWitness,
    map: super::super::connection::ManagedTestShmMapCallbackObservation,
    after: ManagedSqliteShmTestTargetSnapshot,
) -> String {
    let mut values = binding_values(binding);
    values.extend([
        target.registration_id(),
        target.route_ordinal(),
        target.runtime_generation(),
        target.shm_connection_id(),
        map.region() as u64,
        map.region_size() as u64,
        map.raw_extend() as u64,
        map.result_code() as u64,
        u64::from(map.output_was_cleared()),
        u64::from(map.before().methods_installed),
        u64::from(map.before().state_installed),
        u64::from(map.after().methods_installed),
        u64::from(map.after().state_installed),
    ]);
    values.extend([0; 14]);
    values.extend(topology_values(after));
    values.extend([1, 1, 1, 1]);
    debug_assert_eq!(values.len(), PAYLOAD_VALUE_COUNT);
    format!(
        "{PAYLOAD_VERSION},{},{}",
        binding.request_budget.selector(),
        values
            .into_iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn validate_payload(payload: &str, binding: MapRunnerProgramBindingV1) -> anyhow::Result<u64> {
    let mut fields = payload.split(',');
    if fields.next() != Some(PAYLOAD_VERSION)
        || fields.next() != Some(binding.request_budget.selector())
    {
        return Err(anyhow!("Map quotient payload identity mismatch"));
    }
    let values = fields
        .map(|value| value.parse::<u64>())
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() != PAYLOAD_VALUE_COUNT || values[..22] != binding_values(binding) {
        return Err(anyhow!("Map quotient payload program binding mismatch"));
    }
    let expected_request = [
        binding.request_budget.region() as u64,
        binding.request_budget.region_size() as u64,
        binding.mode.raw_extend() as u64,
        ffi::SQLITE_IOERR_SHMMAP as u64,
        1,
        1,
        1,
        1,
        1,
    ];
    let expected_after = [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    if values[22..26].contains(&0)
        || values[26..35] != expected_request
        || values[35..49] != [0; 14]
        || values[49..63] != expected_after
        || values[63..] != [1, 1, 1, 1]
    {
        return Err(anyhow!("Map quotient payload native receipt mismatch"));
    }
    Ok(values[22])
}

fn binding_values(binding: MapRunnerProgramBindingV1) -> Vec<u64> {
    let mut values = vec![binding.mode.tag(), binding.request_budget.tag()];
    for digest in [
        binding.normalized_descriptor_sha256,
        binding.case_key_sha256,
        binding.full_record_sha256,
        binding.plan_sha256,
        binding.implementation_sha256,
    ] {
        for chunk in digest.chunks_exact(8) {
            values.push(u64::from_le_bytes(chunk.try_into().expect("digest chunk")));
        }
    }
    values
}

fn topology_values(value: ManagedSqliteShmTestTargetSnapshot) -> [u64; 14] {
    let topology = value.topology;
    [
        u64::from(value.target_attached),
        topology.shm_connections.into(),
        u64::from(topology.node_present),
        topology.views.into(),
        topology.mappings.into(),
        0,
        u64::from(topology.shm_file_present),
        u64::from(topology.poisoned),
        u64::from(topology.mutation_may_have_occurred),
        u64::from(topology.lock_outcome_uncertain),
        u64::from(topology.domain_terminal),
        topology.quarantined_file_closes.into(),
        value.shared_mask.into(),
        value.exclusive_mask.into(),
    ]
}

fn create_private_child_root() -> anyhow::Result<PathBuf> {
    let requested = std::env::temp_dir().join(format!(
        "elon-a2-map-quotient-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir(&requested).context("create fresh parent-owned Map quotient root")?;
    match fs::canonicalize(&requested) {
        Ok(root) if root.is_absolute() => Ok(root),
        Ok(_) => Err(cleanup_failed_root(
            &requested,
            anyhow!("canonical Map quotient root is not absolute"),
        )),
        Err(error) => Err(cleanup_failed_root(&requested, anyhow!(error))),
    }
}

fn handle_child_failure(root: &Path, failure: DynamicChildFailure) -> anyhow::Error {
    let exit_confirmed = failure.exit_confirmed();
    let error = anyhow!(failure);
    if exit_confirmed {
        cleanup_failed_root(root, error)
    } else {
        error.context("retained Map quotient root because child exit is unconfirmed")
    }
}

fn cleanup_failed_root(root: &Path, error: anyhow::Error) -> anyhow::Error {
    match fs::remove_dir_all(root) {
        Ok(()) => error,
        Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => error,
        Err(cleanup) => error.context(format!("Map quotient fallback cleanup failed: {cleanup}")),
    }
}

fn digest_native_receipt(payload: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-map-quotient-native-receipt-v1\0");
    hasher.update((payload.len() as u64).to_le_bytes());
    hasher.update(payload.as_bytes());
    hasher.finalize().into()
}

fn digest_environment(value: &WindowsDynamicEnvironment) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-map-quotient-environment-v1\0");
    for field in [
        value.git_sha.as_str(),
        value.target,
        value.windows_build.as_str(),
        value.architecture,
        value.volume_kind,
        value.filesystem.as_str(),
        value.bundled_sqlite.as_str(),
    ] {
        hasher.update((field.len() as u64).to_le_bytes());
        hasher.update(field.as_bytes());
    }
    hasher.update(value.root_commitment.0);
    hasher.update(value.child_fingerprint.0);
    hasher.update(value.registration_commitment.0);
    hasher.finalize().into()
}

fn digest_cleanup(value: &ValidatedParentCleanupReceipt) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-map-quotient-parent-cleanup-v1\0");
    hasher.update(value.child_fingerprint.0);
    hasher.update(value.root_commitment.0);
    hasher.update(value.registration_commitment.0);
    hasher.finalize().into()
}
