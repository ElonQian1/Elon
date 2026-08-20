//! Active-subject boundary for V277 genesis and durable refresh no-work observations.
//!
//! Planned genesis reuses the registering producer while binding its owned projected target before
//! and after I/O. Durable-active broker/secret-delivery preparation remains fail-closed until the
//! layer-two carrier gains an exact transaction-free preparation seam.

mod preflight;
mod reproof;
mod types;

pub(in crate::store) use preflight::planned_external_pool_adapter_active_no_work_probe_subject_on;
pub(in crate::store) use reproof::{
    with_reproved_planned_external_pool_adapter_active_no_work_subject,
    ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject,
};
pub(in crate::store) use types::PlannedExternalPoolAdapterActiveNoWorkProbeSubject;
