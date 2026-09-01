//! Canonical 32-slot receipt vectors for the eleven q11 controlled cases.

use rusqlite::ffi;

use super::model::{
    ActiveObservation, HandleBoundSqliteAbiRawLockEvidenceV1,
    HandleBoundSqliteAbiRawLockRejectionCaseV1, RawValidation, RunCodeOutcome, AbandonOutcome,
};

pub(super) fn ordered_values(active: &ActiveObservation) -> Result<[u64; 32], &'static str> {
    Ok([
        1,
        active.case_v1.tag(),
        HandleBoundSqliteAbiRawLockEvidenceV1::ControlledFaultActual.tag(),
        active.observation_id,
        active.counts.fixture_prepare,
        u64::from(active.invocation_file_address == 0),
        active.slots_before,
        active
            .slots_prepared
            .ok_or("raw Lock rejection prepared-slot snapshot was missing")?,
        active.counts.entry,
        active.counts.scalar_admitted,
        active.counts.raw_validation,
        active.validation.map(RawValidation::tag).unwrap_or(0),
        active.counts.type_check,
        u64::from(active.type_matches.unwrap_or(false)),
        active.counts.payload_snapshot,
        u64::from(active.payload_present.unwrap_or(false)),
        active.counts.typed_operation_entry,
        active.counts.handle_file_missing,
        active.run_code_outcome.map(RunCodeOutcome::tag).unwrap_or(0),
        active.counts.abandon_entry,
        active.abandon_outcome.map(AbandonOutcome::tag).unwrap_or(0),
        active.counts.slots_clear,
        active.counts.envelope_drop,
        active.counts.payload_drop_attempt,
        active.counts.payload_drop_completed,
        active.counts.payload_drop_unwind,
        active.counts.abandon_drop_completed,
        active.counts.abandon_drop_unwind,
        active.counts.returned,
        active
            .result_code
            .ok_or("raw Lock rejection callback result was missing")? as u64,
        active
            .slots_after
            .ok_or("raw Lock rejection final-slot snapshot was missing")?,
        active
            .retained_fixture_tag
            .ok_or("raw Lock rejection retained-fixture tag was missing")?,
    ])
}

pub(super) fn validate_exact_values(
    case_v1: HandleBoundSqliteAbiRawLockRejectionCaseV1,
    values: [u64; 32],
) -> Result<(), &'static str> {
    if values[0] != 1
        || values[1] != case_v1.tag()
        || values[2] != 1
        || values[3] == 0
        || values[4] != 1
        || values[6] != 7
        || values[8] != 1
        || values[9] != 1
        || values[10] != 1
        || values[28] != 1
        || values[29] != ffi::SQLITE_IOERR_SHMLOCK as u64
    {
        return Err("raw Lock rejection common ledger vector was not exact");
    }
    let selected = [
        values[1], values[5], values[7], values[11], values[12], values[13], values[14],
        values[15], values[16], values[17], values[18], values[19], values[20], values[21],
        values[22], values[23], values[24], values[25], values[26], values[27], values[30],
        values[31],
    ];
    (selected == expected_case_values(case_v1))
        .then_some(())
        .ok_or("raw Lock rejection case ledger vector was not exact")
}

const fn expected_case_values(
    case_v1: HandleBoundSqliteAbiRawLockRejectionCaseV1,
) -> [u64; 22] {
    use HandleBoundSqliteAbiRawLockRejectionCaseV1 as Case;
    match case_v1 {
        Case::NullFileDirect => {
            [1, 1, 7, 1, 0, 0, 0, 0, 0, 0, 2, 1, 2, 0, 0, 0, 0, 0, 0, 0, 7, 0]
        }
        Case::UninstalledDirect => {
            [2, 0, 0, 2, 0, 0, 0, 0, 0, 0, 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 5]
        }
        Case::MethodsNullStatePresentDirect => {
            [3, 0, 2, 3, 0, 0, 0, 0, 0, 0, 2, 1, 3, 0, 0, 0, 0, 0, 0, 0, 2, 4]
        }
        Case::ForeignMethodsStateNullDirect => {
            [4, 0, 1, 3, 0, 0, 0, 0, 0, 0, 2, 1, 3, 0, 0, 0, 0, 0, 0, 0, 1, 5]
        }
        Case::ForeignMethodsStatePresentDirect => {
            [5, 0, 3, 3, 0, 0, 0, 0, 0, 0, 2, 1, 3, 0, 0, 0, 0, 0, 0, 0, 3, 4]
        }
        Case::ExactMethodsStateNullDirect => {
            [6, 0, 5, 4, 0, 0, 0, 0, 0, 0, 2, 1, 4, 0, 0, 0, 0, 0, 0, 0, 5, 5]
        }
        Case::OtherTypePayloadMissingDropCompleted => {
            [7, 0, 7, 5, 1, 0, 1, 0, 0, 0, 2, 1, 5, 1, 1, 0, 0, 0, 1, 0, 0, 5]
        }
        Case::OtherTypePayloadPresentDropCompleted => {
            [8, 0, 7, 5, 1, 0, 1, 1, 0, 0, 2, 1, 5, 1, 1, 1, 1, 0, 1, 0, 0, 5]
        }
        Case::OtherTypePayloadPresentDropUnwindCaught => {
            [9, 0, 7, 5, 1, 0, 1, 1, 0, 0, 2, 1, 6, 1, 1, 1, 0, 1, 0, 1, 0, 5]
        }
        Case::ExpectedTypePayloadMissingDropCompleted => {
            [10, 0, 7, 6, 1, 1, 1, 0, 0, 0, 3, 1, 5, 1, 1, 0, 0, 0, 1, 0, 0, 5]
        }
        Case::HandleBoundFileMissingDirect => {
            [11, 0, 7, 6, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 6]
        }
    }
}
