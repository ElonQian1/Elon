use super::super::descriptor_binding::{
    authority_for_test, digest_descriptor_binding_v1, DescriptorBindingContextV1,
    DescriptorBindingEntryV1,
};
use super::*;

fn member(case: u8, full: u8) -> StaticMemberSealV1 {
    StaticMemberSealV1 {
        case_key_sha256: Digest32([case; 32]),
        full_record_sha256: Digest32([full; 32]),
    }
}

fn context(root: RootOperationV1) -> DescriptorBindingContextV1 {
    DescriptorBindingContextV1 {
        root,
        projector_schema_version: DYNAMIC_PROJECTOR_SCHEMA_V1,
        static_manifest_sha256: Digest32([7; 32]),
        included_count: 2,
    }
}

#[test]
fn descriptor_binding_rejects_swap_merge_and_authority_context_drift() {
    let first = DescriptorBindingEntryV1 {
        member: member(1, 2),
        descriptor_semantic_sha256: Digest32([3; 32]),
    };
    let second = DescriptorBindingEntryV1 {
        member: member(4, 5),
        descriptor_semantic_sha256: Digest32([6; 32]),
    };
    let base_context = context(RootOperationV1::Map);
    let baseline = digest_descriptor_binding_v1(base_context, [first, second]);

    assert_ne!(
        baseline,
        digest_descriptor_binding_v1(
            base_context,
            [
                DescriptorBindingEntryV1 {
                    descriptor_semantic_sha256: second.descriptor_semantic_sha256,
                    ..first
                },
                DescriptorBindingEntryV1 {
                    descriptor_semantic_sha256: first.descriptor_semantic_sha256,
                    ..second
                },
            ],
        )
    );
    assert_ne!(
        baseline,
        digest_descriptor_binding_v1(
            base_context,
            [
                first,
                DescriptorBindingEntryV1 {
                    descriptor_semantic_sha256: first.descriptor_semantic_sha256,
                    ..second
                },
            ],
        )
    );

    for drifted in [
        DescriptorBindingContextV1 {
            root: RootOperationV1::Lock,
            ..base_context
        },
        DescriptorBindingContextV1 {
            projector_schema_version: DYNAMIC_PROJECTOR_SCHEMA_V1 + 1,
            ..base_context
        },
        DescriptorBindingContextV1 {
            static_manifest_sha256: Digest32([8; 32]),
            ..base_context
        },
        DescriptorBindingContextV1 {
            included_count: base_context.included_count + 1,
            ..base_context
        },
    ] {
        assert_ne!(
            baseline,
            digest_descriptor_binding_v1(drifted, [first, second])
        );
    }
}

#[test]
fn descriptor_semantics_normalize_supported_and_missing_capability() {
    let record = record(
        "descriptor-normalization",
        "descriptor-normalization-branch",
    );
    let validated = project_validated_dynamic_terminal_v1(
        &record,
        &descriptor(RunnerCapabilityV1::Missing(
            CapabilityGapV1::QuotientRunnerNotIntegrated,
        )),
    )
    .unwrap();
    let missing = validated.semantic_key;
    let mut supported = missing;
    supported.recipe.capability = RunnerCapabilityV1::Supported;

    assert_ne!(
        digest_dynamic_class_key_v1(&supported),
        digest_dynamic_class_key_v1(&missing)
    );
    assert_eq!(
        digest_normalized_descriptor_semantics_v1(&supported),
        digest_normalized_descriptor_semantics_v1(&missing)
    );
}

#[test]
fn frozen_descriptor_mismatch_precedes_missing_runner_capability() {
    let record = record("binding-priority", "binding-priority-branch");
    let descriptor = descriptor(RunnerCapabilityV1::Missing(
        CapabilityGapV1::QuotientRunnerNotIntegrated,
    ));
    let seal = seal(&record);
    let validated = project_validated_dynamic_terminal_v1(&record, &descriptor).unwrap();
    assert!(validated.projection.is_err());
    let binding_context = DescriptorBindingContextV1 {
        included_count: 1,
        ..context(RootOperationV1::Map)
    };

    let mut mismatch = DynamicCatalogBuilderV1::new(RootOperationV1::Map);
    mismatch.freeze_descriptor_binding_for_test(authority_for_test(binding_context, []));
    mismatch
        .observe(StreamedLeafV1::Terminal {
            record: &record,
            descriptor: &descriptor,
            seal: &seal,
        })
        .unwrap();
    assert!(matches!(
        mismatch.finish(),
        Err(CatalogErrorV1::DescriptorBindingCommitmentDrift { .. })
    ));

    let mut exact = DynamicCatalogBuilderV1::new(RootOperationV1::Map);
    exact.freeze_descriptor_binding_for_test(authority_for_test(
        binding_context,
        [validated.descriptor_binding],
    ));
    exact
        .observe(StreamedLeafV1::Terminal {
            record: &record,
            descriptor: &descriptor,
            seal: &seal,
        })
        .unwrap();
    assert!(matches!(
        exact.finish(),
        Err(CatalogErrorV1::RunnerCapabilityMissing {
            count: 1,
            gap: CapabilityGapV1::QuotientRunnerNotIntegrated,
            ..
        })
    ));
}
