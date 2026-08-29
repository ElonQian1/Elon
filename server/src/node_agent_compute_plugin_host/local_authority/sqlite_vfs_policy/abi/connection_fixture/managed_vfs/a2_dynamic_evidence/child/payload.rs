use super::super::super::a2b2_cases::{BarrierSelector, RegistrationShutdownSelector};

use super::SanitizedPayloadFamily;

const MAX_ACTUAL_PAYLOAD_BYTES: usize = 1_024;
const REPORT_VALUE_COUNT: usize = 81;
const REGISTRATION_REPORT_VERSION: &str = "a2b2rs1";
const BARRIER_REPORT_VERSION: &str = "a2b2br1";

pub(super) fn validate_actual_payload(
    payload: &str,
) -> Result<SanitizedPayloadFamily, &'static str> {
    if payload.is_empty() || payload.len() > MAX_ACTUAL_PAYLOAD_BYTES || !payload.is_ascii() {
        return Err("A2_DYNAMIC_CHILD_ACTUAL_SIZE_INVALID");
    }
    let mut fields = payload.split(',');
    let version = fields
        .next()
        .ok_or("A2_DYNAMIC_CHILD_ACTUAL_VERSION_INVALID")?;
    let selector = fields
        .next()
        .ok_or("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_MISSING")?;
    let family = match version {
        REGISTRATION_REPORT_VERSION => {
            RegistrationShutdownSelector::from_report_name(selector)
                .ok_or("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")?;
            SanitizedPayloadFamily::RegistrationShutdown
        }
        BARRIER_REPORT_VERSION => {
            BarrierSelector::from_report_name(selector)
                .ok_or("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")?;
            SanitizedPayloadFamily::Barrier
        }
        _ => return Err("A2_DYNAMIC_CHILD_ACTUAL_VERSION_INVALID"),
    };
    let values = fields.collect::<Vec<_>>();
    if values.len() != REPORT_VALUE_COUNT || values.iter().any(|value| !canonical_u64(value)) {
        return Err("A2_DYNAMIC_CHILD_ACTUAL_FIELDS_INVALID");
    }
    Ok(family)
}

fn canonical_u64(value: &str) -> bool {
    value
        .parse::<u64>()
        .map(|parsed| parsed.to_string() == value)
        .unwrap_or(false)
}
