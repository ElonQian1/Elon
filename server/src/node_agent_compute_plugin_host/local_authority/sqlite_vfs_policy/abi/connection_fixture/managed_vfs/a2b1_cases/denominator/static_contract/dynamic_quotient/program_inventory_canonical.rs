//! Canonical commitments for the non-authorizing execution-program inventory.

use sha2::{Digest as _, Sha256};

use super::super::source_leaf_authority::{Digest32, RootOperationV1};
use super::canonical_tags::gap_tag;
use super::program_inventory::{
    ExecutionProgramGroupV1, ExecutionProgramInventoryV1, ExecutionProgramMembershipV1,
    EXECUTION_PROGRAM_INVENTORY_SCHEMA_V1,
};
use super::runner_admission::{
    ExecutionProgramInventoryStatusV1, ABI_SCALAR_REJECTION_PROJECTOR_DELTA_V1,
    NATIVE_ACQUIRE_CREATED_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1,
    NATIVE_ACQUIRE_CREATED_FIRST_TRUNCATE_ERROR_RELEASE_SUCCEEDED_PROJECTOR_DELTA_V1,
    NATIVE_ACQUIRE_EXISTING_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1,
    PRE_MANAGED_CALLBACK_REJECTION_PROJECTOR_DELTA_V1, RAW_STATE_REJECTION_PROJECTOR_DELTA_V1,
};

const SOURCE_SCOPE_DOMAIN: &str = "ELON-A2-MAP-LOCK-EXECUTION-PROGRAM-INVENTORY-SOURCE-SCOPE-V1";
const MEMBERSHIP_DOMAIN: &str = "ELON-A2-MAP-LOCK-EXECUTION-PROGRAM-MEMBERSHIP-V1";
const CATALOG_DOMAIN: &str = "ELON-A2-MAP-LOCK-EXECUTION-PROGRAM-CATALOG-V1";
const INVENTORY_DOMAIN: &str = "ELON-A2-MAP-LOCK-EXECUTION-PROGRAM-INVENTORY-V1";

const SOURCE_SCOPE: &[(&str, &str)] = &[
    ("program_inventory.rs", include_str!("program_inventory.rs")),
    (
        "program_inventory/builder.rs",
        include_str!("program_inventory/builder.rs"),
    ),
    (
        "program_inventory/model.rs",
        include_str!("program_inventory/model.rs"),
    ),
    (
        "program_inventory_canonical.rs",
        include_str!("program_inventory_canonical.rs"),
    ),
    ("projector.rs", include_str!("projector.rs")),
    ("runner_admission.rs", include_str!("runner_admission.rs")),
    (
        "runner_admission/map.rs",
        include_str!("runner_admission/map.rs"),
    ),
    (
        "runner_admission/lock.rs",
        include_str!("runner_admission/lock.rs"),
    ),
    (
        "runner_admission/map_program.rs",
        include_str!("runner_admission/map_program.rs"),
    ),
    (
        "runner_admission/map_program/request_budget.rs",
        include_str!("runner_admission/map_program/request_budget.rs"),
    ),
    (
        "runner_admission/map_program/lifecycle.rs",
        include_str!("runner_admission/map_program/lifecycle.rs"),
    ),
    (
        "runner_admission/map_program/lifecycle/source_scope.rs",
        include_str!("runner_admission/map_program/lifecycle/source_scope.rs"),
    ),
    (
        "runner_admission/map_program/region_loop.rs",
        include_str!("runner_admission/map_program/region_loop.rs"),
    ),
    (
        "runner_admission/map_program/region_loop/catalog.rs",
        include_str!("runner_admission/map_program/region_loop/catalog.rs"),
    ),
    (
        "runner_admission/map_program/region_loop/region_loop_members.v1.tsv",
        include_str!("runner_admission/map_program/region_loop/region_loop_members.v1.tsv"),
    ),
    (
        "runner_admission/map_program/region_loop/source_scope.rs",
        include_str!("runner_admission/map_program/region_loop/source_scope.rs"),
    ),
    (
        "runner_admission/lock_program.rs",
        include_str!("runner_admission/lock_program.rs"),
    ),
    (
        "runner_admission/lock_program/execution_receipt.rs",
        include_str!("runner_admission/lock_program/execution_receipt.rs"),
    ),
    (
        "runner_admission/lock_program/request_validation.rs",
        include_str!("runner_admission/lock_program/request_validation.rs"),
    ),
    (
        "runner_admission/lock_program/lifecycle.rs",
        include_str!("runner_admission/lock_program/lifecycle.rs"),
    ),
    (
        "runner_admission/lock_program/source_program.rs",
        include_str!("runner_admission/lock_program/source_program.rs"),
    ),
    (
        "runner_admission/lock_program/callback_completion_route_unknown.rs",
        include_str!("runner_admission/lock_program/callback_completion_route_unknown.rs"),
    ),
    (
        "runner_admission/lock_program/callback_completion_route_unknown/catalog.rs",
        include_str!("runner_admission/lock_program/callback_completion_route_unknown/catalog.rs"),
    ),
    (
        "runner_admission/lock_program/callback_completion_route_unknown/runtime.rs",
        include_str!("runner_admission/lock_program/callback_completion_route_unknown/runtime.rs"),
    ),
    (
        "runner_admission/lock_program/callback_completion_route_unknown/callback_completion_route_unknown_members.v1.tsv",
        include_str!("runner_admission/lock_program/callback_completion_route_unknown/callback_completion_route_unknown_members.v1.tsv"),
    ),
    (
        "runner_admission/lock_program/callback_completion_route_unknown/source_scope.rs",
        include_str!("runner_admission/lock_program/callback_completion_route_unknown/source_scope.rs"),
    ),
    (
        "lock_callback_completion_route_unknown_source_scope.rs",
        include_str!("lock_callback_completion_route_unknown_source_scope.rs"),
    ),
    (
        "runner_admission/lock_program/local_sibling_contention.rs",
        include_str!("runner_admission/lock_program/local_sibling_contention.rs"),
    ),
    (
        "runner_admission/lock_program/local_sibling_contention/catalog.rs",
        include_str!("runner_admission/lock_program/local_sibling_contention/catalog.rs"),
    ),
    (
        "runner_admission/lock_program/local_sibling_contention/local_sibling_contention_completed_members.v1.tsv",
        include_str!("runner_admission/lock_program/local_sibling_contention/local_sibling_contention_completed_members.v1.tsv"),
    ),
    (
        "runner_admission/lock_program/local_sibling_contention/source_scope.rs",
        include_str!("runner_admission/lock_program/local_sibling_contention/source_scope.rs"),
    ),
    (
        "lock_local_sibling_contention_source_scope.rs",
        include_str!("lock_local_sibling_contention_source_scope.rs"),
    ),
    (
        "runner_admission/lock_program/local_protocol_rejection.rs",
        include_str!("runner_admission/lock_program/local_protocol_rejection.rs"),
    ),
    (
        "runner_admission/lock_program/local_protocol_rejection/catalog.rs",
        include_str!("runner_admission/lock_program/local_protocol_rejection/catalog.rs"),
    ),
    (
        "runner_admission/lock_program/local_protocol_rejection/runtime.rs",
        include_str!("runner_admission/lock_program/local_protocol_rejection/runtime.rs"),
    ),
    (
        "runner_admission/lock_program/local_protocol_rejection/local_protocol_own_overlap_or_not_held_completed_members.v1.tsv",
        include_str!("runner_admission/lock_program/local_protocol_rejection/local_protocol_own_overlap_or_not_held_completed_members.v1.tsv"),
    ),
    (
        "runner_admission/lock_program/local_protocol_rejection/source_scope.rs",
        include_str!("runner_admission/lock_program/local_protocol_rejection/source_scope.rs"),
    ),
    (
        "lock_local_protocol_rejection_source_scope.rs",
        include_str!("lock_local_protocol_rejection_source_scope.rs"),
    ),
    (
        "runner_admission/lock_program/native_acquire_busy.rs",
        include_str!("runner_admission/lock_program/native_acquire_busy.rs"),
    ),
    (
        "runner_admission/lock_program/native_acquire_busy/catalog.rs",
        include_str!("runner_admission/lock_program/native_acquire_busy/catalog.rs"),
    ),
    (
        "runner_admission/lock_program/native_acquire_busy/native_acquire_node_live_native_busy_completed_members.v1.tsv",
        include_str!("runner_admission/lock_program/native_acquire_busy/native_acquire_node_live_native_busy_completed_members.v1.tsv"),
    ),
    (
        "runner_admission/lock_program/native_acquire_busy/source_scope.rs",
        include_str!("runner_admission/lock_program/native_acquire_busy/source_scope.rs"),
    ),
    (
        "lock_native_acquire_busy_source_scope.rs",
        include_str!("lock_native_acquire_busy_source_scope.rs"),
    ),
    (
        "runner_admission/lock_program/stored_poison.rs",
        include_str!("runner_admission/lock_program/stored_poison.rs"),
    ),
    (
        "runner_admission/lock_program/stored_poison/catalog.rs",
        include_str!("runner_admission/lock_program/stored_poison/catalog.rs"),
    ),
    (
        "runner_admission/lock_program/stored_poison/stored_poison_retention_succeeded_members.v1.tsv",
        include_str!("runner_admission/lock_program/stored_poison/stored_poison_retention_succeeded_members.v1.tsv"),
    ),
    (
        "runner_admission/lock_program/stored_poison/stored_poison_retention_route_unknown_members.v1.tsv",
        include_str!("runner_admission/lock_program/stored_poison/stored_poison_retention_route_unknown_members.v1.tsv"),
    ),
    (
        "runner_admission/lock_program/stored_poison/source_scope.rs",
        include_str!("runner_admission/lock_program/stored_poison/source_scope.rs"),
    ),
    (
        "runner_admission/canonical.rs",
        include_str!("runner_admission/canonical.rs"),
    ),
];

pub(super) fn digest_execution_program_inventory_source_scope_v1() -> Digest32 {
    let mut out = Sha256::new();
    add_bytes(&mut out, "domain", SOURCE_SCOPE_DOMAIN.as_bytes());
    add_u16(
        &mut out,
        "schema_version",
        EXECUTION_PROGRAM_INVENTORY_SCHEMA_V1,
    );
    add_u64(
        &mut out,
        "entry_count",
        (SOURCE_SCOPE.len()
            + PRE_MANAGED_CALLBACK_REJECTION_PROJECTOR_DELTA_V1.len()
            + ABI_SCALAR_REJECTION_PROJECTOR_DELTA_V1.len()
            + RAW_STATE_REJECTION_PROJECTOR_DELTA_V1.len()
            + NATIVE_ACQUIRE_CREATED_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1.len()
            + NATIVE_ACQUIRE_EXISTING_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1.len()
            + NATIVE_ACQUIRE_CREATED_FIRST_TRUNCATE_ERROR_RELEASE_SUCCEEDED_PROJECTOR_DELTA_V1
                .len()) as u64,
    );
    for (path, source) in SOURCE_SCOPE
        .iter()
        .copied()
        .chain(
            PRE_MANAGED_CALLBACK_REJECTION_PROJECTOR_DELTA_V1
                .iter()
                .copied()
                .map(|(path, source)| {
                    (
                        path.strip_prefix("dynamic_quotient/").unwrap_or(path),
                        source,
                    )
                }),
        )
        .chain(
            ABI_SCALAR_REJECTION_PROJECTOR_DELTA_V1
                .iter()
                .copied()
                .map(|(path, source)| {
                    (
                        path.strip_prefix("dynamic_quotient/").unwrap_or(path),
                        source,
                    )
                }),
        )
        .chain(
            RAW_STATE_REJECTION_PROJECTOR_DELTA_V1
                .iter()
                .copied()
                .map(|(path, source)| {
                    (
                        path.strip_prefix("dynamic_quotient/").unwrap_or(path),
                        source,
                    )
                }),
        )
        .chain(
            NATIVE_ACQUIRE_CREATED_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1
                .iter()
                .copied()
                .map(|(path, source)| {
                    (
                        path.strip_prefix("dynamic_quotient/").unwrap_or(path),
                        source,
                    )
                }),
        )
        .chain(
            NATIVE_ACQUIRE_EXISTING_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1
                .iter()
                .copied()
                .map(|(path, source)| {
                    (
                        path.strip_prefix("dynamic_quotient/").unwrap_or(path),
                        source,
                    )
                }),
        )
        .chain(
            NATIVE_ACQUIRE_CREATED_FIRST_TRUNCATE_ERROR_RELEASE_SUCCEEDED_PROJECTOR_DELTA_V1
                .iter()
                .copied()
                .map(|(path, source)| {
                    (
                        path.strip_prefix("dynamic_quotient/").unwrap_or(path),
                        source,
                    )
                }),
        )
    {
        add_bytes(&mut out, "path", path.as_bytes());
        add_bytes(&mut out, "source", source.as_bytes());
    }
    Digest32(out.finalize().into())
}

pub(super) fn digest_execution_program_membership_v1(
    root: RootOperationV1,
    entries: &[ExecutionProgramMembershipV1],
) -> Digest32 {
    let mut entries = entries.to_vec();
    entries.sort_unstable();
    let mut out = Sha256::new();
    add_bytes(&mut out, "domain", MEMBERSHIP_DOMAIN.as_bytes());
    add_u16(
        &mut out,
        "schema_version",
        EXECUTION_PROGRAM_INVENTORY_SCHEMA_V1,
    );
    add_bytes(&mut out, "root", root.canonical_name().as_bytes());
    add_u64(&mut out, "entry_count", entries.len() as u64);
    for entry in entries {
        add_member(&mut out, "member", entry.member);
        add_digest(&mut out, "program_id", entry.program_id);
    }
    Digest32(out.finalize().into())
}

pub(super) fn digest_execution_program_catalog_v1(
    root: RootOperationV1,
    groups: &[ExecutionProgramGroupV1],
) -> Digest32 {
    let mut groups = groups.iter().collect::<Vec<_>>();
    groups.sort_by_key(|group| group.program_id);
    let mut out = Sha256::new();
    add_bytes(&mut out, "domain", CATALOG_DOMAIN.as_bytes());
    add_u16(
        &mut out,
        "schema_version",
        EXECUTION_PROGRAM_INVENTORY_SCHEMA_V1,
    );
    add_bytes(&mut out, "root", root.canonical_name().as_bytes());
    add_u64(&mut out, "program_group_count", groups.len() as u64);
    for group in groups {
        add_digest(&mut out, "program_id", group.program_id);
        add_digest(&mut out, "plan_sha256", group.plan_sha256);
        match group.status {
            ExecutionProgramInventoryStatusV1::PlannedMissing(gap) => {
                add_u16(&mut out, "status", 1);
                add_u16(&mut out, "gap", gap_tag(gap));
            }
            ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired {
                implementation_sha256,
            } => {
                add_u16(&mut out, "status", 2);
                add_digest(&mut out, "implementation_sha256", implementation_sha256);
            }
        }
        add_u64(&mut out, "member_count", group.member_count);
        add_digest(&mut out, "member_set_sha256", group.member_set_sha256);
    }
    Digest32(out.finalize().into())
}

pub(super) fn digest_execution_program_inventory_body_v1(
    value: &ExecutionProgramInventoryV1,
) -> Digest32 {
    let context = &value.context;
    let mut out = Sha256::new();
    add_bytes(&mut out, "domain", INVENTORY_DOMAIN.as_bytes());
    add_u16(&mut out, "schema_version", context.schema_version);
    add_bytes(&mut out, "root", context.root.canonical_name().as_bytes());
    add_bytes(
        &mut out,
        "static_source_baseline_sha1",
        context.static_source_baseline_sha1.as_bytes(),
    );
    for (label, digest) in [
        (
            "static_source_scope_sha256",
            context.static_source_scope_sha256,
        ),
        ("static_ledger_sha256", context.static_ledger_sha256),
        ("static_manifest_sha256", context.static_manifest_sha256),
        (
            "static_member_pair_set_sha256",
            context.static_member_pair_set_sha256,
        ),
        ("projector_schema_sha256", context.projector_schema_sha256),
        (
            "projector_source_scope_sha256",
            context.projector_source_scope_sha256,
        ),
        (
            "descriptor_binding_sha256",
            context.descriptor_binding_sha256,
        ),
        (
            "inventory_source_scope_sha256",
            context.inventory_source_scope_sha256,
        ),
        ("membership_sha256", value.membership_sha256),
        ("program_catalog_sha256", value.program_catalog_sha256),
    ] {
        add_digest(&mut out, label, digest);
    }
    for (label, count) in [
        ("static_included_count", context.static_included_count),
        ("static_excluded_count", context.static_excluded_count),
        (
            "static_source_universe_count",
            context.static_source_universe_count,
        ),
        ("member_count", value.member_count),
        ("program_group_count", value.program_group_count),
        (
            "source_present_member_count",
            value.source_present_member_count,
        ),
        (
            "source_present_group_count",
            value.source_present_group_count,
        ),
        (
            "planned_missing_member_count",
            value.planned_missing_member_count,
        ),
        (
            "planned_missing_group_count",
            value.planned_missing_group_count,
        ),
    ] {
        add_u64(&mut out, label, count);
    }
    Digest32(out.finalize().into())
}

fn add_member(out: &mut Sha256, label: &str, member: super::StaticMemberSealV1) {
    let mut bytes = [0_u8; 64];
    bytes[..32].copy_from_slice(&member.case_key_sha256.0);
    bytes[32..].copy_from_slice(&member.full_record_sha256.0);
    add_bytes(out, label, &bytes);
}

fn add_digest(out: &mut Sha256, label: &str, value: Digest32) {
    add_bytes(out, label, &value.0);
}

fn add_u16(out: &mut Sha256, label: &str, value: u16) {
    add_bytes(out, label, &value.to_be_bytes());
}

fn add_u64(out: &mut Sha256, label: &str, value: u64) {
    add_bytes(out, label, &value.to_be_bytes());
}

fn add_bytes(out: &mut Sha256, label: &str, value: &[u8]) {
    out.update(label.as_bytes());
    out.update([0]);
    out.update((value.len() as u64).to_be_bytes());
    out.update(value);
}
