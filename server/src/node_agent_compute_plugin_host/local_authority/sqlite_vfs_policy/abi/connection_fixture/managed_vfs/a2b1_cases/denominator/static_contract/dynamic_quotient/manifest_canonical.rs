mod projector_source_scope;

use sha2::{Digest as _, Sha256};

use super::super::source_leaf_authority::Digest32;
use super::{
    digest_dynamic_class_key_v1, DynamicClassKeyV1, DynamicClassSealV1, DynamicQuotientManifestV1,
    ReverseIndexEntryV1, StaticMemberSealV1, DYNAMIC_PROJECTOR_SCHEMA_V1,
};
pub(super) use projector_source_scope::{
    digest_projector_source_entries_v1, digest_projector_source_scope_v1,
    projector_source_scope_entries_v1,
};

const MEMBER_SET_DOMAIN: &str = "ELON-A2-MAP-LOCK-DYNAMIC-MEMBER-SET-V1";
const MANIFEST_DOMAIN: &str = "ELON-A2-MAP-LOCK-DYNAMIC-QUOTIENT-MANIFEST-V1";

pub(super) const PROJECTOR_SOURCE_SCOPE_V1: &[(&str, &str)] = &[
    ("dynamic_quotient.rs", include_str!("../dynamic_quotient.rs")),
    ("terminal_descriptor.rs", include_str!("../terminal_descriptor.rs")),
    (
        "terminal_descriptor/axes.rs",
        include_str!("../terminal_descriptor/axes.rs"),
    ),
    (
        "terminal_descriptor/recipe.rs",
        include_str!("../terminal_descriptor/recipe.rs"),
    ),
    ("map/dynamic.rs", include_str!("../map/dynamic.rs")),
    ("lock/dynamic.rs", include_str!("../lock/dynamic.rs")),
    ("dynamic_quotient/model.rs", include_str!("model.rs")),
    (
        "dynamic_quotient/map_runtime_source_scope.rs",
        include_str!("map_runtime_source_scope.rs"),
    ),
    (
        "dynamic_quotient/canonical.rs",
        include_str!("canonical.rs"),
    ),
    (
        "dynamic_quotient/canonical_tags.rs",
        include_str!("canonical_tags.rs"),
    ),
    (
        "dynamic_quotient/projector.rs",
        include_str!("projector.rs"),
    ),
    (
        "dynamic_quotient/projector/lock_execution.rs",
        include_str!("projector/lock_execution.rs"),
    ),
    (
        "dynamic_quotient/producer_coherence.rs",
        include_str!("producer_coherence.rs"),
    ),
    (
        "dynamic_quotient/producer_coherence/map.rs",
        include_str!("producer_coherence/map.rs"),
    ),
    (
        "dynamic_quotient/producer_coherence/map_axes.rs",
        include_str!("producer_coherence/map_axes.rs"),
    ),
    (
        "dynamic_quotient/producer_coherence/lock.rs",
        include_str!("producer_coherence/lock.rs"),
    ),
    (
        "dynamic_quotient/producer_coherence/lock_axes.rs",
        include_str!("producer_coherence/lock_axes.rs"),
    ),
    ("dynamic_quotient/catalog.rs", include_str!("catalog.rs")),
    (
        "dynamic_quotient/catalog/finish.rs",
        include_str!("catalog/finish.rs"),
    ),
    (
        "dynamic_quotient/descriptor_binding.rs",
        include_str!("descriptor_binding.rs"),
    ),
    (
        "dynamic_quotient/runner_admission.rs",
        include_str!("runner_admission.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/canonical.rs",
        include_str!("runner_admission/canonical.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/map.rs",
        include_str!("runner_admission/map.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/map_program.rs",
        include_str!("runner_admission/map_program.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/map_program/request_budget.rs",
        include_str!("runner_admission/map_program/request_budget.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/map_program/lifecycle.rs",
        include_str!("runner_admission/map_program/lifecycle.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/map_program/lifecycle/source_scope.rs",
        include_str!("runner_admission/map_program/lifecycle/source_scope.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock.rs",
        include_str!("runner_admission/lock.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock_program.rs",
        include_str!("runner_admission/lock_program.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock_program/request_validation.rs",
        include_str!("runner_admission/lock_program/request_validation.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock_program/lifecycle.rs",
        include_str!("runner_admission/lock_program/lifecycle.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence.rs",
        include_str!("../../../../a2_dynamic_evidence.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/child.rs",
        include_str!("../../../../a2_dynamic_evidence/child.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/child/payload.rs",
        include_str!("../../../../a2_dynamic_evidence/child/payload.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/child/lock_request_validation.rs",
        include_str!("../../../../a2_dynamic_evidence/child/lock_request_validation.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/child/lock_lifecycle.rs",
        include_str!("../../../../a2_dynamic_evidence/child/lock_lifecycle.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/child/map_lifecycle.rs",
        include_str!("../../../../a2_dynamic_evidence/child/map_lifecycle.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/capture.rs",
        include_str!("../../../../a2_dynamic_evidence/capture.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/environment.rs",
        include_str!("../../../../a2_dynamic_evidence/environment.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/cleanup.rs",
        include_str!("../../../../a2_dynamic_evidence/cleanup.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/map_runner.rs",
        include_str!("../../../../a2_dynamic_evidence/map_runner.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/map_runner/lifecycle.rs",
        include_str!("../../../../a2_dynamic_evidence/map_runner/lifecycle.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/map_runner/lifecycle/fixture.rs",
        include_str!("../../../../a2_dynamic_evidence/map_runner/lifecycle/fixture.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/map_runner/lifecycle/payload.rs",
        include_str!("../../../../a2_dynamic_evidence/map_runner/lifecycle/payload.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/map_runner/request_budget.rs",
        include_str!("../../../../a2_dynamic_evidence/map_runner/request_budget.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/lock_runner.rs",
        include_str!("../../../../a2_dynamic_evidence/lock_runner.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/lock_runner/request_validation.rs",
        include_str!("../../../../a2_dynamic_evidence/lock_runner/request_validation.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/lock_runner/lifecycle.rs",
        include_str!("../../../../a2_dynamic_evidence/lock_runner/lifecycle.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/lock_runner/lifecycle/fixture.rs",
        include_str!("../../../../a2_dynamic_evidence/lock_runner/lifecycle/fixture.rs"),
    ),
    (
        "managed_vfs/a2_dynamic_evidence/lock_runner/lifecycle/payload.rs",
        include_str!("../../../../a2_dynamic_evidence/lock_runner/lifecycle/payload.rs"),
    ),
    (
        "managed_vfs.rs",
        include_str!("../../../../../managed_vfs.rs"),
    ),
    (
        "managed_vfs/connection.rs",
        include_str!("../../../../connection.rs"),
    ),
    (
        "managed_vfs/live_registration.rs",
        include_str!("../../../../live_registration.rs"),
    ),
    (
        "managed_vfs/shared_namespace.rs",
        include_str!("../../../../shared_namespace.rs"),
    ),
    (
        "managed_vfs/shm_fault_script.rs",
        include_str!("../../../../shm_fault_script.rs"),
    ),
    (
        "managed_vfs/connection/unmap.rs",
        include_str!("../../../../connection/unmap.rs"),
    ),
    (
        "managed_vfs/callbacks.rs",
        include_str!("../../../../callbacks.rs"),
    ),
    (
        "managed_vfs/route_file.rs",
        include_str!("../../../../route_file.rs"),
    ),
    (
        "managed_vfs/fault_script.rs",
        include_str!("../../../../fault_script.rs"),
    ),
    (
        "managed_vfs/fault_script/file.rs",
        include_str!("../../../../fault_script/file.rs"),
    ),
    (
        "managed_vfs/multi_connection.rs",
        include_str!("../../../../multi_connection.rs"),
    ),
    (
        "registry/test_vfs_bridge/file.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/test_vfs_bridge/file.rs"
        )),
    ),
    (
        "registry/file_custody/abi.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/abi.rs"
        )),
    ),
    (
        "registry/file_custody/operations.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/operations.rs"
        )),
    ),
    (
        "sqlite_vfs_abi.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi.rs"
        )),
    ),
    (
        "sqlite_vfs_abi/boundary.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/boundary.rs"
        )),
    ),
    (
        "sqlite_vfs_abi/io_shm.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/io_shm.rs"
        )),
    ),
    (
        "sqlite_vfs_abi/result_codes.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/result_codes.rs"
        )),
    ),
    (
        "sqlite_vfs_abi/file_state.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/file_state.rs"
        )),
    ),
    (
        "sqlite_vfs_abi/raw_state.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/raw_state.rs"
        )),
    ),
    (
        "sqlite_vfs_abi/types.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/types.rs"
        )),
    ),
    (
        "node_agent_managed_fs.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs.rs"
        )),
    ),
    (
        "node_agent_managed_fs/windows.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/windows.rs"
        )),
    ),
    (
        "node_agent_managed_fs/windows_sqlite_locking.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/windows_sqlite_locking.rs"
        )),
    ),
    (
        "node_agent_managed_fs/windows_sqlite_shm.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/windows_sqlite_shm.rs"
        )),
    ),
    (
        "node_agent_managed_fs/sqlite_api.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_api.rs"
        )),
    ),
    (
        "node_agent_managed_fs/sqlite_namespace_io.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_io.rs"
        )),
    ),
    (
        "node_agent_managed_fs/sqlite_namespace.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace.rs"
        )),
    ),
    (
        "node_agent_managed_fs/sqlite_namespace_shm.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm.rs"
        )),
    ),
    (
        "node_agent_managed_fs/sqlite_namespace_shm/coordinator.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/coordinator.rs"
        )),
    ),
    (
        "node_agent_managed_fs/sqlite_namespace_shm/types.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/types.rs"
        )),
    ),
    (
        "node_agent_managed_fs/sqlite_namespace_shm/node_initialization.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/node_initialization.rs"
        )),
    ),
    (
        "node_agent_managed_fs/sqlite_namespace_shm/mapping.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/mapping.rs"
        )),
    ),
    (
        "node_agent_managed_fs/sqlite_namespace_shm/locking.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/locking.rs"
        )),
    ),
    (
        "node_agent_managed_fs/sqlite_namespace_shm/test_faults/api.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/api.rs"
        )),
    ),
    (
        "node_agent_managed_fs/sqlite_namespace_shm/test_snapshot.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/test_snapshot.rs"
        )),
    ),
    (
        "node_agent_managed_fs/sqlite_namespace_shm/test_faults.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/test_faults.rs"
        )),
    ),
    (
        "node_agent_managed_fs/sqlite_namespace_shm/test_faults/controller.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/controller.rs"
        )),
    ),
    (
        "node_agent_managed_fs/sqlite_namespace_shm/test_faults/operation.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/operation.rs"
        )),
    ),
    (
        "node_agent_managed_fs/sqlite_namespace_shm/test_faults/mapping.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/mapping.rs"
        )),
    ),
    (
        "node_agent_managed_fs/sqlite_namespace_shm/test_map_runtime.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/test_map_runtime.rs"
        )),
    ),
    (
        "node_agent_managed_fs/sqlite_namespace_shm/test_map_runtime/mapping_sequence.rs",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/test_map_runtime/mapping_sequence.rs"
        )),
    ),
    (
        "dynamic_quotient/candidate.rs",
        include_str!("candidate.rs"),
    ),
    (
        "dynamic_quotient/program_inventory.rs",
        include_str!("program_inventory.rs"),
    ),
    (
        "dynamic_quotient/program_inventory/builder.rs",
        include_str!("program_inventory/builder.rs"),
    ),
    (
        "dynamic_quotient/program_inventory/model.rs",
        include_str!("program_inventory/model.rs"),
    ),
    (
        "dynamic_quotient/program_inventory/admission.rs",
        include_str!("program_inventory/admission.rs"),
    ),
    (
        "dynamic_quotient/program_inventory/admission/canonical.rs",
        include_str!("program_inventory/admission/canonical.rs"),
    ),
    (
        "dynamic_quotient/program_inventory/admission/validation.rs",
        include_str!("program_inventory/admission/validation.rs"),
    ),
    (
        "dynamic_quotient/program_inventory_canonical.rs",
        include_str!("program_inventory_canonical.rs"),
    ),
    ("dynamic_quotient/manifest.rs", include_str!("manifest.rs")),
    (
        "dynamic_quotient/membership_commitment.rs",
        include_str!("membership_commitment.rs"),
    ),
    (
        "dynamic_quotient/manifest_canonical.rs",
        include_str!("manifest_canonical.rs"),
    ),
    (
        "dynamic_quotient/manifest_canonical/projector_source_scope.rs",
        include_str!("manifest_canonical/projector_source_scope.rs"),
    ),
    (
        "source_leaf_authority/canonical.rs",
        include_str!("../source_leaf_authority/canonical.rs"),
    ),
    (
        "source_leaf_authority/canonical/precomputed.rs",
        include_str!("../source_leaf_authority/canonical/precomputed.rs"),
    ),
    (
        "source_leaf_authority/observer.rs",
        include_str!("../source_leaf_authority/observer.rs"),
    ),
    (
        "source_leaf_authority/adapter.rs",
        include_str!("../source_leaf_authority/adapter.rs"),
    ),
    (
        "source_leaf_authority/frozen.rs",
        include_str!("../source_leaf_authority/frozen.rs"),
    ),
];

pub(super) fn digest_member_set_v1(members: &[StaticMemberSealV1]) -> Digest32 {
    let mut sorted = members.to_vec();
    sorted.sort_unstable();
    let mut out = StableHasher::new(MEMBER_SET_DOMAIN);
    out.u64("member_count", sorted.len() as u64);
    for member in sorted {
        out.member("member", member);
    }
    out.finish()
}

pub(super) fn digest_retained_axes_v1(key: &DynamicClassKeyV1) -> Digest32 {
    let mut out = StableHasher::new("ELON-A2-MAP-LOCK-DYNAMIC-RETAINED-AXES-V1");
    out.digest("class_key_sha256", digest_dynamic_class_key_v1(key));
    out.finish()
}

pub(super) fn digest_erasure_proof_v1() -> Digest32 {
    let mut out = StableHasher::new("ELON-A2-MAP-LOCK-DYNAMIC-ERASURE-PROOF-V1");
    for axis in [
        "run-nonce",
        "temporary-root",
        "registration-id",
        "route-ordinal",
        "runtime-generation",
        "shm-connection-id",
        "child-pid",
    ] {
        out.text("alpha_renamed_axis", axis);
    }
    out.text("proof_kind", "runtime-binding-only");
    out.finish()
}

pub(super) fn digest_class_record_v1(class: &DynamicClassSealV1) -> Digest32 {
    let mut out = StableHasher::new("ELON-A2-MAP-LOCK-DYNAMIC-CLASS-RECORD-V1");
    out.digest("class_key_sha256", class.class_key_sha256);
    out.digest("class_id", class.class_id);
    out.u64("member_count", class.member_count);
    out.digest("member_set_sha256", class.member_set_sha256);
    out.member("representative", class.representative);
    out.digest("retained_axes_sha256", class.retained_axes_sha256);
    out.digest("erased_axes_proof_sha256", class.erased_axes_proof_sha256);
    out.finish()
}

pub(super) fn digest_class_key_set_v1(classes: &[DynamicClassSealV1]) -> Digest32 {
    digest_class_items(
        "ELON-A2-MAP-LOCK-DYNAMIC-CLASS-KEY-SET-V1",
        classes,
        |out, class| out.digest("class_key_sha256", class.class_key_sha256),
    )
}

pub(super) fn digest_membership_map_v1(classes: &[DynamicClassSealV1]) -> Digest32 {
    digest_class_items(
        "ELON-A2-MAP-LOCK-DYNAMIC-MEMBERSHIP-MAP-V1",
        classes,
        |out, class| {
            out.digest("class_key_sha256", class.class_key_sha256);
            out.digest("member_set_sha256", class.member_set_sha256);
        },
    )
}

pub(super) fn digest_representative_map_v1(classes: &[DynamicClassSealV1]) -> Digest32 {
    digest_class_items(
        "ELON-A2-MAP-LOCK-DYNAMIC-REPRESENTATIVE-MAP-V1",
        classes,
        |out, class| {
            out.digest("class_key_sha256", class.class_key_sha256);
            out.member("representative", class.representative);
        },
    )
}

pub(super) fn digest_class_catalog_v1(classes: &[DynamicClassSealV1]) -> Digest32 {
    digest_class_items(
        "ELON-A2-MAP-LOCK-DYNAMIC-CLASS-CATALOG-V1",
        classes,
        |out, class| out.digest("class_record_sha256", class.class_record_sha256),
    )
}

pub(super) fn digest_reverse_index_v1(entries: &[ReverseIndexEntryV1]) -> Digest32 {
    let mut sorted = entries.to_vec();
    sorted.sort_unstable();
    let mut out = StableHasher::new("ELON-A2-MAP-LOCK-DYNAMIC-REVERSE-INDEX-V1");
    out.u64("entry_count", sorted.len() as u64);
    for entry in sorted {
        out.member("member", entry.member);
        out.digest("class_key_sha256", entry.class_key_sha256);
    }
    out.finish()
}

pub(super) fn digest_projector_schema_v1() -> Digest32 {
    let mut out = StableHasher::new("ELON-A2-MAP-LOCK-DYNAMIC-PROJECTOR-SCHEMA-V1");
    out.u16("schema_version", DYNAMIC_PROJECTOR_SCHEMA_V1);
    out.text("identity_erasure", "case-key-and-leaf-identity-excluded");
    out.text("unknown_policy", "fail-closed");
    out.finish()
}

pub(super) fn digest_dynamic_manifest_body_v1(manifest: &DynamicQuotientManifestV1) -> Digest32 {
    let mut out = StableHasher::new(MANIFEST_DOMAIN);
    let context = &manifest.context;
    out.u16("schema_version", context.schema_version);
    out.text("root", context.root.canonical_name());
    out.text(
        "static_source_baseline_sha1",
        &context.static_source_baseline_sha1,
    );
    out.digest(
        "static_source_scope_sha256",
        context.static_source_scope_sha256,
    );
    out.digest("static_ledger_sha256", context.static_ledger_sha256);
    out.digest("static_manifest_sha256", context.static_manifest_sha256);
    out.digest(
        "static_member_pair_set_sha256",
        context.static_member_pair_set_sha256,
    );
    out.u64("static_included_count", context.static_included_count);
    out.u64("static_excluded_count", context.static_excluded_count);
    out.u64(
        "static_source_universe_count",
        context.static_source_universe_count,
    );
    out.digest("projector_schema_sha256", context.projector_schema_sha256);
    out.digest(
        "projector_source_scope_sha256",
        context.projector_source_scope_sha256,
    );
    out.digest(
        "descriptor_binding_sha256",
        context.descriptor_binding_sha256,
    );
    out.digest(
        "runner_admission_binding_sha256",
        context.runner_admission_binding_sha256,
    );
    out.digest(
        "execution_program_inventory_sha256",
        context.execution_program_inventory_sha256,
    );
    out.digest(
        "execution_program_membership_sha256",
        context.execution_program_membership_sha256,
    );
    out.digest(
        "execution_program_catalog_sha256",
        context.execution_program_catalog_sha256,
    );
    out.digest(
        "program_catalog_admission_binding_sha256",
        context.program_catalog_admission_binding_sha256,
    );
    out.u64("class_count", manifest.class_count);
    out.u64("member_count", manifest.member_count);
    out.digest("class_key_set_sha256", manifest.class_key_set_sha256);
    out.digest("membership_map_sha256", manifest.membership_map_sha256);
    out.digest(
        "representative_map_sha256",
        manifest.representative_map_sha256,
    );
    out.digest("class_catalog_sha256", manifest.class_catalog_sha256);
    out.digest("reverse_index_sha256", manifest.reverse_index_sha256);
    out.finish()
}

fn digest_class_items(
    domain: &str,
    classes: &[DynamicClassSealV1],
    mut add: impl FnMut(&mut StableHasher, &DynamicClassSealV1),
) -> Digest32 {
    let mut sorted = classes.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|class| class.class_key_sha256);
    let mut out = StableHasher::new(domain);
    out.u64("class_count", sorted.len() as u64);
    for class in sorted {
        add(&mut out, class);
    }
    out.finish()
}

struct StableHasher(Sha256);

impl StableHasher {
    fn new(domain: &str) -> Self {
        let mut out = Sha256::new();
        out.update(domain.as_bytes());
        out.update([0]);
        Self(out)
    }

    fn text(&mut self, label: &str, value: &str) {
        self.bytes(label, value.as_bytes());
    }

    fn u16(&mut self, label: &str, value: u16) {
        self.bytes(label, &value.to_be_bytes());
    }

    fn u64(&mut self, label: &str, value: u64) {
        self.bytes(label, &value.to_be_bytes());
    }

    fn digest(&mut self, label: &str, value: Digest32) {
        self.bytes(label, &value.0);
    }

    fn member(&mut self, label: &str, value: StaticMemberSealV1) {
        let mut bytes = [0_u8; 64];
        bytes[..32].copy_from_slice(&value.case_key_sha256.0);
        bytes[32..].copy_from_slice(&value.full_record_sha256.0);
        self.bytes(label, &bytes);
    }

    fn bytes(&mut self, label: &str, value: &[u8]) {
        self.0.update(label.as_bytes());
        self.0.update([0]);
        self.0.update((value.len() as u64).to_be_bytes());
        self.0.update(value);
    }

    fn finish(self) -> Digest32 {
        Digest32(self.0.finalize().into())
    }
}
