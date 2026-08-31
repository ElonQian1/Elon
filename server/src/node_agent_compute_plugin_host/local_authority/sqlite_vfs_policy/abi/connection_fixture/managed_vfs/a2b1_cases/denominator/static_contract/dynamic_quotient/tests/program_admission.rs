use super::super::{
    program_inventory::{provider_for_source_program_for_test, ProgramCatalogAdmissionErrorV1},
    runner_admission::RunnerAdmissionDecisionV1,
};
use super::program_inventory::{budget_descriptor, request_budget_record};
use super::*;

fn source_present_descriptor(mode: MapModeV1) -> TerminalDescriptorV1 {
    budget_descriptor(
        MapManagedStimulusV1::RegionCountBudget,
        mode,
        RunnerCapabilityV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated),
    )
}

#[test]
fn source_program_provider_projects_without_granting_runner_supported() {
    let record = request_budget_record();
    let descriptor = source_present_descriptor(MapModeV1::Extend);
    let prepared = prepare_dynamic_terminal_v1(&record, &descriptor).unwrap();
    let mut provider =
        provider_for_source_program_for_test(prepared.member, &prepared.key).unwrap();

    let validated = project_validated_dynamic_terminal_with_program_catalog_v1(
        &record,
        &descriptor,
        &mut provider,
    )
    .unwrap();
    let projection = validated.projection.unwrap();
    assert_eq!(projection.member, prepared.member);
    assert_eq!(projection.key, prepared.key);
    assert_eq!(
        validated.runner_admission.decision(),
        RunnerAdmissionDecisionV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated),
    );

    assert!(provider.finish().is_ok());
}

#[test]
fn source_program_provider_rejects_duplicate_and_wrong_members() {
    let record = request_budget_record();
    let descriptor = source_present_descriptor(MapModeV1::Extend);
    let prepared = prepare_dynamic_terminal_v1(&record, &descriptor).unwrap();
    let mut provider =
        provider_for_source_program_for_test(prepared.member, &prepared.key).unwrap();

    project_validated_dynamic_terminal_with_program_catalog_v1(&record, &descriptor, &mut provider)
        .unwrap();
    assert_eq!(
        project_validated_dynamic_terminal_with_program_catalog_v1(
            &record,
            &descriptor,
            &mut provider,
        ),
        Err(ProjectionErrorV1::ProgramCatalogAdmission(
            ProgramCatalogAdmissionErrorV1::ReceiptAlreadyConsumed(prepared.member),
        )),
    );

    let mut wrong_record = record.clone();
    wrong_record.key.identity.leaf_id =
        "map-region-count-budget-program-admission-wrong-member".to_owned();
    let wrong_prepared = prepare_dynamic_terminal_v1(&wrong_record, &descriptor).unwrap();
    assert_ne!(wrong_prepared.member, prepared.member);
    let mut provider =
        provider_for_source_program_for_test(prepared.member, &prepared.key).unwrap();
    assert_eq!(
        project_validated_dynamic_terminal_with_program_catalog_v1(
            &wrong_record,
            &descriptor,
            &mut provider,
        ),
        Err(ProjectionErrorV1::ProgramCatalogAdmission(
            ProgramCatalogAdmissionErrorV1::ReceiptMissing(wrong_prepared.member),
        )),
    );
    assert_eq!(
        provider.finish(),
        Err(ProgramCatalogAdmissionErrorV1::UnconsumedReceipts(1)),
    );
}

#[test]
fn source_program_provider_rejects_semantic_drift_for_the_same_member() {
    let record = request_budget_record();
    let descriptor = source_present_descriptor(MapModeV1::Extend);
    let prepared = prepare_dynamic_terminal_v1(&record, &descriptor).unwrap();
    let mut provider =
        provider_for_source_program_for_test(prepared.member, &prepared.key).unwrap();
    let drifted_descriptor = source_present_descriptor(MapModeV1::Observe);
    let drifted = prepare_dynamic_terminal_v1(&record, &drifted_descriptor).unwrap();
    assert_eq!(drifted.member, prepared.member);
    assert_ne!(drifted.key, prepared.key);

    assert_eq!(
        project_validated_dynamic_terminal_with_program_catalog_v1(
            &record,
            &drifted_descriptor,
            &mut provider,
        ),
        Err(ProjectionErrorV1::ProgramCatalogAdmission(
            ProgramCatalogAdmissionErrorV1::ReceiptBindingMismatch(prepared.member),
        )),
    );
}
