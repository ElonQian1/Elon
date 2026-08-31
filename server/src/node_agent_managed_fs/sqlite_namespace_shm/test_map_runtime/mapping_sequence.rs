//! Ordered accounting for the mapping loop of one observed Map action.

use super::{
    ExactTarget, ManagedSqliteShmMapMode, ManagedSqliteShmTestMapDmsPath,
    ManagedSqliteShmTestMapExpectation, ManagedSqliteShmTestMapPath,
};

pub(super) fn validate_expectation(
    target: ExactTarget,
    expectation: ManagedSqliteShmTestMapExpectation,
) -> Result<(), &'static str> {
    if target.0 == 0 || target.1 == 0 {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_TARGET_ZERO");
    }
    if expectation.region_size == 0 {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_REGION_SIZE_ZERO");
    }
    let path_matches_mode = match expectation.path {
        ManagedSqliteShmTestMapPath::NotPresent => {
            expectation.mode == ManagedSqliteShmMapMode::Observe
        }
        ManagedSqliteShmTestMapPath::MappedNew => {
            expectation.mode == ManagedSqliteShmMapMode::Extend
        }
        ManagedSqliteShmTestMapPath::MappedReuse => true,
    };
    if !path_matches_mode {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_MODE_PATH_MISMATCH");
    }
    let exact_lifecycle = match (expectation.dms_path, expectation.path) {
        (
            ManagedSqliteShmTestMapDmsPath::CreatedFirstShared,
            ManagedSqliteShmTestMapPath::NotPresent,
        ) => expectation.region == 0 && expectation.regions_to_create == 0,
        (
            ManagedSqliteShmTestMapDmsPath::CreatedFirstShared,
            ManagedSqliteShmTestMapPath::MappedNew,
        ) => {
            expectation.region <= 255
                && expectation.regions_to_create == expectation.region as u16 + 1
        }
        (ManagedSqliteShmTestMapDmsPath::NodeLive, ManagedSqliteShmTestMapPath::MappedReuse) => {
            expectation.region == 0 && expectation.regions_to_create == 0
        }
        (ManagedSqliteShmTestMapDmsPath::NodeLive, ManagedSqliteShmTestMapPath::NotPresent) => {
            expectation.region == 1 && expectation.regions_to_create == 0
        }
        (ManagedSqliteShmTestMapDmsPath::NodeLive, ManagedSqliteShmTestMapPath::MappedNew) => {
            (1..=255).contains(&expectation.region)
                && expectation.regions_to_create == expectation.region as u16
        }
        _ => false,
    };
    if !exact_lifecycle {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_LIFECYCLE_EXPECTATION_INVALID");
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) enum MappingSequenceEvent {
    MappingCreate(u16),
    ViewMap(u16),
    Record(u16),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MappingSequenceStage {
    MappingCreate,
    ViewMap,
    Record,
    Complete,
}

#[derive(Clone, Copy)]
pub(super) struct MappingSequenceCounts {
    pub(super) mapping_creates: u16,
    pub(super) view_maps: u16,
    pub(super) records: u16,
}

pub(super) struct MappingSequence {
    expected: u16,
    counts: MappingSequenceCounts,
    stage: MappingSequenceStage,
}

impl MappingSequence {
    pub(super) fn new(expected: u16) -> Self {
        Self {
            expected,
            counts: MappingSequenceCounts {
                mapping_creates: 0,
                view_maps: 0,
                records: 0,
            },
            stage: if expected == 0 {
                MappingSequenceStage::Complete
            } else {
                MappingSequenceStage::MappingCreate
            },
        }
    }

    pub(super) fn observe(
        &mut self,
        event: MappingSequenceEvent,
    ) -> Result<MappingSequenceCounts, &'static str> {
        let next = self.counts.records;
        match (self.stage, event) {
            (MappingSequenceStage::MappingCreate, MappingSequenceEvent::MappingCreate(ordinal))
                if ordinal == next =>
            {
                self.counts.mapping_creates += 1;
                self.stage = MappingSequenceStage::ViewMap;
            }
            (MappingSequenceStage::ViewMap, MappingSequenceEvent::ViewMap(ordinal))
                if ordinal == next =>
            {
                self.counts.view_maps += 1;
                self.stage = MappingSequenceStage::Record;
            }
            (MappingSequenceStage::Record, MappingSequenceEvent::Record(ordinal))
                if ordinal == next =>
            {
                self.counts.records += 1;
                self.stage = if self.counts.records == self.expected {
                    MappingSequenceStage::Complete
                } else {
                    MappingSequenceStage::MappingCreate
                };
            }
            _ => return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_MAPPING_SEQUENCE_INVALID"),
        }
        if self.counts.mapping_creates > self.expected
            || self.counts.view_maps > self.expected
            || self.counts.records > self.expected
        {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_MAPPING_COUNT_EXCEEDED");
        }
        Ok(self.counts)
    }

    pub(super) fn is_complete(&self) -> bool {
        self.stage == MappingSequenceStage::Complete
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_complete_ordered_triples() {
        let mut sequence = MappingSequence::new(2);
        for event in [
            MappingSequenceEvent::MappingCreate(0),
            MappingSequenceEvent::ViewMap(0),
            MappingSequenceEvent::Record(0),
            MappingSequenceEvent::MappingCreate(1),
            MappingSequenceEvent::ViewMap(1),
            MappingSequenceEvent::Record(1),
        ] {
            sequence.observe(event).unwrap();
        }
        assert!(sequence.is_complete());
        assert!(sequence
            .observe(MappingSequenceEvent::MappingCreate(2))
            .is_err());
    }

    #[test]
    fn rejects_skipped_or_interleaved_phases() {
        let mut sequence = MappingSequence::new(2);
        assert!(sequence.observe(MappingSequenceEvent::ViewMap(0)).is_err());
        let mut sequence = MappingSequence::new(2);
        sequence
            .observe(MappingSequenceEvent::MappingCreate(0))
            .unwrap();
        assert!(sequence
            .observe(MappingSequenceEvent::MappingCreate(0))
            .is_err());
    }

    #[test]
    fn rejects_wrong_or_replayed_ordinals() {
        let mut sequence = MappingSequence::new(2);
        assert!(sequence
            .observe(MappingSequenceEvent::MappingCreate(1))
            .is_err());
        let mut sequence = MappingSequence::new(2);
        for event in [
            MappingSequenceEvent::MappingCreate(0),
            MappingSequenceEvent::ViewMap(0),
            MappingSequenceEvent::Record(0),
        ] {
            sequence.observe(event).unwrap();
        }
        assert!(sequence
            .observe(MappingSequenceEvent::MappingCreate(0))
            .is_err());
    }
}
