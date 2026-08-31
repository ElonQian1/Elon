use super::*;

const TARGET: ExactTarget = (7, 11);
const REQUEST: MapRequest = MapRequest {
    region: 1,
    region_size: 32_768,
    mode: ManagedSqliteShmMapMode::Extend,
};

fn expectation(path: ManagedSqliteShmTestMapPath) -> ManagedSqliteShmTestMapExpectation {
    ManagedSqliteShmTestMapExpectation {
        region: REQUEST.region,
        region_size: REQUEST.region_size,
        mode: if path == ManagedSqliteShmTestMapPath::NotPresent {
            ManagedSqliteShmMapMode::Observe
        } else {
            REQUEST.mode
        },
        path,
        dms_path: ManagedSqliteShmTestMapDmsPath::NodeLive,
    }
}

fn complete(
    controller: &mut ManagedSqliteShmTestMapController,
    expected: ManagedSqliteShmTestMapExpectation,
    before: u64,
    logical_end: u64,
) {
    let request = MapRequest {
        region: expected.region,
        region_size: expected.region_size,
        mode: expected.mode,
    };
    controller
        .record(TARGET, request, MapEvent::ManagedAttempt)
        .unwrap();
    controller
        .record(TARGET, request, MapEvent::DmsPath(expected.dms_path))
        .unwrap();
    if expected.dms_path == ManagedSqliteShmTestMapDmsPath::CreatedFirstShared {
        for phase in [
            ManagedSqliteShmFailurePhase::DmsExclusiveAcquire,
            ManagedSqliteShmFailurePhase::DmsTruncate,
            ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
            ManagedSqliteShmFailurePhase::DmsSharedAcquire,
        ] {
            controller.record_dms_phase(TARGET, phase).unwrap();
        }
    }
    controller
        .record(TARGET, request, MapEvent::DmsReady)
        .unwrap();
    controller
        .record(
            TARGET,
            request,
            MapEvent::FileSize {
                before,
                logical_end,
            },
        )
        .unwrap();
    match expected.path {
        ManagedSqliteShmTestMapPath::NotPresent => {
            controller
                .record(TARGET, request, MapEvent::NotPresent)
                .unwrap();
        }
        ManagedSqliteShmTestMapPath::MappedNew => {
            for event in [
                MapEvent::FileGrow,
                MapEvent::MappingCreate,
                MapEvent::ViewMap,
                MapEvent::Record,
            ] {
                controller.record(TARGET, request, event).unwrap();
            }
            select(controller, request, expected.path);
        }
        ManagedSqliteShmTestMapPath::MappedReuse => select(controller, request, expected.path),
    }
    controller
        .record(TARGET, request, MapEvent::ManagedSuccess)
        .unwrap();
}

fn select(
    controller: &mut ManagedSqliteShmTestMapController,
    request: MapRequest,
    path: ManagedSqliteShmTestMapPath,
) {
    controller
        .record(
            TARGET,
            request,
            MapEvent::Selected {
                path,
                pointer: NonNull::dangling(),
                length: 32_768,
                runtime_generation: TARGET.0,
            },
        )
        .unwrap();
}

#[test]
fn all_six_lifecycle_cases_finish_with_exact_counts() {
    for (path, dms_path, region, mode, before, logical_end) in [
        (
            ManagedSqliteShmTestMapPath::NotPresent,
            ManagedSqliteShmTestMapDmsPath::CreatedFirstShared,
            0,
            ManagedSqliteShmMapMode::Observe,
            0,
            32_768,
        ),
        (
            ManagedSqliteShmTestMapPath::MappedNew,
            ManagedSqliteShmTestMapDmsPath::CreatedFirstShared,
            0,
            ManagedSqliteShmMapMode::Extend,
            0,
            32_768,
        ),
        (
            ManagedSqliteShmTestMapPath::MappedReuse,
            ManagedSqliteShmTestMapDmsPath::NodeLive,
            0,
            ManagedSqliteShmMapMode::Observe,
            32_768,
            32_768,
        ),
        (
            ManagedSqliteShmTestMapPath::MappedReuse,
            ManagedSqliteShmTestMapDmsPath::NodeLive,
            0,
            ManagedSqliteShmMapMode::Extend,
            32_768,
            32_768,
        ),
        (
            ManagedSqliteShmTestMapPath::NotPresent,
            ManagedSqliteShmTestMapDmsPath::NodeLive,
            1,
            ManagedSqliteShmMapMode::Observe,
            32_768,
            65_536,
        ),
        (
            ManagedSqliteShmTestMapPath::MappedNew,
            ManagedSqliteShmTestMapDmsPath::NodeLive,
            1,
            ManagedSqliteShmMapMode::Extend,
            32_768,
            65_536,
        ),
    ] {
        let expected = ManagedSqliteShmTestMapExpectation {
            region,
            region_size: 32_768,
            mode,
            path,
            dms_path,
        };
        let mut controller = ManagedSqliteShmTestMapController::default();
        controller.arm(TARGET, expected).unwrap();
        complete(&mut controller, expected, before, logical_end);
        let receipt = controller.finish(TARGET).unwrap();
        assert!(receipt.finished);
        assert_eq!(receipt.file_size_before, before);
        assert_eq!(receipt.logical_end, logical_end);
        assert_eq!(receipt.dms_ready, 1);
        let created = u8::from(dms_path == ManagedSqliteShmTestMapDmsPath::CreatedFirstShared);
        let mapped_new = u8::from(path == ManagedSqliteShmTestMapPath::MappedNew);
        let mapped_reuse = u8::from(path == ManagedSqliteShmTestMapPath::MappedReuse);
        assert_eq!(receipt.dms_exclusive_acquires, created);
        assert_eq!(receipt.dms_truncates, created);
        assert_eq!(receipt.dms_exclusive_releases, created);
        assert_eq!(receipt.dms_shared_acquires, created);
        assert_eq!(receipt.file_grows, mapped_new);
        assert_eq!(receipt.mapping_creates, mapped_new);
        assert_eq!(receipt.view_maps, mapped_new);
        assert_eq!(receipt.records, mapped_new);
        assert_eq!(receipt.mapped_new, mapped_new);
        assert_eq!(receipt.mapped_reuses, mapped_reuse);
        assert_eq!(
            receipt.not_present,
            u8::from(path == ManagedSqliteShmTestMapPath::NotPresent)
        );
        assert_eq!(
            receipt.mapped,
            u8::from(path != ManagedSqliteShmTestMapPath::NotPresent)
        );
        if path != ManagedSqliteShmTestMapPath::NotPresent {
            assert!(receipt.selected_pointer_matches(NonNull::<u8>::dangling().as_ptr()));
            assert!(format!("{receipt:?}").contains("<mapped>"));
        }
    }
}

#[test]
fn wrong_target_request_path_and_sequence_fail_closed() {
    let mut controller = ManagedSqliteShmTestMapController::default();
    controller
        .arm(TARGET, expectation(ManagedSqliteShmTestMapPath::MappedNew))
        .unwrap();
    assert!(controller
        .record((7, 12), REQUEST, MapEvent::ManagedAttempt)
        .is_err());
    assert!(controller.finish(TARGET).is_err());
    controller
        .arm(TARGET, expectation(ManagedSqliteShmTestMapPath::MappedNew))
        .unwrap();
    assert!(controller
        .record(
            TARGET,
            MapRequest {
                region: 0,
                ..REQUEST
            },
            MapEvent::ManagedAttempt,
        )
        .is_err());
    assert!(controller.finish(TARGET).is_err());
    controller
        .arm(TARGET, expectation(ManagedSqliteShmTestMapPath::MappedNew))
        .unwrap();
    controller
        .record(TARGET, REQUEST, MapEvent::ManagedAttempt)
        .unwrap();
    assert!(controller
        .record(TARGET, REQUEST, MapEvent::NotPresent)
        .is_err());
    assert!(controller.finish(TARGET).is_err());
}

#[test]
fn duplicate_and_incomplete_finish_disarm_for_rearm() {
    let mut controller = ManagedSqliteShmTestMapController::default();
    let expected = expectation(ManagedSqliteShmTestMapPath::MappedNew);
    controller.arm(TARGET, expected).unwrap();
    controller
        .record(TARGET, REQUEST, MapEvent::ManagedAttempt)
        .unwrap();
    assert!(controller
        .record(TARGET, REQUEST, MapEvent::ManagedAttempt)
        .is_err());
    assert!(controller.finish(TARGET).is_err());
    controller.arm(TARGET, expected).unwrap();
    assert!(controller.finish(TARGET).is_err());
    controller.arm(TARGET, expected).unwrap();
}

#[test]
fn setup_before_arm_and_cleanup_after_finish_are_not_observed() {
    let mut controller = ManagedSqliteShmTestMapController::default();
    controller
        .record(TARGET, REQUEST, MapEvent::ManagedAttempt)
        .unwrap();
    let expected = expectation(ManagedSqliteShmTestMapPath::MappedNew);
    controller.arm(TARGET, expected).unwrap();
    complete(&mut controller, expected, 32_768, 65_536);
    controller.finish(TARGET).unwrap();
    controller
        .record(TARGET, REQUEST, MapEvent::ManagedAttempt)
        .unwrap();
    controller.arm(TARGET, expected).unwrap();
}

#[test]
fn cancel_requires_exact_target_and_preserves_wrong_target_state() {
    let mut controller = ManagedSqliteShmTestMapController::default();
    let expected = expectation(ManagedSqliteShmTestMapPath::MappedNew);
    controller.arm(TARGET, expected).unwrap();
    assert!(controller.cancel((7, 12)).is_err());
    assert!(controller.arm(TARGET, expected).is_err());
    controller.cancel(TARGET).unwrap();
    controller.arm(TARGET, expected).unwrap();
}
