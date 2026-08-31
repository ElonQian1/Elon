//! Source-level admission tests for the two bounded Map region-loop success families.

use super::super::runner_admission::{
    compile_for_test, region_loop_catalog_row_count_for_test, validate_map_program_for_test,
};
#[cfg(windows)]
use super::super::runner_admission::{
    run_isolated_for_test, MapRunnerIsolatedOutcomeV1, RunnerAdmissionDecisionV1,
};
use super::map_program_cases::{
    frozen_map_region_loop_leaves_v1, map_region_loop_descriptor_v1, map_region_loop_leaf_v1,
    MapRegionLoopFamilyV1, MAP_REGION_LOOP_FAMILIES, MAP_REGION_LOOP_MEMBER_COUNT,
};
use super::*;

#[cfg(windows)]
const EXACT_TEST_PREFIX: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b1_cases::denominator::static_contract::dynamic_quotient::tests::runner_admission_map_region_loop_supported::";

fn supported_key_and_member(
    family: MapRegionLoopFamilyV1,
    ordinal: u16,
) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let leaf = map_region_loop_leaf_v1(family, ordinal);
    let descriptor = map_region_loop_descriptor_v1(
        family,
        ordinal,
        RunnerCapabilityV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated),
    );
    let validated = project_validated_dynamic_terminal_v1(&leaf.record, &descriptor).unwrap();
    assert_eq!(validated.descriptor_binding.member, leaf.member);
    let mut key = validated.semantic_key;
    key.recipe.capability = RunnerCapabilityV1::Supported;
    (key, leaf.member)
}

fn assert_rejected(key: DynamicClassKeyV1, member: StaticMemberSealV1, mutation: &str) {
    assert!(
        validate_map_program_for_test(&key, member, compile_for_test(&key)).is_err(),
        "Map region-loop admission accepted {mutation}"
    );
}

fn set_consistent_region_count(key: &mut DynamicClassKeyV1, regions_to_create: u16) {
    key.occurrence = OccurrenceV1::Exact(regions_to_create);
    let DynamicAxesV1::Map(axes) = &mut key.axes else {
        unreachable!()
    };
    axes.ordinal = ReachabilityV1::Reached(regions_to_create);
    axes.regions_to_create = ReachabilityV1::Reached(regions_to_create);
    key.expected.counts.mapping_create = regions_to_create;
    key.expected.counts.view_map = regions_to_create;
}

#[test]
fn all_511_exact_region_loop_descriptors_and_members_are_admitted() {
    let leaves = frozen_map_region_loop_leaves_v1();
    assert_eq!(leaves.len(), MAP_REGION_LOOP_MEMBER_COUNT);
    assert_eq!(
        region_loop_catalog_row_count_for_test(),
        MAP_REGION_LOOP_MEMBER_COUNT
    );
    for (&(family, ordinal), leaf) in leaves {
        let (key, member) = supported_key_and_member(family, ordinal);
        assert_eq!(member, leaf.member);
        validate_map_program_for_test(&key, member, compile_for_test(&key)).unwrap_or_else(
            |error| {
                panic!("exact Map region-loop member {family:?}/{ordinal} was rejected: {error:?}")
            },
        );
    }
}

#[test]
fn every_region_loop_program_rejects_a_sibling_member_seal() {
    for family in MAP_REGION_LOOP_FAMILIES {
        for ordinal in 1..=family.max_ordinal() {
            let sibling = if ordinal == family.max_ordinal() {
                ordinal - 1
            } else {
                ordinal + 1
            };
            let (key, _) = supported_key_and_member(family, ordinal);
            assert_rejected(
                key,
                map_region_loop_leaf_v1(family, sibling).member,
                "a sibling member seal",
            );
        }
    }
}

#[test]
fn region_loop_programs_reject_each_near_neighbor_semantic_mutation() {
    for family in MAP_REGION_LOOP_FAMILIES {
        let ordinal = 2;
        let (key, member) = supported_key_and_member(family, ordinal);

        let mut profile_drift = key;
        let DynamicAxesV1::Map(axes) = &mut profile_drift.axes else {
            unreachable!()
        };
        let ReachabilityV1::Reached(profile) = &mut axes.profile else {
            unreachable!()
        };
        profile.prior_mutation = false;
        assert_rejected(profile_drift, member, "profile drift");

        let sibling_family = match family {
            MapRegionLoopFamilyV1::CreatedFirstEmptyExtend => {
                MapRegionLoopFamilyV1::NodeLiveTargetMissingExtend
            }
            MapRegionLoopFamilyV1::NodeLiveTargetMissingExtend => {
                MapRegionLoopFamilyV1::CreatedFirstEmptyExtend
            }
        };
        let (family_swapped, _) = supported_key_and_member(sibling_family, ordinal);
        assert_rejected(family_swapped, member, "family/profile swap");

        let mut occurrence_drift = key;
        occurrence_drift.occurrence = OccurrenceV1::Exact(ordinal + 1);
        assert_rejected(occurrence_drift, member, "occurrence drift");

        let mut ordinal_drift = key;
        let DynamicAxesV1::Map(axes) = &mut ordinal_drift.axes else {
            unreachable!()
        };
        axes.ordinal = ReachabilityV1::Reached(ordinal + 1);
        assert_rejected(ordinal_drift, member, "ordinal drift");

        let mut regions_drift = key;
        let DynamicAxesV1::Map(axes) = &mut regions_drift.axes else {
            unreachable!()
        };
        axes.regions_to_create = ReachabilityV1::Reached(ordinal + 1);
        assert_rejected(regions_drift, member, "regions-to-create drift");

        let mut mapping_count_drift = key;
        mapping_count_drift.expected.counts.mapping_create += 1;
        assert_rejected(mapping_count_drift, member, "mapping-create count drift");

        let mut view_count_drift = key;
        view_count_drift.expected.counts.view_map += 1;
        assert_rejected(view_count_drift, member, "view-map count drift");
    }
}

#[test]
fn region_loop_programs_reject_zero_and_first_out_of_range_ordinals() {
    for family in MAP_REGION_LOOP_FAMILIES {
        let (mut zero, zero_member) = supported_key_and_member(family, 1);
        set_consistent_region_count(&mut zero, 0);
        assert_rejected(zero, zero_member, "ordinal zero");
    }

    let (mut empty_257, empty_member) =
        supported_key_and_member(MapRegionLoopFamilyV1::CreatedFirstEmptyExtend, 256);
    set_consistent_region_count(&mut empty_257, 257);
    assert_rejected(empty_257, empty_member, "created-first ordinal 257");

    let (mut missing_256, missing_member) =
        supported_key_and_member(MapRegionLoopFamilyV1::NodeLiveTargetMissingExtend, 255);
    set_consistent_region_count(&mut missing_256, 256);
    assert_rejected(
        missing_256,
        missing_member,
        "node-live target-missing ordinal 256",
    );
}

#[cfg(windows)]
fn exercise_isolated_family(family: MapRegionLoopFamilyV1, exact_test: &str) -> anyhow::Result<()> {
    for ordinal in 1..=family.max_ordinal() {
        let leaf = map_region_loop_leaf_v1(family, ordinal);
        let descriptor =
            map_region_loop_descriptor_v1(family, ordinal, RunnerCapabilityV1::Supported);
        let (key, member) = supported_key_and_member(family, ordinal);
        let plan = compile_for_test(&key);
        let execution = match run_isolated_for_test(exact_test, &key, member, plan)? {
            // A child executes only the selector chosen by its parent. It must continue through
            // the family so that the unique matching member is reached exactly once.
            MapRunnerIsolatedOutcomeV1::ChildReported => continue,
            MapRunnerIsolatedOutcomeV1::ParentReceipt(receipt) => receipt,
        };
        let validated = project_validated_dynamic_terminal_with_map_execution_v1(
            &leaf.record,
            &descriptor,
            execution,
        )
        .map_err(|error| {
            anyhow::anyhow!(
                "supported Map region-loop member {family:?}/{ordinal} failed: {error:?}"
            )
        })?;
        assert!(validated.projection.is_ok());
        assert!(matches!(
            validated.runner_admission.decision(),
            RunnerAdmissionDecisionV1::Supported { .. }
        ));
    }
    Ok(())
}

#[cfg(windows)]
#[test]
fn isolated_created_first_empty_extend_family_receipts_are_exact() -> anyhow::Result<()> {
    exercise_isolated_family(
        MapRegionLoopFamilyV1::CreatedFirstEmptyExtend,
        &format!(
            "{EXACT_TEST_PREFIX}isolated_created_first_empty_extend_family_receipts_are_exact"
        ),
    )
}

#[cfg(windows)]
#[test]
fn isolated_node_live_target_missing_extend_family_receipts_are_exact() -> anyhow::Result<()> {
    exercise_isolated_family(
        MapRegionLoopFamilyV1::NodeLiveTargetMissingExtend,
        &format!(
            "{EXACT_TEST_PREFIX}isolated_node_live_target_missing_extend_family_receipts_are_exact"
        ),
    )
}
