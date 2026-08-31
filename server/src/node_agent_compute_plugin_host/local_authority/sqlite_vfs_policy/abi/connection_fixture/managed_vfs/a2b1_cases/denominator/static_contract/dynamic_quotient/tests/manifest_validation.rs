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
        PROJECTOR_SOURCE_SCOPE_V1,
    };

    let required_sources = [
        "dynamic_quotient.rs",
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
        "dynamic_quotient/membership_commitment.rs",
        "dynamic_quotient/descriptor_binding.rs",
        "dynamic_quotient/runner_admission.rs",
        "dynamic_quotient/runner_admission/canonical.rs",
        "dynamic_quotient/runner_admission/map.rs",
        "dynamic_quotient/runner_admission/map_program.rs",
        "dynamic_quotient/runner_admission/map_program/request_budget.rs",
        "dynamic_quotient/runner_admission/lock.rs",
        "managed_vfs/a2_dynamic_evidence/child.rs",
        "managed_vfs/a2_dynamic_evidence/child/payload.rs",
        "managed_vfs/a2_dynamic_evidence/map_runner.rs",
        "managed_vfs/connection/unmap.rs",
    ];
    let source_names = PROJECTOR_SOURCE_SCOPE_V1
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>();

    for required in required_sources {
        assert!(
            source_names.contains(&required),
            "projector provenance omitted {required}"
        );
    }

    let baseline = digest_projector_source_scope_v1();
    for required in required_sources {
        let mutated = PROJECTOR_SOURCE_SCOPE_V1
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
