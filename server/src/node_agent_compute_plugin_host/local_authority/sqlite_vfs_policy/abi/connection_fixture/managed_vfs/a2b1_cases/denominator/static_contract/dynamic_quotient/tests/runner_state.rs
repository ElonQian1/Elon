use super::*;

fn observe(
    builder: &mut DynamicCatalogBuilderV1,
    record: &LeafRecordV1,
    descriptor: &TerminalDescriptorV1,
) {
    let seal = seal(record);
    builder
        .observe(StreamedLeafV1::Terminal {
            record,
            descriptor,
            seal: &seal,
        })
        .unwrap();
}

#[test]
fn partial_supported_and_missing_runner_state_is_rejected() {
    let supported = record("supported", "supported-branch");
    let missing = record("missing", "missing-branch");
    let mut builder = DynamicCatalogBuilderV1::new(RootOperationV1::Map);
    observe_synthetic_for_test(
        &mut builder,
        &supported,
        &descriptor(RunnerCapabilityV1::Missing(
            CapabilityGapV1::QuotientRunnerNotIntegrated,
        )),
    );
    observe(
        &mut builder,
        &missing,
        &descriptor(RunnerCapabilityV1::Missing(
            CapabilityGapV1::QuotientRunnerNotIntegrated,
        )),
    );
    assert!(matches!(
        builder.finish(),
        Err(CatalogErrorV1::MixedRunnerCapabilityState {
            supported: 1,
            missing: 1,
            ..
        })
    ));
}

#[test]
fn semantic_failure_count_excludes_runner_capability_failures() {
    let mut semantic = record("semantic", "semantic-branch");
    let semantic_seal = seal(&semantic);
    let LeafOutcomeV1::Terminal(expected) = &mut semantic.outcome else {
        unreachable!()
    };
    expected.phase = "Success".to_owned();
    let missing = record("missing-too", "missing-too-branch");
    let missing_descriptor = descriptor(RunnerCapabilityV1::Missing(
        CapabilityGapV1::QuotientRunnerNotIntegrated,
    ));
    let mut builder = DynamicCatalogBuilderV1::new(RootOperationV1::Map);
    builder
        .observe(StreamedLeafV1::Terminal {
            record: &semantic,
            descriptor: &missing_descriptor,
            seal: &semantic_seal,
        })
        .unwrap();
    observe(&mut builder, &missing, &missing_descriptor);
    assert!(matches!(
        builder.finish(),
        Err(CatalogErrorV1::ProjectionFailed {
            count: 1,
            first: ProjectionFailureV1 {
                error: ProjectionErrorV1::StaticPhaseMismatch { .. },
                ..
            },
        })
    ));
}
