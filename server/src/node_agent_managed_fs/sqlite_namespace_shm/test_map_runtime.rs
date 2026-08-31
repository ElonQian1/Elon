//! Exact-target, Windows-test-only observation for one managed SHM Map action.

use std::{fmt, ptr::NonNull};

use super::{
    coordinator::ManagedSqliteShmCoordinator,
    types::{ManagedSqliteShmFailure, ManagedSqliteShmFailurePhase, ManagedSqliteShmMapMode},
};

mod mapping_sequence;

use mapping_sequence::{validate_expectation, MappingSequence, MappingSequenceEvent};

type ExactTarget = (u64, u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmTestMapPath {
    NotPresent,
    MappedNew,
    MappedReuse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmTestMapDmsPath {
    CreatedFirstShared,
    NodeLive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestMapExpectation {
    pub(crate) region: u32,
    pub(crate) region_size: u32,
    pub(crate) regions_to_create: u16,
    pub(crate) mode: ManagedSqliteShmMapMode,
    pub(crate) path: ManagedSqliteShmTestMapPath,
    pub(crate) dms_path: ManagedSqliteShmTestMapDmsPath,
}

/// Address-sized equality token retained only by the in-process test receipt.
///
/// It is never converted back into a pointer, formatted as an address, serialized or hashed.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestMapPointerIdentity(usize);

impl ManagedSqliteShmTestMapPointerIdentity {
    pub(crate) fn matches(self, pointer: *mut u8) -> bool {
        self.0 == pointer as usize
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestMapReceipt {
    pub(crate) runtime_generation: u64,
    pub(crate) shm_connection_id: u64,
    pub(crate) expectation: ManagedSqliteShmTestMapExpectation,
    pub(crate) managed_attempts: u16,
    pub(crate) created_first_shared: u16,
    pub(crate) node_live: u16,
    pub(crate) dms_exclusive_acquires: u16,
    pub(crate) dms_truncates: u16,
    pub(crate) dms_exclusive_releases: u16,
    pub(crate) dms_shared_acquires: u16,
    pub(crate) dms_ready: u16,
    pub(crate) file_size_checks: u16,
    pub(crate) file_size_before: u64,
    pub(crate) logical_end: u64,
    pub(crate) file_grows: u16,
    pub(crate) mapping_creates: u16,
    pub(crate) view_maps: u16,
    pub(crate) records: u16,
    pub(crate) not_present: u16,
    pub(crate) mapped: u16,
    pub(crate) mapped_new: u16,
    pub(crate) mapped_reuses: u16,
    pub(crate) selected_pointer: Option<ManagedSqliteShmTestMapPointerIdentity>,
    pub(crate) selected_length: usize,
    pub(crate) selected_region: Option<u32>,
    pub(crate) selected_runtime_generation: Option<u64>,
    pub(crate) managed_successes: u16,
    pub(crate) finished: bool,
}

impl ManagedSqliteShmTestMapReceipt {
    pub(crate) fn selected_pointer_matches(&self, pointer: *mut u8) -> bool {
        self.selected_pointer
            .is_some_and(|selected| selected.matches(pointer))
    }
}

impl fmt::Debug for ManagedSqliteShmTestMapReceipt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedSqliteShmTestMapReceipt")
            .field("runtime_generation", &self.runtime_generation)
            .field("shm_connection_id", &self.shm_connection_id)
            .field("expectation", &self.expectation)
            .field("managed_attempts", &self.managed_attempts)
            .field("created_first_shared", &self.created_first_shared)
            .field("node_live", &self.node_live)
            .field("dms_ready", &self.dms_ready)
            .field("file_size_checks", &self.file_size_checks)
            .field("file_grows", &self.file_grows)
            .field("mapping_creates", &self.mapping_creates)
            .field("view_maps", &self.view_maps)
            .field("records", &self.records)
            .field("not_present", &self.not_present)
            .field("mapped", &self.mapped)
            .field(
                "selected_pointer",
                &self.selected_pointer.map(|_| "<mapped>"),
            )
            .field("selected_length", &self.selected_length)
            .field("selected_region", &self.selected_region)
            .field("managed_successes", &self.managed_successes)
            .field("finished", &self.finished)
            .finish()
    }
}

#[derive(Clone, Copy)]
struct MapRequest {
    region: u32,
    region_size: u32,
    mode: ManagedSqliteShmMapMode,
}

enum MapEvent {
    ManagedAttempt,
    DmsPath(ManagedSqliteShmTestMapDmsPath),
    DmsReady,
    FileSize {
        before: u64,
        logical_end: u64,
    },
    FileGrow,
    MappingCreate(u16),
    ViewMap(u16),
    Record(u16),
    NotPresent,
    Selected {
        path: ManagedSqliteShmTestMapPath,
        pointer: NonNull<u8>,
        length: usize,
        runtime_generation: u64,
    },
    ManagedSuccess,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Progress {
    Armed,
    ManagedAttempted,
    DmsPathObserved,
    DmsExclusiveAcquired,
    DmsTruncated,
    DmsExclusiveReleased,
    DmsSharedAcquired,
    DmsReady,
    FileSized,
    MappingSequence,
    MappingSequenceCompleted,
    NotPresent,
    Mapped,
    ManagedSucceeded,
}

struct ArmedMapObservation {
    target: ExactTarget,
    receipt: ManagedSqliteShmTestMapReceipt,
    progress: Progress,
    mapping_sequence: MappingSequence,
    invalid: bool,
}

#[derive(Default)]
pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) struct ManagedSqliteShmTestMapController
{
    armed: Option<ArmedMapObservation>,
}

impl ManagedSqliteShmTestMapController {
    pub(super) fn arm(
        &mut self,
        target: ExactTarget,
        expectation: ManagedSqliteShmTestMapExpectation,
    ) -> Result<(), &'static str> {
        validate_expectation(target, expectation)?;
        if self.armed.is_some() {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_OBSERVATION_ALREADY_ARMED");
        }
        self.armed = Some(ArmedMapObservation {
            target,
            receipt: ManagedSqliteShmTestMapReceipt {
                runtime_generation: target.0,
                shm_connection_id: target.1,
                expectation,
                managed_attempts: 0,
                created_first_shared: 0,
                node_live: 0,
                dms_exclusive_acquires: 0,
                dms_truncates: 0,
                dms_exclusive_releases: 0,
                dms_shared_acquires: 0,
                dms_ready: 0,
                file_size_checks: 0,
                file_size_before: 0,
                logical_end: 0,
                file_grows: 0,
                mapping_creates: 0,
                view_maps: 0,
                records: 0,
                not_present: 0,
                mapped: 0,
                mapped_new: 0,
                mapped_reuses: 0,
                selected_pointer: None,
                selected_length: 0,
                selected_region: None,
                selected_runtime_generation: None,
                managed_successes: 0,
                finished: false,
            },
            progress: Progress::Armed,
            mapping_sequence: MappingSequence::new(expectation.regions_to_create),
            invalid: false,
        });
        Ok(())
    }

    fn record(
        &mut self,
        target: ExactTarget,
        request: MapRequest,
        event: MapEvent,
    ) -> Result<(), &'static str> {
        let Some(armed) = self.armed.as_mut() else {
            return Ok(());
        };
        if armed.target != target {
            armed.invalid = true;
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_TARGET_MISMATCH");
        }
        require_request(armed, request)?;
        macro_rules! once {
            ($counter:ident, $required:expr, $next:expr, $duplicate:literal) => {{
                if armed.receipt.$counter != 0 {
                    armed.invalid = true;
                    return Err($duplicate);
                }
                require_progress(armed, $required)?;
                armed.receipt.$counter = 1;
                armed.progress = $next;
            }};
        }
        match event {
            MapEvent::ManagedAttempt => once!(
                managed_attempts,
                Progress::Armed,
                Progress::ManagedAttempted,
                "NODE_MANAGED_SQLITE_SHM_TEST_MAP_MANAGED_ATTEMPT_DUPLICATE"
            ),
            MapEvent::DmsPath(path) => {
                if path != armed.receipt.expectation.dms_path {
                    armed.invalid = true;
                    return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_DMS_PATH_MISMATCH");
                }
                require_progress(armed, Progress::ManagedAttempted)?;
                match path {
                    ManagedSqliteShmTestMapDmsPath::CreatedFirstShared => {
                        armed.receipt.created_first_shared = 1;
                    }
                    ManagedSqliteShmTestMapDmsPath::NodeLive => armed.receipt.node_live = 1,
                }
                armed.progress = Progress::DmsPathObserved;
            }
            MapEvent::DmsReady => {
                let required = match armed.receipt.expectation.dms_path {
                    ManagedSqliteShmTestMapDmsPath::CreatedFirstShared => {
                        Progress::DmsSharedAcquired
                    }
                    ManagedSqliteShmTestMapDmsPath::NodeLive => Progress::DmsPathObserved,
                };
                once!(
                    dms_ready,
                    required,
                    Progress::DmsReady,
                    "NODE_MANAGED_SQLITE_SHM_TEST_MAP_DMS_READY_DUPLICATE"
                );
            }
            MapEvent::FileSize {
                before,
                logical_end,
            } => {
                once!(
                    file_size_checks,
                    Progress::DmsReady,
                    Progress::FileSized,
                    "NODE_MANAGED_SQLITE_SHM_TEST_MAP_FILE_SIZE_DUPLICATE"
                );
                let short = before < logical_end;
                let expected_short =
                    armed.receipt.expectation.path != ManagedSqliteShmTestMapPath::MappedReuse;
                let expected_end = (u64::from(armed.receipt.expectation.region) + 1)
                    * u64::from(armed.receipt.expectation.region_size);
                if logical_end != expected_end || short != expected_short {
                    armed.invalid = true;
                    return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_FILE_SIZE_PATH_MISMATCH");
                }
                armed.receipt.file_size_before = before;
                armed.receipt.logical_end = logical_end;
            }
            MapEvent::FileGrow => {
                require_path(armed, ManagedSqliteShmTestMapPath::MappedNew)?;
                once!(
                    file_grows,
                    Progress::FileSized,
                    Progress::MappingSequence,
                    "NODE_MANAGED_SQLITE_SHM_TEST_MAP_FILE_GROW_DUPLICATE"
                );
            }
            MapEvent::MappingCreate(ordinal) => {
                require_path(armed, ManagedSqliteShmTestMapPath::MappedNew)?;
                observe_mapping_sequence(armed, MappingSequenceEvent::MappingCreate(ordinal))?;
            }
            MapEvent::ViewMap(ordinal) => {
                require_path(armed, ManagedSqliteShmTestMapPath::MappedNew)?;
                observe_mapping_sequence(armed, MappingSequenceEvent::ViewMap(ordinal))?;
            }
            MapEvent::Record(ordinal) => {
                require_path(armed, ManagedSqliteShmTestMapPath::MappedNew)?;
                observe_mapping_sequence(armed, MappingSequenceEvent::Record(ordinal))?;
            }
            MapEvent::NotPresent => {
                require_path(armed, ManagedSqliteShmTestMapPath::NotPresent)?;
                once!(
                    not_present,
                    Progress::FileSized,
                    Progress::NotPresent,
                    "NODE_MANAGED_SQLITE_SHM_TEST_MAP_NOT_PRESENT_DUPLICATE"
                );
            }
            MapEvent::Selected {
                path,
                pointer,
                length,
                runtime_generation,
            } => {
                require_path(armed, path)?;
                let required = match path {
                    ManagedSqliteShmTestMapPath::MappedNew => Progress::MappingSequenceCompleted,
                    ManagedSqliteShmTestMapPath::MappedReuse => Progress::FileSized,
                    ManagedSqliteShmTestMapPath::NotPresent => {
                        armed.invalid = true;
                        return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_NOT_PRESENT_SELECTED");
                    }
                };
                once!(
                    mapped,
                    required,
                    Progress::Mapped,
                    "NODE_MANAGED_SQLITE_SHM_TEST_MAP_SELECTED_DUPLICATE"
                );
                if length != armed.receipt.expectation.region_size as usize
                    || runtime_generation != target.0
                {
                    armed.invalid = true;
                    return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_SELECTION_MISMATCH");
                }
                armed.receipt.selected_pointer = Some(ManagedSqliteShmTestMapPointerIdentity(
                    pointer.as_ptr() as usize,
                ));
                armed.receipt.selected_length = length;
                armed.receipt.selected_region = Some(request.region);
                armed.receipt.selected_runtime_generation = Some(runtime_generation);
                match path {
                    ManagedSqliteShmTestMapPath::MappedNew => armed.receipt.mapped_new = 1,
                    ManagedSqliteShmTestMapPath::MappedReuse => armed.receipt.mapped_reuses = 1,
                    ManagedSqliteShmTestMapPath::NotPresent => unreachable!(),
                }
            }
            MapEvent::ManagedSuccess => {
                let required = match armed.receipt.expectation.path {
                    ManagedSqliteShmTestMapPath::NotPresent => Progress::NotPresent,
                    ManagedSqliteShmTestMapPath::MappedNew
                    | ManagedSqliteShmTestMapPath::MappedReuse => Progress::Mapped,
                };
                once!(
                    managed_successes,
                    required,
                    Progress::ManagedSucceeded,
                    "NODE_MANAGED_SQLITE_SHM_TEST_MAP_MANAGED_SUCCESS_DUPLICATE"
                );
            }
        }
        Ok(())
    }

    pub(super) fn finish(
        &mut self,
        target: ExactTarget,
    ) -> Result<ManagedSqliteShmTestMapReceipt, &'static str> {
        if self.armed.as_ref().map(|armed| armed.target) != Some(target) {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_TARGET_MISMATCH");
        }
        let armed = self
            .armed
            .take()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_MAP_OBSERVATION_NOT_ARMED")?;
        if armed.invalid {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_OBSERVATION_INVALID");
        }
        if armed.progress != Progress::ManagedSucceeded {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_OBSERVATION_INCOMPLETE");
        }
        let mut receipt = armed.receipt;
        receipt.finished = true;
        Ok(receipt)
    }

    pub(super) fn cancel(&mut self, target: ExactTarget) -> Result<(), &'static str> {
        if self.armed.as_ref().map(|armed| armed.target) != Some(target) {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_TARGET_MISMATCH");
        }
        self.armed.take();
        Ok(())
    }

    fn record_dms_phase(
        &mut self,
        target: ExactTarget,
        phase: ManagedSqliteShmFailurePhase,
    ) -> Result<(), &'static str> {
        let Some(armed) = self.armed.as_mut() else {
            return Ok(());
        };
        if armed.target != target {
            armed.invalid = true;
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_TARGET_MISMATCH");
        }
        require_dms_phase(armed, phase)
    }
}

fn observe_mapping_sequence(
    armed: &mut ArmedMapObservation,
    event: MappingSequenceEvent,
) -> Result<(), &'static str> {
    require_progress(armed, Progress::MappingSequence)?;
    let counts = match armed.mapping_sequence.observe(event) {
        Ok(counts) => counts,
        Err(error) => {
            armed.invalid = true;
            return Err(error);
        }
    };
    armed.receipt.mapping_creates = counts.mapping_creates;
    armed.receipt.view_maps = counts.view_maps;
    armed.receipt.records = counts.records;
    if armed.mapping_sequence.is_complete() {
        armed.progress = Progress::MappingSequenceCompleted;
    }
    Ok(())
}

fn require_request(
    armed: &mut ArmedMapObservation,
    request: MapRequest,
) -> Result<(), &'static str> {
    let expected = armed.receipt.expectation;
    if expected.region != request.region
        || expected.region_size != request.region_size
        || expected.mode != request.mode
    {
        armed.invalid = true;
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_REQUEST_MISMATCH");
    }
    Ok(())
}

fn require_path(
    armed: &mut ArmedMapObservation,
    path: ManagedSqliteShmTestMapPath,
) -> Result<(), &'static str> {
    if armed.receipt.expectation.path != path {
        armed.invalid = true;
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_PATH_MISMATCH");
    }
    Ok(())
}

fn require_progress(
    armed: &mut ArmedMapObservation,
    progress: Progress,
) -> Result<(), &'static str> {
    if armed.progress != progress {
        armed.invalid = true;
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_EVENT_SEQUENCE_INVALID");
    }
    Ok(())
}

fn require_dms_phase(
    armed: &mut ArmedMapObservation,
    phase: ManagedSqliteShmFailurePhase,
) -> Result<(), &'static str> {
    if armed.receipt.expectation.dms_path != ManagedSqliteShmTestMapDmsPath::CreatedFirstShared {
        armed.invalid = true;
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_DMS_PHASE_ON_LIVE_NODE");
    }
    macro_rules! dms_once {
        ($counter:ident, $required:expr, $next:expr, $duplicate:literal) => {{
            if armed.receipt.$counter != 0 {
                armed.invalid = true;
                return Err($duplicate);
            }
            require_progress(armed, $required)?;
            armed.receipt.$counter = 1;
            armed.progress = $next;
        }};
    }
    match phase {
        ManagedSqliteShmFailurePhase::DmsExclusiveAcquire => dms_once!(
            dms_exclusive_acquires,
            Progress::DmsPathObserved,
            Progress::DmsExclusiveAcquired,
            "NODE_MANAGED_SQLITE_SHM_TEST_MAP_DMS_EXCLUSIVE_ACQUIRE_DUPLICATE"
        ),
        ManagedSqliteShmFailurePhase::DmsTruncate => dms_once!(
            dms_truncates,
            Progress::DmsExclusiveAcquired,
            Progress::DmsTruncated,
            "NODE_MANAGED_SQLITE_SHM_TEST_MAP_DMS_TRUNCATE_DUPLICATE"
        ),
        ManagedSqliteShmFailurePhase::DmsExclusiveRelease => dms_once!(
            dms_exclusive_releases,
            Progress::DmsTruncated,
            Progress::DmsExclusiveReleased,
            "NODE_MANAGED_SQLITE_SHM_TEST_MAP_DMS_EXCLUSIVE_RELEASE_DUPLICATE"
        ),
        ManagedSqliteShmFailurePhase::DmsSharedAcquire => dms_once!(
            dms_shared_acquires,
            Progress::DmsExclusiveReleased,
            Progress::DmsSharedAcquired,
            "NODE_MANAGED_SQLITE_SHM_TEST_MAP_DMS_SHARED_ACQUIRE_DUPLICATE"
        ),
        _ => {
            armed.invalid = true;
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_DMS_PHASE_UNEXPECTED");
        }
    }
    Ok(())
}

impl ManagedSqliteShmCoordinator {
    pub(super) fn begin_test_map_action(
        &self,
        connection_id: u64,
        region: u32,
        region_size: u32,
        mode: ManagedSqliteShmMapMode,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_test_map_event(
            connection_id,
            region,
            region_size,
            mode,
            ManagedSqliteShmFailurePhase::RequestValidation,
            false,
            MapEvent::ManagedAttempt,
        )
    }

    pub(super) fn record_test_map_dms_path(
        &self,
        connection_id: u64,
        request: (u32, u32, ManagedSqliteShmMapMode),
        path: ManagedSqliteShmTestMapDmsPath,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_test_map_event(
            connection_id,
            request.0,
            request.1,
            request.2,
            ManagedSqliteShmFailurePhase::Gate,
            false,
            MapEvent::DmsPath(path),
        )
    }

    pub(super) fn record_test_map_dms_ready(
        &self,
        connection_id: u64,
        request: (u32, u32, ManagedSqliteShmMapMode),
        known_mutation: bool,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_test_map_event(
            connection_id,
            request.0,
            request.1,
            request.2,
            ManagedSqliteShmFailurePhase::Gate,
            known_mutation,
            MapEvent::DmsReady,
        )
    }

    pub(super) fn record_test_map_dms_phase(
        &self,
        connection_id: u64,
        phase: ManagedSqliteShmFailurePhase,
        known_mutation: bool,
    ) -> Result<(), ManagedSqliteShmFailure> {
        if !matches!(
            phase,
            ManagedSqliteShmFailurePhase::DmsExclusiveAcquire
                | ManagedSqliteShmFailurePhase::DmsTruncate
                | ManagedSqliteShmFailurePhase::DmsExclusiveRelease
                | ManagedSqliteShmFailurePhase::DmsSharedAcquire
        ) {
            return Ok(());
        }
        let target = (self.generation.get(), connection_id);
        self.test_map_runtime
            .lock()
            .map_err(|_| self.test_map_runtime_failure(phase, known_mutation))
            .and_then(|mut controller| {
                controller
                    .record_dms_phase(target, phase)
                    .map_err(|_| self.test_map_runtime_failure(phase, known_mutation))
            })
    }

    pub(super) fn record_test_map_file_size(
        &self,
        connection_id: u64,
        request: (u32, u32, ManagedSqliteShmMapMode),
        before: u64,
        logical_end: u64,
        known_mutation: bool,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_test_map_event(
            connection_id,
            request.0,
            request.1,
            request.2,
            ManagedSqliteShmFailurePhase::FileSize,
            known_mutation,
            MapEvent::FileSize {
                before,
                logical_end,
            },
        )
    }

    pub(super) fn record_test_map_step(
        &self,
        connection_id: u64,
        request: (u32, u32, ManagedSqliteShmMapMode),
        phase: ManagedSqliteShmFailurePhase,
        known_mutation: bool,
        step: ManagedSqliteShmTestMapStep,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let event = match step {
            ManagedSqliteShmTestMapStep::FileGrow => MapEvent::FileGrow,
            ManagedSqliteShmTestMapStep::MappingCreate(ordinal) => MapEvent::MappingCreate(ordinal),
            ManagedSqliteShmTestMapStep::ViewMap(ordinal) => MapEvent::ViewMap(ordinal),
            ManagedSqliteShmTestMapStep::Record(ordinal) => MapEvent::Record(ordinal),
            ManagedSqliteShmTestMapStep::NotPresent => MapEvent::NotPresent,
        };
        self.record_test_map_event(
            connection_id,
            request.0,
            request.1,
            request.2,
            phase,
            known_mutation,
            event,
        )
    }

    pub(super) fn record_test_map_selected(
        &self,
        connection_id: u64,
        request: (u32, u32, ManagedSqliteShmMapMode),
        path: ManagedSqliteShmTestMapPath,
        pointer: NonNull<u8>,
        length: usize,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_test_map_event(
            connection_id,
            request.0,
            request.1,
            request.2,
            ManagedSqliteShmFailurePhase::Gate,
            path == ManagedSqliteShmTestMapPath::MappedNew,
            MapEvent::Selected {
                path,
                pointer,
                length,
                runtime_generation: self.generation.get(),
            },
        )
    }

    pub(super) fn finish_test_map_action(
        &self,
        connection_id: u64,
        request: (u32, u32, ManagedSqliteShmMapMode),
        known_mutation: bool,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_test_map_event(
            connection_id,
            request.0,
            request.1,
            request.2,
            ManagedSqliteShmFailurePhase::Gate,
            known_mutation,
            MapEvent::ManagedSuccess,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn record_test_map_event(
        &self,
        connection_id: u64,
        region: u32,
        region_size: u32,
        mode: ManagedSqliteShmMapMode,
        phase: ManagedSqliteShmFailurePhase,
        known_mutation: bool,
        event: MapEvent,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let target = (self.generation.get(), connection_id);
        let request = MapRequest {
            region,
            region_size,
            mode,
        };
        self.test_map_runtime
            .lock()
            .map_err(|_| self.test_map_runtime_failure(phase, known_mutation))
            .and_then(|mut controller| {
                controller
                    .record(target, request, event)
                    .map_err(|_| self.test_map_runtime_failure(phase, known_mutation))
            })
    }

    fn test_map_runtime_failure(
        &self,
        phase: ManagedSqliteShmFailurePhase,
        known_mutation: bool,
    ) -> ManagedSqliteShmFailure {
        self.mark_domain_terminal();
        ManagedSqliteShmFailure::poisoned_code(
            phase,
            "NODE_MANAGED_SQLITE_SHM_TEST_MAP_RUNTIME_INVALID",
            known_mutation,
            known_mutation,
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedSqliteShmTestMapStep {
    FileGrow,
    MappingCreate(u16),
    ViewMap(u16),
    Record(u16),
    NotPresent,
}

#[cfg(test)]
#[path = "test_map_runtime/tests.rs"]
mod tests;
