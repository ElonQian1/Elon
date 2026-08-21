mod actual;
mod record;
#[cfg(test)]
mod tests;
mod validate;

pub(in super::super) use actual::{
    RegistrationShutdownActual, RegistrationShutdownActualCounts,
    RegistrationShutdownActualCustody, RegistrationShutdownActualIdentity,
    RegistrationShutdownActualTarget, RegistrationShutdownActualTopology,
    RegistrationShutdownDmsCustody, RegistrationShutdownFailureClass,
    RegistrationShutdownLogicalRoutePhase, RegistrationShutdownPhase,
    RegistrationShutdownRegistrationPhase, RegistrationShutdownRegistryRoutePhase,
    RegistrationShutdownSelector, RegistrationShutdownTiming,
};
pub(in super::super) use record::{
    ValidatedRegistrationShutdownObservation, ValidatedRegistrationShutdownReportPayload,
};
pub(in super::super) use validate::validate_registration_shutdown_report_payload;
