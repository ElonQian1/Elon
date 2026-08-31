//! Sealed admission bridge for the one executable Map dynamic-quotient program.

#[cfg(windows)]
use std::fmt;

use sha2::{Digest, Sha256};

use super::super::super::{
    source_leaf_authority::{
        CustodyStateV1, Digest32, DmsLockCustodyV1, FailureClassV1, LockEffectV1, MutationStateV1,
        ObservableCountsV1, RootOperationV1, SqliteResultV1, TerminalDispositionV1,
    },
    terminal_descriptor::{
        CallbackV1, CleanupV1, FaultSeamV1, FixtureV1, MapCompletionV1, MapManagedStimulusV1,
        MapModeV1, MapOperationV1, MapPrestateV1, ObserverV1, OccurrenceV1, PhaseV1, PrestateV1,
        ReachabilityV1, RunnerCapabilityV1, SourceSiteV1, StimulusV1, TimingV1,
    },
};
use super::super::{
    DynamicAxesV1, DynamicClassKeyV1, DynamicExpectedV1, DynamicOperationV1, StaticMemberSealV1,
    DYNAMIC_PROJECTOR_SCHEMA_V1,
};
use super::CompiledRunnerPlanV1;

#[cfg(windows)]
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    run_map_program_isolated, MapRunnerEvidenceReceiptV1, MapRunnerIsolatedEvidenceV1,
    MapRunnerModeV1, MapRunnerProgramBindingV1,
};

/// A real execution receipt. Private fields and the absent public constructor prevent digest-only
/// callers from converting a capability declaration or compiled plan into execution authority.
pub(in super::super) struct MapRunnerExecutionReceiptV1 {
    normalized_descriptor_sha256: Digest32,
    member: StaticMemberSealV1,
    plan_sha256: Digest32,
    implementation_sha256: Digest32,
    root_commitment_sha256: Digest32,
    child_fingerprint_sha256: Digest32,
    registration_commitment_sha256: Digest32,
    payload_commitment_sha256: Digest32,
    environment_sha256: Digest32,
    cleanup_sha256: Digest32,
    native_receipt_sha256: Digest32,
    child_exit_code: i32,
    execution_sha256: Digest32,
}

pub(super) struct ValidatedMapRunnerExecutionV1 {
    implementation_sha256: Digest32,
    execution_sha256: Digest32,
}

impl ValidatedMapRunnerExecutionV1 {
    pub(super) const fn implementation_sha256(&self) -> Digest32 {
        self.implementation_sha256
    }

    pub(super) const fn execution_sha256(&self) -> Digest32 {
        self.execution_sha256
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MapRunnerExecutionViolationV1 {
    UnsupportedProgram,
    PlanBindingMismatch,
    ReceiptBindingMismatch,
    ExecutionSealMismatch,
}

#[cfg(windows)]
pub(in super::super) enum MapRunnerIsolatedOutcomeV1 {
    ParentReceipt(MapRunnerExecutionReceiptV1),
    ChildReported,
}

#[cfg(windows)]
#[derive(Debug)]
pub(in super::super) struct MapRunnerExecutionErrorV1(String);

#[cfg(windows)]
impl fmt::Display for MapRunnerExecutionErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(windows)]
impl std::error::Error for MapRunnerExecutionErrorV1 {}

pub(super) fn validate_execution_receipt_v1(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    plan: CompiledRunnerPlanV1,
    receipt: MapRunnerExecutionReceiptV1,
) -> Result<ValidatedMapRunnerExecutionV1, MapRunnerExecutionViolationV1> {
    let program = program_v1(key, member, plan)?;
    if receipt.normalized_descriptor_sha256 != program.normalized_descriptor_sha256
        || receipt.member != program.member
        || receipt.plan_sha256 != program.plan_sha256
        || receipt.implementation_sha256 != program.implementation_sha256
        || receipt.root_commitment_sha256 == Digest32::ZERO
        || receipt.child_fingerprint_sha256 == Digest32::ZERO
        || receipt.registration_commitment_sha256 == Digest32::ZERO
        || receipt.payload_commitment_sha256 == Digest32::ZERO
        || receipt.environment_sha256 == Digest32::ZERO
        || receipt.cleanup_sha256 == Digest32::ZERO
        || receipt.native_receipt_sha256 == Digest32::ZERO
        || receipt.child_exit_code != 0
    {
        return Err(MapRunnerExecutionViolationV1::ReceiptBindingMismatch);
    }
    let execution_sha256 = digest_execution_receipt_v1(&receipt);
    if receipt.execution_sha256 != execution_sha256 {
        return Err(MapRunnerExecutionViolationV1::ExecutionSealMismatch);
    }
    Ok(ValidatedMapRunnerExecutionV1 {
        implementation_sha256: receipt.implementation_sha256,
        execution_sha256,
    })
}

#[cfg(windows)]
pub(in super::super) fn run_isolated_for_test(
    exact_test: &str,
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    plan: CompiledRunnerPlanV1,
) -> Result<MapRunnerIsolatedOutcomeV1, MapRunnerExecutionErrorV1> {
    let program = program_v1(key, member, plan)
        .map_err(|violation| MapRunnerExecutionErrorV1(format!("{violation:?}")))?;
    let binding = MapRunnerProgramBindingV1 {
        mode: match program.mode {
            ProgramModeV1::Observe => MapRunnerModeV1::Observe,
            ProgramModeV1::Extend => MapRunnerModeV1::Extend,
        },
        normalized_descriptor_sha256: program.normalized_descriptor_sha256.0,
        case_key_sha256: member.case_key_sha256.0,
        full_record_sha256: member.full_record_sha256.0,
        plan_sha256: program.plan_sha256.0,
        implementation_sha256: program.implementation_sha256.0,
    };
    match run_map_program_isolated(exact_test, binding)
        .map_err(|error| MapRunnerExecutionErrorV1(error.to_string()))?
    {
        MapRunnerIsolatedEvidenceV1::ChildReported => Ok(MapRunnerIsolatedOutcomeV1::ChildReported),
        MapRunnerIsolatedEvidenceV1::ParentReceipt(evidence) => Ok(
            MapRunnerIsolatedOutcomeV1::ParentReceipt(seal_execution_receipt(program, evidence)),
        ),
    }
}

#[cfg(all(test, windows))]
pub(in super::super) fn tamper_implementation_digest_for_test(
    receipt: &mut MapRunnerExecutionReceiptV1,
    digest: Digest32,
) {
    receipt.implementation_sha256 = digest;
}

#[derive(Clone, Copy)]
enum ProgramModeV1 {
    Observe,
    Extend,
}

#[derive(Clone, Copy)]
struct MapProgramV1 {
    mode: ProgramModeV1,
    normalized_descriptor_sha256: Digest32,
    member: StaticMemberSealV1,
    plan_sha256: Digest32,
    implementation_sha256: Digest32,
}

fn program_v1(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    plan: CompiledRunnerPlanV1,
) -> Result<MapProgramV1, MapRunnerExecutionViolationV1> {
    if plan != super::compile_v1(key) {
        return Err(MapRunnerExecutionViolationV1::PlanBindingMismatch);
    }
    let DynamicAxesV1::Map(axes) = key.axes else {
        return Err(MapRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let mode = match axes.mode {
        ReachabilityV1::Reached(MapModeV1::Observe) => ProgramModeV1::Observe,
        ReachabilityV1::Reached(MapModeV1::Extend) => ProgramModeV1::Extend,
        ReachabilityV1::NotReached => {
            return Err(MapRunnerExecutionViolationV1::UnsupportedProgram)
        }
    };
    if key.schema_version != DYNAMIC_PROJECTOR_SCHEMA_V1
        || key.root != RootOperationV1::Map
        || plan.root != RootOperationV1::Map
        || key.source_site != SourceSiteV1::ManagedRequestValidation
        || key.stimulus != StimulusV1::MapManaged(MapManagedStimulusV1::RegionCountBudget)
        || key.prestate != PrestateV1::Map(MapPrestateV1::NotReached)
        || key.operation != DynamicOperationV1::Map(MapOperationV1::ManagedRequest)
        || key.phase != PhaseV1::RequestValidation
        || key.timing != TimingV1::BeforeCall
        || key.occurrence != OccurrenceV1::Natural
        || key.recipe.fixture != FixtureV1::ManagedWalMainSingleConnection
        || key.recipe.callback != CallbackV1::XShmMap
        || key.recipe.fault_seam != FaultSeamV1::ManagedRequest
        || key.recipe.observer != ObserverV1::MapCallbackAndSnapshot
        || key.recipe.cleanup != CleanupV1::ParentOwnedRoot
        || key.recipe.capability != RunnerCapabilityV1::Supported
        || axes.profile != ReachabilityV1::NotReached
        || axes.ordinal != ReachabilityV1::NotReached
        || axes.regions_to_create != ReachabilityV1::NotReached
        || axes.completion != ReachabilityV1::Reached(MapCompletionV1::Completed)
        || key.expected != expected_v1()
    {
        return Err(MapRunnerExecutionViolationV1::UnsupportedProgram);
    }
    Ok(MapProgramV1 {
        mode,
        normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
        member,
        plan_sha256: plan.plan_sha256,
        implementation_sha256: digest_implementation_v1(),
    })
}

#[cfg(windows)]
fn seal_execution_receipt(
    program: MapProgramV1,
    evidence: MapRunnerEvidenceReceiptV1,
) -> MapRunnerExecutionReceiptV1 {
    let (root, child, registration, payload, environment, cleanup, native, exit_code) =
        evidence.into_bindings();
    let mut receipt = MapRunnerExecutionReceiptV1 {
        normalized_descriptor_sha256: program.normalized_descriptor_sha256,
        member: program.member,
        plan_sha256: program.plan_sha256,
        implementation_sha256: program.implementation_sha256,
        root_commitment_sha256: Digest32(root),
        child_fingerprint_sha256: Digest32(child),
        registration_commitment_sha256: Digest32(registration),
        payload_commitment_sha256: Digest32(payload),
        environment_sha256: Digest32(environment),
        cleanup_sha256: Digest32(cleanup),
        native_receipt_sha256: Digest32(native),
        child_exit_code: exit_code,
        execution_sha256: Digest32::ZERO,
    };
    receipt.execution_sha256 = digest_execution_receipt_v1(&receipt);
    receipt
}

fn digest_implementation_v1() -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-map-region-count-budget-completed-implementation-v1\0");
    for source in [
        include_str!("map_program.rs"),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/map_runner.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/child.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/child/payload.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/capture.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/environment.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/cleanup.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/connection.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/connection/unmap.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/callbacks.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/route_file.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/fault_script/file.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/test_vfs_bridge/file.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/abi.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/operations.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/io_shm.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/file_state.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/types.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/mapping.rs"
        )),
    ] {
        hasher.update((source.len() as u64).to_le_bytes());
        hasher.update(source.as_bytes());
    }
    hasher.update(256u32.to_le_bytes());
    hasher.update(32_768u32.to_le_bytes());
    Digest32(hasher.finalize().into())
}

fn expected_v1() -> DynamicExpectedV1 {
    DynamicExpectedV1 {
        sqlite: SqliteResultV1::MapUnavailable,
        disposition: TerminalDispositionV1::Returned,
        phase: PhaseV1::RequestValidation,
        failure: FailureClassV1::ProtocolViolation,
        mutation: MutationStateV1::None,
        lock_outcome_uncertain: false,
        lock_effect: LockEffectV1::NotReached,
        dms_lock: DmsLockCustodyV1::NotReached,
        raw_slots: CustodyStateV1::Unchanged,
        route: CustodyStateV1::Unchanged,
        callback: CustodyStateV1::Released,
        file: CustodyStateV1::Retained,
        mapping: CustodyStateV1::NotReached,
        view: CustodyStateV1::NotReached,
        payload: CustodyStateV1::NotReached,
        counts: ObservableCountsV1 {
            callback_begin: 1,
            callback_complete: 1,
            ..ObservableCountsV1::default()
        },
    }
}

fn digest_execution_receipt_v1(receipt: &MapRunnerExecutionReceiptV1) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-map-quotient-real-execution-v1\0");
    for digest in [
        receipt.normalized_descriptor_sha256,
        receipt.member.case_key_sha256,
        receipt.member.full_record_sha256,
        receipt.plan_sha256,
        receipt.implementation_sha256,
        receipt.root_commitment_sha256,
        receipt.child_fingerprint_sha256,
        receipt.registration_commitment_sha256,
        receipt.payload_commitment_sha256,
        receipt.environment_sha256,
        receipt.cleanup_sha256,
        receipt.native_receipt_sha256,
    ] {
        hasher.update(digest.0);
    }
    hasher.update(receipt.child_exit_code.to_le_bytes());
    Digest32(hasher.finalize().into())
}
