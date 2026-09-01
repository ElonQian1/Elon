//! Test constructors kept out of the production-shaped child identity parser.

use super::{
    payload_commitment, registration_commitment, validate_actual_payload, ReportedChildIdentity,
    RootCommitment, ValidatedChildProcessReceipt,
};

pub(super) fn validate_payload_for_test(payload: &str) -> Result<(), &'static str> {
    validate_actual_payload(payload).map(|_| ())
}

pub(super) fn validated_receipt_for_record_test(
    payload: &str,
    registration_id: u64,
) -> Result<ValidatedChildProcessReceipt, &'static str> {
    if registration_id == 0 {
        return Err("A2_DYNAMIC_REGISTRATION_ID_INVALID");
    }
    let family = validate_actual_payload(payload)?;
    let identity = ReportedChildIdentity {
        process_id: std::process::id(),
        nonce: "0123456789abcdef0123456789abcdef".to_owned(),
    };
    let root_commitment = RootCommitment([0x5a; 32]);
    let payload_commitment = payload_commitment(payload);
    let registration_commitment = registration_commitment(
        identity.process_id,
        &identity.nonce,
        &root_commitment,
        &payload_commitment,
        family,
        registration_id,
    );
    Ok(ValidatedChildProcessReceipt {
        identity,
        root_commitment,
        registration_commitment,
        actual_payload: payload.to_owned(),
        payload_commitment,
        family,
        exit_code: 0,
    })
}
