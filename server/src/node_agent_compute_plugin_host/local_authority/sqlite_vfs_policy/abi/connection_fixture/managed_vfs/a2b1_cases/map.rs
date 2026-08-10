use crate::node_agent_managed_fs::{
    ManagedSqliteShmFailureClass as Class, ManagedSqliteShmFailurePhase as Phase,
};

use super::model::{
    case, A2b1Path, Case, FaultTiming, RetainedCustody, SqliteResult, BASE_RETAINED,
};
use super::operation::{
    map_at_dms, DmsCustodyEvidence as Dms, OperationShape, REGION_ZERO_OBSERVE,
};

const NO_NODE: RetainedCustody = RetainedCustody {
    dms: Dms::Absent,
    shm_file: false,
    ..BASE_RETAINED
};

const RELEASED: RetainedCustody = RetainedCustody {
    dms: Dms::Released,
    ..BASE_RETAINED
};

const EXCLUSIVE: RetainedCustody = RetainedCustody {
    dms: Dms::ExclusiveKnown,
    ..BASE_RETAINED
};

const MAPPING_ONLY: RetainedCustody = RetainedCustody {
    mappings: 1,
    ..BASE_RETAINED
};

const MAPPED_VIEW: RetainedCustody = RetainedCustody {
    mappings: 1,
    views: 1,
    ..BASE_RETAINED
};

pub(super) const CASES: &[Case] = &[
    case(
        "map.exact_open.before",
        A2b1Path::ShmMap,
        map_operation(Phase::ExactSiblingOpen),
        FaultTiming::BeforeCall,
        Phase::ExactSiblingOpen,
        Class::IoBeforeMutation,
        SqliteResult::IoerrShmMap,
        false,
        false,
        false,
        NO_NODE,
        0,
        0,
    ),
    case(
        "map.exact_open.after",
        A2b1Path::ShmMap,
        map_operation(Phase::ExactSiblingOpen),
        FaultTiming::AfterSuccess,
        Phase::ExactSiblingOpen,
        Class::MutatedButKnown,
        SqliteResult::IoerrShmMap,
        true,
        false,
        true,
        RELEASED,
        1,
        1,
    ),
    prior_mutation_before(
        "map.dms_exclusive_acquire.before",
        Phase::DmsExclusiveAcquire,
        NO_NODE,
    ),
    uncertain_after(
        "map.dms_exclusive_acquire.after",
        Phase::DmsExclusiveAcquire,
        EXCLUSIVE,
    ),
    prior_mutation_before("map.dms_truncate.before", Phase::DmsTruncate, NO_NODE),
    known_after("map.dms_truncate.after", Phase::DmsTruncate, EXCLUSIVE),
    prior_mutation_before(
        "map.dms_exclusive_release.before",
        Phase::DmsExclusiveRelease,
        EXCLUSIVE,
    ),
    uncertain_after(
        "map.dms_exclusive_release.after",
        Phase::DmsExclusiveRelease,
        RELEASED,
    ),
    prior_mutation_before(
        "map.dms_shared_acquire.before",
        Phase::DmsSharedAcquire,
        NO_NODE,
    ),
    known_after(
        "map.dms_shared_acquire.after",
        Phase::DmsSharedAcquire,
        BASE_RETAINED,
    ),
    clean_before("map.file_size.before", Phase::FileSize),
    clean_before("map.file_grow.before", Phase::FileGrow),
    known_after("map.file_grow.after", Phase::FileGrow, BASE_RETAINED),
    prior_mutation_before(
        "map.mapping_create.before",
        Phase::MappingCreate,
        BASE_RETAINED,
    ),
    known_after(
        "map.mapping_create.after",
        Phase::MappingCreate,
        MAPPING_ONLY,
    ),
    case(
        "map.view_map.before_with_mapping",
        A2b1Path::ShmMap,
        map_operation(Phase::ViewMap),
        FaultTiming::BeforeCall,
        Phase::ViewMap,
        Class::MutatedButKnown,
        SqliteResult::IoerrShmMap,
        true,
        false,
        true,
        BASE_RETAINED,
        0,
        0,
    ),
    case(
        "map.view_map.after_uncertain",
        A2b1Path::ShmMap,
        map_operation(Phase::ViewMap),
        FaultTiming::AfterSuccess,
        Phase::ViewMap,
        Class::OutcomeUncertainPoisoned,
        SqliteResult::IoerrShmMap,
        true,
        false,
        true,
        MAPPED_VIEW,
        1,
        1,
    ),
    case(
        "map.observe.not_present",
        A2b1Path::ShmMap,
        REGION_ZERO_OBSERVE,
        FaultTiming::Native,
        Phase::FileSize,
        Class::NotPresent,
        SqliteResult::Ok,
        false,
        false,
        false,
        BASE_RETAINED,
        1,
        1,
    ),
];

const fn clean_before(id: &'static str, phase: Phase) -> Case {
    case(
        id,
        A2b1Path::ShmMap,
        map_operation(phase),
        FaultTiming::BeforeCall,
        phase,
        Class::IoBeforeMutation,
        SqliteResult::IoerrShmMap,
        false,
        false,
        false,
        BASE_RETAINED,
        0,
        0,
    )
}

const fn prior_mutation_before(id: &'static str, phase: Phase, retained: RetainedCustody) -> Case {
    case(
        id,
        A2b1Path::ShmMap,
        map_operation(phase),
        FaultTiming::BeforeCall,
        phase,
        Class::MutatedButKnown,
        SqliteResult::IoerrShmMap,
        true,
        false,
        true,
        retained,
        0,
        0,
    )
}

const fn known_after(id: &'static str, phase: Phase, retained: RetainedCustody) -> Case {
    case(
        id,
        A2b1Path::ShmMap,
        map_operation(phase),
        FaultTiming::AfterSuccess,
        phase,
        Class::MutatedButKnown,
        SqliteResult::IoerrShmMap,
        true,
        false,
        true,
        retained,
        1,
        1,
    )
}

const fn uncertain_after(id: &'static str, phase: Phase, retained: RetainedCustody) -> Case {
    case(
        id,
        A2b1Path::ShmMap,
        map_operation(phase),
        FaultTiming::AfterSuccess,
        phase,
        Class::OutcomeUncertainPoisoned,
        SqliteResult::IoerrShmMap,
        true,
        true,
        true,
        retained,
        1,
        1,
    )
}

const fn map_operation(phase: Phase) -> OperationShape {
    map_at_dms(match phase {
        Phase::ExactSiblingOpen => Dms::Absent,
        Phase::DmsExclusiveAcquire | Phase::DmsSharedAcquire => Dms::Released,
        Phase::DmsTruncate | Phase::DmsExclusiveRelease => Dms::ExclusiveKnown,
        _ => Dms::Shared,
    })
}
