use super::super::manifest_canonical::digest_dynamic_manifest_body_v1;
use super::*;

fn catalog_and_binding() -> (DynamicCatalogV1, FrozenStaticBindingV1) {
    let left = record("manifest-left", "left-branch");
    let right = record("manifest-right", "right-branch");
    let descriptor = descriptor(RunnerCapabilityV1::Missing(
        CapabilityGapV1::QuotientRunnerNotIntegrated,
    ));
    let mut builder = DynamicCatalogBuilderV1::new(RootOperationV1::Map);
    for record in [&left, &right] {
        observe_synthetic_for_test(&mut builder, record, &descriptor);
    }
    let catalog = builder.finish().unwrap();
    let binding = FrozenStaticBindingV1 {
        context: ManifestContextV1 {
            schema: "static-v1".to_owned(),
            root: RootOperationV1::Map,
            target_scope: "windows-x64".to_owned(),
            source_baseline_commit_sha1: "a".repeat(40),
            source_scope_sha256: Digest32([1; 32]),
            ledger_sha256: Digest32([2; 32]),
            map_profile_set_sha256: Some(Digest32([3; 32])),
            map_ordinal_domain_sha256: Some(Digest32([4; 32])),
            lock_range_set_sha256: None,
            lock_range_count: None,
        },
        included_count: 2,
        excluded_count: 0,
        source_universe_count: 2,
        static_manifest_sha256: Digest32([5; 32]),
        included_member_pair_set_sha256: catalog.member_pair_set_sha256(),
    };
    (catalog, binding)
}

fn two_class_catalog_and_binding() -> (DynamicCatalogV1, FrozenStaticBindingV1) {
    let left = record("membership-left", "left-branch");
    let right = record("membership-right", "right-branch");
    let left_descriptor = descriptor(RunnerCapabilityV1::Missing(
        CapabilityGapV1::QuotientRunnerNotIntegrated,
    ));
    let mut right_descriptor = left_descriptor;
    let TerminalDescriptorV1::Map(value) = &mut right_descriptor else {
        unreachable!()
    };
    let StimulusV1::MapAbi(scalar) = &mut value.stimulus else {
        unreachable!()
    };
    scalar.output = PresenceV1::Absent;
    let mut builder = DynamicCatalogBuilderV1::new(RootOperationV1::Map);
    observe_synthetic_for_test(&mut builder, &left, &left_descriptor);
    observe_synthetic_for_test(&mut builder, &right, &right_descriptor);
    let catalog = builder.finish().unwrap();
    assert_eq!(catalog.classes().len(), 2);
    let binding = FrozenStaticBindingV1 {
        context: ManifestContextV1 {
            schema: "static-v1".to_owned(),
            root: RootOperationV1::Map,
            target_scope: "windows-x64".to_owned(),
            source_baseline_commit_sha1: "a".repeat(40),
            source_scope_sha256: Digest32([1; 32]),
            ledger_sha256: Digest32([2; 32]),
            map_profile_set_sha256: Some(Digest32([3; 32])),
            map_ordinal_domain_sha256: Some(Digest32([4; 32])),
            lock_range_set_sha256: None,
            lock_range_count: None,
        },
        included_count: 2,
        excluded_count: 0,
        source_universe_count: 2,
        static_manifest_sha256: Digest32([5; 32]),
        included_member_pair_set_sha256: catalog.member_pair_set_sha256(),
    };
    (catalog, binding)
}

#[test]
fn manifest_recomputes_class_key_and_exact_member_union() {
    let (mut catalog, binding) = catalog_and_binding();
    catalog.tamper_first_class_key_for_test(Digest32([91; 32]));
    assert_eq!(
        build_dynamic_manifest_v1(&binding, &catalog),
        Err(ManifestBuildErrorV1::ClassKeyMismatch)
    );

    let (mut catalog, binding) = catalog_and_binding();
    catalog.tamper_first_member_full_record_for_test(Digest32([92; 32]));
    assert_eq!(
        build_dynamic_manifest_v1(&binding, &catalog),
        Err(ManifestBuildErrorV1::StaticMemberSetMismatch)
    );
}

#[test]
fn manifest_rejects_noncanonical_member_order_and_binding_drift() {
    let (mut catalog, binding) = catalog_and_binding();
    catalog.reverse_first_class_members_for_test();
    assert_eq!(
        build_dynamic_manifest_v1(&binding, &catalog),
        Err(ManifestBuildErrorV1::MemberOrderMismatch)
    );

    let (catalog, mut binding) = catalog_and_binding();
    binding.included_member_pair_set_sha256 = Digest32([93; 32]);
    assert_eq!(
        build_dynamic_manifest_v1(&binding, &catalog),
        Err(ManifestBuildErrorV1::StaticMemberSetMismatch)
    );
}

#[test]
fn manifest_rejects_member_to_class_swap_despite_an_unchanged_union() {
    let (mut catalog, binding) = two_class_catalog_and_binding();
    catalog.swap_first_members_for_test();
    assert_eq!(
        build_dynamic_manifest_v1(&binding, &catalog),
        Err(ManifestBuildErrorV1::ProjectedMembershipMismatch)
    );
}

#[test]
fn manifest_rejects_class_merge_despite_an_unchanged_union() {
    let (mut catalog, binding) = two_class_catalog_and_binding();
    catalog.merge_second_class_into_first_for_test();
    assert_eq!(
        build_dynamic_manifest_v1(&binding, &catalog),
        Err(ManifestBuildErrorV1::ProjectedMembershipMismatch)
    );
}

#[test]
fn synthetic_manifest_zeros_and_binds_program_catalog_context() {
    let (catalog, binding) = catalog_and_binding();
    let bundle = build_dynamic_manifest_v1(&binding, &catalog).unwrap();
    let context = &bundle.manifest.context;
    assert_eq!(
        [
            context.execution_program_inventory_sha256,
            context.execution_program_membership_sha256,
            context.execution_program_catalog_sha256,
            context.program_catalog_admission_binding_sha256,
        ],
        [Digest32::ZERO; 4],
    );

    let baseline = digest_dynamic_manifest_body_v1(&bundle.manifest);
    assert_eq!(baseline, bundle.manifest.manifest_sha256);

    let mut inventory = bundle.manifest.clone();
    inventory.context.execution_program_inventory_sha256 = Digest32([6; 32]);
    let mut membership = bundle.manifest.clone();
    membership.context.execution_program_membership_sha256 = Digest32([7; 32]);
    let mut catalog = bundle.manifest.clone();
    catalog.context.execution_program_catalog_sha256 = Digest32([8; 32]);
    let mut admission = bundle.manifest.clone();
    admission.context.program_catalog_admission_binding_sha256 = Digest32([9; 32]);

    for tampered in [&inventory, &membership, &catalog, &admission] {
        assert_ne!(digest_dynamic_manifest_body_v1(tampered), baseline);
    }
}

#[test]
fn projector_provenance_binds_typed_descriptor_producers_and_wiring() {
    use super::super::manifest_canonical::{
        digest_projector_source_entries_v1, digest_projector_source_scope_v1,
        projector_source_scope_entries_v1,
    };

    let required_sources = [
        "dynamic_quotient.rs",
        "dynamic_quotient/manifest_canonical.rs",
        "dynamic_quotient/manifest_canonical/projector_source_scope.rs",
        "terminal_descriptor.rs",
        "terminal_descriptor/axes.rs",
        "terminal_descriptor/recipe.rs",
        "map/dynamic.rs",
        "lock/dynamic.rs",
        "dynamic_quotient/producer_coherence.rs",
        "dynamic_quotient/producer_coherence/map.rs",
        "dynamic_quotient/producer_coherence/map_axes.rs",
        "dynamic_quotient/producer_coherence/lock.rs",
        "dynamic_quotient/producer_coherence/lock_axes.rs",
        "dynamic_quotient/projector/lock_execution.rs",
        "dynamic_quotient/map_runtime_source_scope.rs",
        "dynamic_quotient/lock_local_sibling_contention_source_scope.rs",
        "dynamic_quotient/lock_callback_completion_route_unknown_source_scope.rs",
        "dynamic_quotient/lock_local_protocol_rejection_source_scope.rs",
        "dynamic_quotient/lock_native_acquire_busy_source_scope.rs",
        "dynamic_quotient/lock_stored_poison_source_scope.rs",
        "dynamic_quotient/membership_commitment.rs",
        "dynamic_quotient/descriptor_binding.rs",
        "dynamic_quotient/runner_admission.rs",
        "dynamic_quotient/runner_admission/canonical.rs",
        "dynamic_quotient/runner_admission/map.rs",
        "dynamic_quotient/runner_admission/map_program.rs",
        "dynamic_quotient/runner_admission/map_program/request_budget.rs",
        "dynamic_quotient/runner_admission/map_program/lifecycle.rs",
        "dynamic_quotient/runner_admission/map_program/lifecycle/source_scope.rs",
        "dynamic_quotient/runner_admission/map_program/region_loop.rs",
        "dynamic_quotient/runner_admission/map_program/region_loop/catalog.rs",
        "dynamic_quotient/runner_admission/map_program/region_loop/region_loop_members.v1.tsv",
        "dynamic_quotient/runner_admission/map_program/region_loop/source_scope.rs",
        "dynamic_quotient/runner_admission/lock.rs",
        "dynamic_quotient/runner_admission/lock_program.rs",
        "dynamic_quotient/runner_admission/lock_program/execution_receipt.rs",
        "dynamic_quotient/runner_admission/lock_program/request_validation.rs",
        "dynamic_quotient/runner_admission/lock_program/lifecycle.rs",
        "dynamic_quotient/runner_admission/lock_program/source_program.rs",
        "dynamic_quotient/runner_admission/lock_program/local_sibling_contention.rs",
        "dynamic_quotient/runner_admission/lock_program/local_sibling_contention/catalog.rs",
        "dynamic_quotient/runner_admission/lock_program/local_sibling_contention/source_scope.rs",
        "dynamic_quotient/runner_admission/lock_program/local_sibling_contention/local_sibling_contention_completed_members.v1.tsv",
        "dynamic_quotient/runner_admission/lock_program/callback_completion_route_unknown.rs",
        "dynamic_quotient/runner_admission/lock_program/callback_completion_route_unknown/catalog.rs",
        "dynamic_quotient/runner_admission/lock_program/callback_completion_route_unknown/runtime.rs",
        "dynamic_quotient/runner_admission/lock_program/callback_completion_route_unknown/source_scope.rs",
        "dynamic_quotient/runner_admission/lock_program/callback_completion_route_unknown/callback_completion_route_unknown_members.v1.tsv",
        "dynamic_quotient/runner_admission/lock_program/local_protocol_rejection.rs",
        "dynamic_quotient/runner_admission/lock_program/local_protocol_rejection/catalog.rs",
        "dynamic_quotient/runner_admission/lock_program/local_protocol_rejection/runtime.rs",
        "dynamic_quotient/runner_admission/lock_program/local_protocol_rejection/source_scope.rs",
        "dynamic_quotient/runner_admission/lock_program/local_protocol_rejection/local_protocol_own_overlap_or_not_held_completed_members.v1.tsv",
        "dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection.rs",
        "dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/catalog.rs",
        "dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/runtime.rs",
        "dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/source_scope.rs",
        "dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/admission_route_unknown_direct_members.v1.tsv",
        "dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/admission_counter_overflow_direct_members.v1.tsv",
        "dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/unsupported_file_role_completed_members.v1.tsv",
        "dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/unsupported_file_role_route_unknown_members.v1.tsv",
        "dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/shm_detached_completed_members.v1.tsv",
        "dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/shm_detached_route_unknown_members.v1.tsv",
        "dynamic_quotient/runner_admission/lock_program/abi_scalar_rejection.rs",
        "dynamic_quotient/runner_admission/lock_program/abi_scalar_rejection/catalog.rs",
        "dynamic_quotient/runner_admission/lock_program/abi_scalar_rejection/runtime.rs",
        "dynamic_quotient/runner_admission/lock_program/abi_scalar_rejection/source_scope.rs",
        "dynamic_quotient/runner_admission/lock_program/abi_scalar_rejection/abi_scalar_rejection_members.v1.tsv",
        "dynamic_quotient/runner_admission/lock_program/raw_state_rejection.rs",
        "dynamic_quotient/runner_admission/lock_program/raw_state_rejection/case.rs",
        "dynamic_quotient/runner_admission/lock_program/raw_state_rejection/catalog.rs",
        "dynamic_quotient/runner_admission/lock_program/raw_state_rejection/expected.rs",
        "dynamic_quotient/runner_admission/lock_program/raw_state_rejection/runtime.rs",
        "dynamic_quotient/runner_admission/lock_program/raw_state_rejection/source_scope.rs",
        "dynamic_quotient/runner_admission/lock_program/raw_state_rejection/raw_state_rejection_members.v1.tsv",
        "dynamic_quotient/runner_admission/lock_program/native_acquire_created_first_exclusive_release_error.rs",
        "dynamic_quotient/runner_admission/lock_program/native_acquire_created_first_exclusive_release_error/catalog.rs",
        "dynamic_quotient/runner_admission/lock_program/native_acquire_created_first_exclusive_release_error/runtime.rs",
        "dynamic_quotient/runner_admission/lock_program/native_acquire_created_first_exclusive_release_error/source_scope.rs",
        "dynamic_quotient/runner_admission/lock_program/native_acquire_created_first_exclusive_release_error/native_acquire_created_first_exclusive_release_error_members.v1.tsv",
        "dynamic_quotient/runner_admission/lock_program/native_acquire_existing_first_exclusive_release_error.rs",
        "dynamic_quotient/runner_admission/lock_program/native_acquire_existing_first_exclusive_release_error/catalog.rs",
        "dynamic_quotient/runner_admission/lock_program/native_acquire_existing_first_exclusive_release_error/runtime.rs",
        "dynamic_quotient/runner_admission/lock_program/native_acquire_existing_first_exclusive_release_error/source_scope.rs",
        "dynamic_quotient/runner_admission/lock_program/native_acquire_existing_first_exclusive_release_error/native_acquire_existing_first_exclusive_release_error_members.v1.tsv",
        "registry/state/test_lock_callback_admission.rs",
        "registry/owner/test_lock_callback_admission.rs",
        "registry/process_owner/test_lock_callback_admission.rs",
        "registry/test_vfs_bridge/lock_callback_admission.rs",
        "registry/file_custody/operations/shm.rs",
        "registry/file_custody/operations/pre_managed_lock.rs",
        "managed_vfs/lifecycle_faults/pre_managed_lock.rs",
        "managed_vfs/lifecycle_faults/pre_managed_lock/abi_rejected.rs",
        "managed_vfs/connection/lock_pre_managed.rs",
        "managed_vfs/a2_dynamic_evidence/child/lock_pre_managed_rejection.rs",
        "managed_vfs/a2_dynamic_evidence/child/lock_abi_scalar_rejection.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/pre_managed_rejection.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/pre_managed_rejection/fixture.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/pre_managed_rejection/payload.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/abi_scalar_rejection.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/abi_scalar_rejection/payload.rs",
        "managed_vfs/a2_dynamic_evidence/child/lock_raw_state_rejection.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/raw_state_rejection.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/raw_state_rejection/fixture.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/raw_state_rejection/payload.rs",
        "managed_vfs/a2_dynamic_evidence/child/lock_created_first_exclusive_release_error.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/created_first_exclusive_release_error.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/created_first_exclusive_release_error/fixture.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/created_first_exclusive_release_error/payload.rs",
        "managed_vfs/a2_dynamic_evidence/child/lock_existing_first_exclusive_release_error.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/existing_first_exclusive_release_error.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/existing_first_exclusive_release_error/fixture.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/existing_first_exclusive_release_error/payload.rs",
        "managed_vfs/connection/lock_raw.rs",
        "managed_vfs/connection/lock_initialization.rs",
        "managed_vfs/lifecycle_faults/pre_managed_lock/raw_rejected.rs",
        "sqlite_vfs_abi/lock_observation.rs",
        "sqlite_vfs_abi/raw_lock_observation.rs",
        "sqlite_vfs_abi/raw_lock_observation/events.rs",
        "sqlite_vfs_abi/raw_lock_observation/expected.rs",
        "sqlite_vfs_abi/raw_lock_observation/model.rs",
        "sqlite_vfs_abi/raw_state/lock_raw_control.rs",
        "dynamic_quotient/runner_admission/lock_program/native_acquire_busy.rs",
        "dynamic_quotient/runner_admission/lock_program/native_acquire_busy/catalog.rs",
        "dynamic_quotient/runner_admission/lock_program/native_acquire_busy/source_scope.rs",
        "dynamic_quotient/runner_admission/lock_program/native_acquire_busy/native_acquire_node_live_native_busy_completed_members.v1.tsv",
        "dynamic_quotient/runner_admission/lock_program/stored_poison.rs",
        "dynamic_quotient/runner_admission/lock_program/stored_poison/catalog.rs",
        "dynamic_quotient/runner_admission/lock_program/stored_poison/source_scope.rs",
        "dynamic_quotient/runner_admission/lock_program/stored_poison/stored_poison_retention_succeeded_members.v1.tsv",
        "dynamic_quotient/runner_admission/lock_program/stored_poison/stored_poison_retention_route_unknown_members.v1.tsv",
        "managed_vfs/a2_dynamic_evidence.rs",
        "managed_vfs/a2_dynamic_evidence/child.rs",
        "managed_vfs/a2_dynamic_evidence/child/payload.rs",
        "managed_vfs/a2_dynamic_evidence/child/lock_request_validation.rs",
        "managed_vfs/a2_dynamic_evidence/child/lock_lifecycle.rs",
        "managed_vfs/a2_dynamic_evidence/child/lock_local_sibling_contention.rs",
        "managed_vfs/a2_dynamic_evidence/child/lock_callback_route_unknown.rs",
        "managed_vfs/a2_dynamic_evidence/child/lock_local_protocol_rejection.rs",
        "managed_vfs/a2_dynamic_evidence/child/test_support.rs",
        "managed_vfs/a2_dynamic_evidence/child/lock_native_acquire_busy.rs",
        "managed_vfs/a2_dynamic_evidence/child/lock_stored_poison.rs",
        "managed_vfs/a2_dynamic_evidence/child/lock_stored_poison/route_unknown.rs",
        "managed_vfs/a2_dynamic_evidence/child/map_lifecycle.rs",
        "managed_vfs/a2_dynamic_evidence/child/map_region_loop.rs",
        "managed_vfs/a2_dynamic_evidence/capture.rs",
        "managed_vfs/a2_dynamic_evidence/environment.rs",
        "managed_vfs/a2_dynamic_evidence/cleanup.rs",
        "managed_vfs/a2_dynamic_evidence/map_runner.rs",
        "managed_vfs/a2_dynamic_evidence/map_runner/request_budget.rs",
        "managed_vfs/a2_dynamic_evidence/map_runner/lifecycle.rs",
        "managed_vfs/a2_dynamic_evidence/map_runner/lifecycle/fixture.rs",
        "managed_vfs/a2_dynamic_evidence/map_runner/lifecycle/payload.rs",
        "managed_vfs/a2_dynamic_evidence/map_runner/region_loop.rs",
        "managed_vfs/a2_dynamic_evidence/map_runner/region_loop/fixture.rs",
        "managed_vfs/a2_dynamic_evidence/map_runner/region_loop/payload.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/request_validation.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/lifecycle.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/lifecycle/fixture.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/lifecycle/payload.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/local_sibling_contention.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/local_sibling_contention/fixture.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/local_sibling_contention/payload.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/callback_completion_route_unknown.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/callback_completion_route_unknown/fixture.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/callback_completion_route_unknown/fixture/validation.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/callback_completion_route_unknown/payload.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/local_protocol_rejection.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/local_protocol_rejection/fixture.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/local_protocol_rejection/payload.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/selector_test_support.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/native_acquire_busy.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/native_acquire_busy/fixture.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/native_acquire_busy/payload.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/stored_poison.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/stored_poison_dispatch.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/stored_poison_model.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/stored_poison_route_unknown.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/stored_poison_route_unknown/payload.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/stored_poison/fixture.rs",
        "managed_vfs/a2_dynamic_evidence/lock_runner/stored_poison/payload.rs",
        "managed_vfs.rs",
        "managed_vfs/connection.rs",
        "managed_vfs/live_registration.rs",
        "managed_vfs/shared_namespace.rs",
        "managed_vfs/shm_fault_script.rs",
        "managed_vfs/connection/unmap.rs",
        "managed_vfs/callbacks.rs",
        "managed_vfs/route_file.rs",
        "managed_vfs/fault_script.rs",
        "managed_vfs/fault_script/file.rs",
        "managed_vfs/multi_connection.rs",
        "managed_vfs/connection/registry_lifecycle.rs",
        "managed_vfs/shared_namespace/registration_shutdown.rs",
        "managed_vfs/shared_namespace/registry_lifecycle.rs",
        "managed_vfs/lifecycle_faults.rs",
        "managed_vfs/lifecycle_faults/native_gate.rs",
        "managed_vfs/lifecycle_faults/registry_lifecycle.rs",
        "managed_vfs/lifecycle_faults/registry_lifecycle/binding.rs",
        "managed_vfs/lifecycle_faults/ordinary_shm_lock_preemption.rs",
        "managed_vfs/lifecycle_faults/unmap.rs",
        "managed_vfs/lifecycle_faults/unsafe_shm_preemption.rs",
        "managed_vfs/lifecycle_faults/joint_close.rs",
        "managed_vfs/lifecycle_faults/registration_shutdown.rs",
        "managed_vfs/registration_shutdown_custody.rs",
        "registry.rs",
        "registry/test_vfs_bridge.rs",
        "registry/test_vfs_bridge/file.rs",
        "registry/file_custody.rs",
        "registry/file_custody/abi.rs",
        "registry/file_custody/joint_close_runtime.rs",
        "registry/file_custody/lifecycle_events.rs",
        "registry/file_custody/promotion.rs",
        "registry/file_custody/registry_lifecycle.rs",
        "registry/file_custody/test_faults.rs",
        "registry/file_custody/operations.rs",
        "registry/file_custody/operations/ordinary_shm_lock_preemption.rs",
        "registry/file_custody/operations/unmap.rs",
        "registry/owner.rs",
        "registry/owner/lifecycle.rs",
        "registry/owner/vfs.rs",
        "registry/process_owner.rs",
        "registry/process_owner/joint_close_direct_xclose.rs",
        "registry/process_owner/joint_close_fault.rs",
        "registry/process_owner/lifecycle.rs",
        "registry/process_owner/vfs.rs",
        "registry/state.rs",
        "registry/state/owner.rs",
        "registry/state/test_lifecycle.rs",
        "registry/state/test_snapshot.rs",
        "registry/types.rs",
        "sqlite_vfs_abi.rs",
        "sqlite_vfs_abi/boundary.rs",
        "sqlite_vfs_abi/io_shm.rs",
        "sqlite_vfs_abi/result_codes.rs",
        "sqlite_vfs_abi/file_state.rs",
        "sqlite_vfs_abi/raw_state.rs",
        "sqlite_vfs_abi/types.rs",
        "node_agent_managed_fs.rs",
        "node_agent_managed_fs/windows.rs",
        "node_agent_managed_fs/windows_sqlite.rs",
        "node_agent_managed_fs/windows_sqlite_locking.rs",
        "node_agent_managed_fs/windows_sqlite_shm.rs",
        "node_agent_managed_fs/sqlite_api.rs",
        "node_agent_managed_fs/sqlite_namespace_io.rs",
        "node_agent_managed_fs/sqlite_namespace.rs",
        "node_agent_managed_fs/sqlite_namespace_close.rs",
        "node_agent_managed_fs/sqlite_namespace_close/main_close.rs",
        "node_agent_managed_fs/sqlite_namespace_close/main_close_test_native.rs",
        "node_agent_managed_fs/sqlite_namespace_close/test_faults.rs",
        "node_agent_managed_fs/sqlite_namespace_main.rs",
        "node_agent_managed_fs/sqlite_namespace_types.rs",
        "node_agent_managed_fs/sqlite_namespace_validation.rs",
        "node_agent_managed_fs/sqlite_namespace_lock_domain.rs",
        "node_agent_managed_fs/sqlite_namespace_locking.rs",
        "node_agent_managed_fs/sqlite_namespace_locking/test_native.rs",
        "node_agent_managed_fs/sqlite_namespace_shm.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/barrier.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/close.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/coordinator.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/failure_custody.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/types.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/node_initialization.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/mapping.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/locking.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/unmap.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/teardown.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_faults/api.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_snapshot.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_faults.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_faults/controller.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_faults/operation.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_faults/mapping.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_faults/native_lock_contention.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_faults/stored_poison.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_lock_runtime.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_initialization_runtime.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_initialization_runtime/controller.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_initialization_runtime/model.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_initialization_runtime/tests.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_support.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_map_runtime.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_map_runtime/mapping_sequence.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_unmap_runtime.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_unmap_runtime/authority.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_unmap_runtime/detach.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_unmap_runtime/native.rs",
        "node_agent_managed_fs/sqlite_namespace_shm/test_unmap_runtime/prestate.rs",
    ];
    let source_scope = projector_source_scope_entries_v1().collect::<Vec<_>>();
    let source_names = source_scope
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>();
    let unique_names = source_names
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        unique_names.len(),
        source_names.len(),
        "projector provenance source names must be unique"
    );

    assert!(
        !source_names.contains(&"dynamic_quotient/program_inventory/review.rs"),
        "review digest must remain outside the source scope it freezes"
    );

    for required in required_sources {
        assert!(
            source_names.contains(&required),
            "projector provenance omitted {required}"
        );
    }

    let baseline = digest_projector_source_scope_v1();
    for required in required_sources {
        let mutated = source_scope
            .iter()
            .map(|(name, source)| {
                let source = if *name == required {
                    format!("{source}\n// provenance mutation")
                } else {
                    (*source).to_owned()
                };
                (*name, source)
            })
            .collect::<Vec<_>>();
        let mutated_digest = digest_projector_source_entries_v1(
            mutated
                .iter()
                .map(|(name, source)| (*name, source.as_str())),
        );
        assert_ne!(
            mutated_digest, baseline,
            "mutating {required} did not change projector provenance"
        );
    }
}
