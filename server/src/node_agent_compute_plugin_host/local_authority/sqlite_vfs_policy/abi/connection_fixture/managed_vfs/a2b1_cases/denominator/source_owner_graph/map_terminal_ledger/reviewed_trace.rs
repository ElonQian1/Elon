//! Reviewed successor prefix for Map ABI validation and raw-state admission.
//!
//! The canonical denominator-facing ABI fragment is a separate 15-terminal/one-continuation local
//! quotient. Its continuation is `AbiRawDispatch`, not the later typed-operation open frontier.
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

pub(super) use super::super::super::abi_map_fragment::{
    AbiNullWriteOutcome, AbiOutputSlotShape, AbiScalarInvalidityShape,
    ReviewedMapAbiDecisionFragment, ReviewedMapAbiDispositionFragment,
    ReviewedMapAbiDownstreamFragment, ReviewedMapAbiExitFragment, ReviewedMapAbiTerminalFragment,
    ABI_OUTPUT_SLOT_SHAPES, ABI_SCALAR_INVALIDITY_SHAPES, REVIEWED_MAP_ABI_FRAGMENTS,
};
pub(super) use super::super::super::raw_state_fragment::{RawAbandonOutcome, RawSlotRetention};
pub(super) use model::{
    AbiInputCell, RawAbandonCause, RawAbandonCauseDisposition, RawAbandonOutcomeRecord,
    RawStateCase, RawStateOutcome, RawStateOutcomeRecord, RawStateTraceDisposition,
    ReviewedFrontierIngress, ReviewedOpenFrontier, ReviewedOpenFrontierRecord,
    ReviewedSuccessorEdge, ReviewedTerminal, ReviewedTraceCondition, ReviewedTraceEndpoint,
    ReviewedTraceRelation, RAW_ABANDON_OUTCOMES, RAW_STATE_OUTCOMES,
};
pub(super) use prefix::{
    ABI_INPUT_CELLS, OPEN_FRONTIERS, OPEN_FRONTIER_RECORDS, SUCCESSOR_EDGES, TERMINALS,
};
