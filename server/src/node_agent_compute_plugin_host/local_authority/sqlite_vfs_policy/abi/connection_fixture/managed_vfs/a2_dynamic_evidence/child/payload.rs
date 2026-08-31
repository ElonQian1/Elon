use super::super::super::a2b2_cases::{
    BarrierSelector, JointCloseSelector, RegistrationShutdownSelector, RegistryLifecycleSelector,
    UnmapSelector,
};

use super::{
    lock_lifecycle, lock_native_acquire_busy, lock_request_validation, lock_stored_poison,
    map_lifecycle, map_region_loop, SanitizedPayloadFamily,
};

const MAX_ACTUAL_PAYLOAD_BYTES: usize = 2_048;
const COMMON_REPORT_VALUE_COUNT: usize = 81;
const JOINT_CLOSE_REPORT_VALUE_COUNT: usize = 83;
const MAP_QUOTIENT_REPORT_VALUE_COUNT: usize = 67;
const REGISTRATION_REPORT_VERSION: &str = "a2b2rs1";
const BARRIER_REPORT_VERSION: &str = "a2b2br1";
const REGISTRY_LIFECYCLE_REPORT_VERSION: &str = "a2b2rl1";
const UNMAP_REPORT_VERSION: &str = "a2b2un1";
const JOINT_CLOSE_REPORT_VERSION: &str = "a2b2jc1";
const MAP_QUOTIENT_REPORT_VERSION: &str = "a2mapq2";

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
    let (family, expected_value_count) = match version {
        REGISTRATION_REPORT_VERSION => {
            RegistrationShutdownSelector::from_report_name(selector)
                .ok_or("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")?;
            (
                SanitizedPayloadFamily::RegistrationShutdown,
                COMMON_REPORT_VALUE_COUNT,
            )
        }
        BARRIER_REPORT_VERSION => {
            BarrierSelector::from_report_name(selector)
                .ok_or("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")?;
            (SanitizedPayloadFamily::Barrier, COMMON_REPORT_VALUE_COUNT)
        }
        REGISTRY_LIFECYCLE_REPORT_VERSION => {
            RegistryLifecycleSelector::from_report_name(selector)
                .ok_or("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")?;
            (
                SanitizedPayloadFamily::RegistryLifecycle,
                COMMON_REPORT_VALUE_COUNT,
            )
        }
        UNMAP_REPORT_VERSION => {
            UnmapSelector::from_report_name(selector)
                .ok_or("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")?;
            (SanitizedPayloadFamily::Unmap, COMMON_REPORT_VALUE_COUNT)
        }
        JOINT_CLOSE_REPORT_VERSION => {
            JointCloseSelector::from_report_name(selector)
                .ok_or("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")?;
            (
                SanitizedPayloadFamily::JointClose,
                JOINT_CLOSE_REPORT_VALUE_COUNT,
            )
        }
        MAP_QUOTIENT_REPORT_VERSION => {
            if !matches!(
                selector,
                "region-size-budget-completed"
                    | "region-count-budget-completed"
                    | "logical-size-budget-completed"
            ) {
                return Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID");
            }
            (
                SanitizedPayloadFamily::MapQuotient,
                MAP_QUOTIENT_REPORT_VALUE_COUNT,
            )
        }
        map_lifecycle::REPORT_VERSION => (
            SanitizedPayloadFamily::MapQuotient,
            map_lifecycle::classify_header(version, selector)?
                .ok_or("A2_DYNAMIC_CHILD_ACTUAL_VERSION_INVALID")?,
        ),
        map_region_loop::REPORT_VERSION => (
            SanitizedPayloadFamily::MapQuotient,
            map_region_loop::classify_header(version, selector)?
                .ok_or("A2_DYNAMIC_CHILD_ACTUAL_VERSION_INVALID")?,
        ),
        lock_lifecycle::REPORT_VERSION => (
            SanitizedPayloadFamily::LockQuotient,
            lock_lifecycle::classify_header(version, selector)?
                .ok_or("A2_DYNAMIC_CHILD_ACTUAL_VERSION_INVALID")?,
        ),
        lock_native_acquire_busy::REPORT_VERSION => (
            SanitizedPayloadFamily::LockQuotient,
            lock_native_acquire_busy::classify_header(version, selector)?
                .ok_or("A2_DYNAMIC_CHILD_ACTUAL_VERSION_INVALID")?,
        ),
        lock_stored_poison::REPORT_VERSION => (
            SanitizedPayloadFamily::LockQuotient,
            lock_stored_poison::classify_header(version, selector)?
                .ok_or("A2_DYNAMIC_CHILD_ACTUAL_VERSION_INVALID")?,
        ),
        lock_stored_poison::route_unknown::REPORT_VERSION => (
            SanitizedPayloadFamily::LockQuotient,
            lock_stored_poison::route_unknown::classify_header(version, selector)?
                .ok_or("A2_DYNAMIC_CHILD_ACTUAL_VERSION_INVALID")?,
        ),
        lock_request_validation::REPORT_VERSION => (
            SanitizedPayloadFamily::LockQuotient,
            lock_request_validation::classify_header(version, selector)?
                .ok_or("A2_DYNAMIC_CHILD_ACTUAL_VERSION_INVALID")?,
        ),
        _ => return Err("A2_DYNAMIC_CHILD_ACTUAL_VERSION_INVALID"),
    };
    let values = fields.collect::<Vec<_>>();
    if values.len() != expected_value_count || values.iter().any(|value| !canonical_u64(value)) {
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
