use std::fmt::Write as _;

use super::{
    super::case_key::CaseKey,
    actual::{
        RegistrationShutdownActual, RegistrationShutdownActualCounts,
        RegistrationShutdownActualCustody, RegistrationShutdownActualIdentity,
        RegistrationShutdownActualTarget, RegistrationShutdownActualTopology,
        RegistrationShutdownDmsCustody, RegistrationShutdownFailureClass,
        RegistrationShutdownLogicalRoutePhase, RegistrationShutdownPhase,
        RegistrationShutdownRegistrationPhase, RegistrationShutdownRegistryRoutePhase,
        RegistrationShutdownSelector, RegistrationShutdownTiming,
    },
};

const REPORT_VERSION: &str = "a2b2rs1";
const MAX_REPORT_PAYLOAD_BYTES: usize = 1_024;

impl RegistrationShutdownActual {
    pub(in super::super::super) fn to_report_payload(&self) -> String {
        let identity = self.identity;
        let target = identity.target;
        let retained = self.retained;
        let counts = self.counts;
        let values: [u64; 81] = [
            u64::from(identity.path_is_registration_shutdown),
            u64::from(identity.topology_is_registration_only),
            u64::from(identity.unmap_is_not_applicable),
            u64::from(identity.node_is_not_applicable),
            u64::from(identity.variant),
            u64::from(identity.pre_shared_mask),
            u64::from(identity.pre_exclusive_mask),
            u64::from(identity.phase as u8),
            u64::from(identity.cause_phase_is_none),
            u64::from(identity.timing as u8),
            u64::from(identity.class as u8),
            u64::from(target.scope_is_registration),
            target.registration_id,
            u64::from(target.route_ordinal_is_not_applicable),
            u64::from(target.runtime_generation_is_not_applicable),
            u64::from(target.shm_connection_id_is_not_applicable),
            u64::from(target.role_is_none),
            u64::from(target.callback_is_none),
            u64::from(target.occurrence),
            u64::from(identity.sqlite_outcome_is_not_applicable),
            u64::from(self.mutation_may_have_occurred),
            u64::from(self.lock_outcome_uncertain),
            u64::from(self.domain_terminal),
            u64::from(self.registry_route_phase as u8),
            u64::from(self.logical_route_phase as u8),
            u64::from(self.registration_phase as u8),
            u64::from(self.later_callback_allowed),
            u64::from(self.pre.sqlite_connections),
            u64::from(self.pre.shm_connections),
            u64::from(self.pre.registry_routes),
            u64::from(self.pre.logical_names),
            u64::from(self.post.sqlite_connections),
            u64::from(self.post.shm_connections),
            u64::from(self.post.registry_routes),
            u64::from(self.post.logical_names),
            u64::from(retained.node),
            u64::from(retained.views),
            u64::from(retained.mappings),
            u64::from(retained.dms as u8),
            u64::from(retained.shm_file),
            u64::from(retained.main_file),
            u64::from(retained.main_lock_owner),
            u64::from(retained.main_lease),
            u64::from(retained.shm_lease),
            u64::from(retained.callback_leases),
            u64::from(retained.registry_entry),
            u64::from(retained.logical_names),
            u64::from(retained.vfs_table),
            u64::from(retained.vfs_name),
            u64::from(retained.vfs_context),
            u64::from(retained.root_deletable),
            u64::from(counts.raw_state_take_attempt),
            u64::from(counts.raw_state_take_success),
            u64::from(counts.raw_state_abandon),
            u64::from(counts.methods_clear),
            u64::from(counts.callback_begin),
            u64::from(counts.callback_complete_attempt),
            u64::from(counts.callback_complete_success),
            u64::from(counts.selected_action_attempt),
            u64::from(counts.selected_action_success),
            u64::from(counts.shm_detach),
            u64::from(counts.main_unlock_attempt),
            u64::from(counts.main_unlock_success),
            u64::from(counts.main_file_close_attempt),
            u64::from(counts.main_file_close_success),
            u64::from(counts.registry_close_attempt),
            u64::from(counts.registry_close_success),
            u64::from(counts.connection_observe_attempt),
            u64::from(counts.connection_observe_success),
            u64::from(counts.registry_route_remove_attempt),
            u64::from(counts.registry_route_remove_success),
            u64::from(counts.logical_names_remove_attempt),
            u64::from(counts.logical_names_remove_success),
            u64::from(counts.logical_names_remove),
            u64::from(counts.vfs_unregister_attempt),
            u64::from(counts.vfs_unregister_success),
            u64::from(counts.fault_observe),
            u64::from(counts.fault_trigger),
            u64::from(counts.fault_pending),
            u64::from(counts.custody_retain),
            u64::from(counts.physical_retry),
        ];
        let mut report = String::with_capacity(256);
        write!(
            &mut report,
            "{REPORT_VERSION},{}",
            self.selector.report_name()
        )
        .expect("writing to String cannot fail");
        for value in values {
            write!(&mut report, ",{value}").expect("writing to String cannot fail");
        }
        report
    }

    pub(super) fn from_report_payload(input: &str) -> Result<Self, &'static str> {
        if input.len() > MAX_REPORT_PAYLOAD_BYTES || !input.is_ascii() {
            return Err("RegistrationShutdown report payload is not bounded ASCII");
        }
        let mut parts = input.split(',');
        if parts.next() != Some(REPORT_VERSION) {
            return Err("RegistrationShutdown report payload version is unsupported");
        }
        let selector = RegistrationShutdownSelector::from_report_name(
            parts
                .next()
                .ok_or("RegistrationShutdown report selector is missing")?,
        )
        .ok_or("RegistrationShutdown report selector is unknown")?;
        let mut fields = ReportFields { parts };
        let identity = RegistrationShutdownActualIdentity {
            path_is_registration_shutdown: fields.boolean()?,
            topology_is_registration_only: fields.boolean()?,
            unmap_is_not_applicable: fields.boolean()?,
            node_is_not_applicable: fields.boolean()?,
            variant: fields.u8()?,
            pre_shared_mask: fields.u8()?,
            pre_exclusive_mask: fields.u8()?,
            phase: fields.phase()?,
            cause_phase_is_none: fields.boolean()?,
            timing: fields.timing()?,
            class: fields.failure_class()?,
            target: RegistrationShutdownActualTarget {
                scope_is_registration: fields.boolean()?,
                registration_id: fields.u64()?,
                route_ordinal_is_not_applicable: fields.boolean()?,
                runtime_generation_is_not_applicable: fields.boolean()?,
                shm_connection_id_is_not_applicable: fields.boolean()?,
                role_is_none: fields.boolean()?,
                callback_is_none: fields.boolean()?,
                occurrence: fields.u32()?,
            },
            sqlite_outcome_is_not_applicable: fields.boolean()?,
        };
        let actual = Self {
            selector,
            identity,
            mutation_may_have_occurred: fields.boolean()?,
            lock_outcome_uncertain: fields.boolean()?,
            domain_terminal: fields.boolean()?,
            registry_route_phase: fields.registry_route_phase()?,
            logical_route_phase: fields.logical_route_phase()?,
            registration_phase: fields.registration_phase()?,
            later_callback_allowed: fields.boolean()?,
            pre: fields.topology()?,
            post: fields.topology()?,
            retained: fields.custody()?,
            counts: fields.counts()?,
        };
        if fields.parts.next().is_some() || actual.to_report_payload() != input {
            return Err("RegistrationShutdown report payload is not canonical");
        }
        Ok(actual)
    }
}

struct ReportFields<'a> {
    parts: std::str::Split<'a, char>,
}

impl ReportFields<'_> {
    fn u64(&mut self) -> Result<u64, &'static str> {
        self.parts
            .next()
            .ok_or("RegistrationShutdown report payload is truncated")?
            .parse()
            .map_err(|_| "RegistrationShutdown report field is not an unsigned integer")
    }

    fn u32(&mut self) -> Result<u32, &'static str> {
        self.u64()?
            .try_into()
            .map_err(|_| "RegistrationShutdown report field exceeds u32")
    }

    fn u8(&mut self) -> Result<u8, &'static str> {
        self.u64()?
            .try_into()
            .map_err(|_| "RegistrationShutdown report field exceeds u8")
    }

    fn boolean(&mut self) -> Result<bool, &'static str> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("RegistrationShutdown report boolean is not zero or one"),
        }
    }

    fn phase(&mut self) -> Result<RegistrationShutdownPhase, &'static str> {
        match self.u8()? {
            0 => Ok(RegistrationShutdownPhase::OutstandingCallbackGate),
            1 => Ok(RegistrationShutdownPhase::LiveRouteGate),
            2 => Ok(RegistrationShutdownPhase::QuarantinedCustodyGate),
            3 => Ok(RegistrationShutdownPhase::RouteIndexObservation),
            4 => Ok(RegistrationShutdownPhase::VfsUnregister),
            5 => Ok(RegistrationShutdownPhase::Success),
            _ => Err("RegistrationShutdown report phase is unknown"),
        }
    }

    fn timing(&mut self) -> Result<RegistrationShutdownTiming, &'static str> {
        match self.u8()? {
            0 => Ok(RegistrationShutdownTiming::Validation),
            1 => Ok(RegistrationShutdownTiming::BeforeCall),
            2 => Ok(RegistrationShutdownTiming::NativeRetryable),
            3 => Ok(RegistrationShutdownTiming::NativeUncertain),
            4 => Ok(RegistrationShutdownTiming::AfterSuccessKnown),
            5 => Ok(RegistrationShutdownTiming::Success),
            _ => Err("RegistrationShutdown report timing is unknown"),
        }
    }

    fn failure_class(&mut self) -> Result<RegistrationShutdownFailureClass, &'static str> {
        match self.u8()? {
            0 => Ok(RegistrationShutdownFailureClass::None),
            1 => Ok(RegistrationShutdownFailureClass::RegistrationRetained),
            _ => Err("RegistrationShutdown report failure class is unknown"),
        }
    }

    fn registry_route_phase(
        &mut self,
    ) -> Result<RegistrationShutdownRegistryRoutePhase, &'static str> {
        match self.u8()? {
            0 => Ok(RegistrationShutdownRegistryRoutePhase::Active),
            1 => Ok(RegistrationShutdownRegistryRoutePhase::Closing),
            2 => Ok(RegistrationShutdownRegistryRoutePhase::AwaitingRetirement),
            3 => Ok(RegistrationShutdownRegistryRoutePhase::Removed),
            4 => Ok(RegistrationShutdownRegistryRoutePhase::TerminalQuarantine),
            _ => Err("RegistrationShutdown report registry route phase is unknown"),
        }
    }

    fn logical_route_phase(
        &mut self,
    ) -> Result<RegistrationShutdownLogicalRoutePhase, &'static str> {
        match self.u8()? {
            0 => Ok(RegistrationShutdownLogicalRoutePhase::Indexed),
            1 => Ok(RegistrationShutdownLogicalRoutePhase::Removed),
            2 => Ok(RegistrationShutdownLogicalRoutePhase::Retained),
            _ => Err("RegistrationShutdown report logical route phase is unknown"),
        }
    }

    fn registration_phase(
        &mut self,
    ) -> Result<RegistrationShutdownRegistrationPhase, &'static str> {
        match self.u8()? {
            0 => Ok(RegistrationShutdownRegistrationPhase::Registered),
            1 => Ok(RegistrationShutdownRegistrationPhase::Unregistered),
            2 => Ok(RegistrationShutdownRegistrationPhase::RetainedRegistered),
            3 => Ok(RegistrationShutdownRegistrationPhase::RetainedAfterUnregister),
            _ => Err("RegistrationShutdown report registration phase is unknown"),
        }
    }

    fn dms(&mut self) -> Result<RegistrationShutdownDmsCustody, &'static str> {
        match self.u8()? {
            0 => Ok(RegistrationShutdownDmsCustody::Absent),
            1 => Ok(RegistrationShutdownDmsCustody::Shared),
            2 => Ok(RegistrationShutdownDmsCustody::Released),
            3 => Ok(RegistrationShutdownDmsCustody::OutcomeUncertain),
            _ => Err("RegistrationShutdown report DMS custody is unknown"),
        }
    }

    fn topology(&mut self) -> Result<RegistrationShutdownActualTopology, &'static str> {
        Ok(RegistrationShutdownActualTopology {
            sqlite_connections: self.u8()?,
            shm_connections: self.u8()?,
            registry_routes: self.u8()?,
            logical_names: self.u8()?,
        })
    }

    fn custody(&mut self) -> Result<RegistrationShutdownActualCustody, &'static str> {
        Ok(RegistrationShutdownActualCustody {
            node: self.boolean()?,
            views: self.u8()?,
            mappings: self.u8()?,
            dms: self.dms()?,
            shm_file: self.boolean()?,
            main_file: self.boolean()?,
            main_lock_owner: self.boolean()?,
            main_lease: self.boolean()?,
            shm_lease: self.boolean()?,
            callback_leases: self.u8()?,
            registry_entry: self.boolean()?,
            logical_names: self.u8()?,
            vfs_table: self.boolean()?,
            vfs_name: self.boolean()?,
            vfs_context: self.boolean()?,
            root_deletable: self.boolean()?,
        })
    }

    fn counts(&mut self) -> Result<RegistrationShutdownActualCounts, &'static str> {
        Ok(RegistrationShutdownActualCounts {
            raw_state_take_attempt: self.u8()?,
            raw_state_take_success: self.u8()?,
            raw_state_abandon: self.u8()?,
            methods_clear: self.u8()?,
            callback_begin: self.u8()?,
            callback_complete_attempt: self.u8()?,
            callback_complete_success: self.u8()?,
            selected_action_attempt: self.u8()?,
            selected_action_success: self.u8()?,
            shm_detach: self.u8()?,
            main_unlock_attempt: self.u8()?,
            main_unlock_success: self.u8()?,
            main_file_close_attempt: self.u8()?,
            main_file_close_success: self.u8()?,
            registry_close_attempt: self.u8()?,
            registry_close_success: self.u8()?,
            connection_observe_attempt: self.u8()?,
            connection_observe_success: self.u8()?,
            registry_route_remove_attempt: self.u8()?,
            registry_route_remove_success: self.u8()?,
            logical_names_remove_attempt: self.u8()?,
            logical_names_remove_success: self.u8()?,
            logical_names_remove: self.u8()?,
            vfs_unregister_attempt: self.u8()?,
            vfs_unregister_success: self.u8()?,
            fault_observe: self.u8()?,
            fault_trigger: self.u8()?,
            fault_pending: self.u8()?,
            custody_retain: self.u8()?,
            physical_retry: self.u8()?,
        })
    }
}

#[derive(PartialEq, Eq)]
#[must_use = "the validated canonical payload must be cross-bound to the child receipt"]
pub(in super::super::super) struct ValidatedRegistrationShutdownReportPayload {
    exact_payload: String,
}

impl std::fmt::Debug for ValidatedRegistrationShutdownReportPayload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ValidatedRegistrationShutdownReportPayload(<sealed>)")
    }
}

impl ValidatedRegistrationShutdownReportPayload {
    pub(super) fn new(exact_payload: String) -> Self {
        Self { exact_payload }
    }

    pub(in super::super::super) fn matches_exact(&self, candidate: &str) -> bool {
        self.exact_payload == candidate
    }

    pub(in super::super::super) fn matches_commitment(
        &self,
        commitment: &super::super::super::a2_dynamic_evidence::SanitizedActualPayloadCommitment,
    ) -> bool {
        commitment.matches_payload(&self.exact_payload)
    }

    pub(in super::super::super) fn exact_payload(&self) -> &str {
        &self.exact_payload
    }
}

#[derive(Debug, PartialEq, Eq)]
#[must_use = "a validated RegistrationShutdown observation must be consumed by the evidence gate"]
pub(in super::super::super) struct ValidatedRegistrationShutdownObservation {
    selector: RegistrationShutdownSelector,
    case_key: CaseKey,
    report_payload: ValidatedRegistrationShutdownReportPayload,
}

impl ValidatedRegistrationShutdownObservation {
    pub(super) fn new(
        selector: RegistrationShutdownSelector,
        case_key: CaseKey,
        exact_payload: String,
    ) -> Self {
        Self {
            selector,
            case_key,
            report_payload: ValidatedRegistrationShutdownReportPayload::new(exact_payload),
        }
    }

    pub(in super::super::super) const fn selector(&self) -> RegistrationShutdownSelector {
        self.selector
    }

    pub(in super::super::super) const fn registration_id(&self) -> u64 {
        self.case_key.registration_id
    }

    pub(in super::super::super) fn into_evidence_parts(
        self,
    ) -> (CaseKey, ValidatedRegistrationShutdownReportPayload) {
        (self.case_key, self.report_payload)
    }
}
