//! Reviewed successor prefix for Map ABI validation and raw-state admission.
//!
//! This inventory deliberately stops at the typed Map operation and at fallback custody/route
//! projection. It is not the complete Map `SourceBranch`, `Expected`, terminal universe,
//! `CaseKey`, static denominator, or dynamic-observation inventory.
//! Normal return and caught unwind are both recorded beyond the typed-operation frontier and have
//! no incident edge in this prefix. A caught-unwind association with abandonment remains an exact
//! tagged source fact, not a reviewed temporal chain through the unresolved operation interior.
//! A non-null file is also a safety premise: it must be live, aligned, initialized and serialized;
//! exact methods plus non-null state must identify this module's live envelope. Forged or dangling
//! pointers are UB, not `TypeMismatch` or caught-unwind cases.

mod model;
mod prefix;

pub(super) use model::{
    AbiInputCell, AbiNullWriteOutcome, AbiOutputSlotShape, AbiScalarInvalidityShape,
    RawAbandonCause, RawAbandonCauseDisposition, RawAbandonOutcome, RawAbandonOutcomeRecord,
    RawCustodyRetention, RawSlotRetention, RawStateCase, RawStateOutcome, RawStateOutcomeRecord,
    RawStateTraceDisposition, ReviewedFrontierIngress, ReviewedOpenFrontier,
    ReviewedOpenFrontierRecord, ReviewedSuccessorEdge, ReviewedTerminal, ReviewedTraceCondition,
    ReviewedTraceEndpoint, ReviewedTraceRelation, RAW_ABANDON_OUTCOMES, RAW_STATE_OUTCOMES,
};
pub(super) use prefix::{
    ABI_INPUT_CELLS, ABI_OUTPUT_SLOT_SHAPES, ABI_SCALAR_INVALIDITY_SHAPES, OPEN_FRONTIERS,
    OPEN_FRONTIER_RECORDS, SUCCESSOR_EDGES, TERMINALS,
};
