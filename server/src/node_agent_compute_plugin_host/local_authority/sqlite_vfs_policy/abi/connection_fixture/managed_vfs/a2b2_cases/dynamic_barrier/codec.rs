use std::fmt::Write as _;

use super::actual::*;

const REPORT_VERSION: &str = "a2b2br1";
const REPORT_FIELD_COUNT: usize = 81;
const MAX_REPORT_PAYLOAD_BYTES: usize = 1_024;

impl BarrierActual {
    pub(in super::super::super) fn to_report_payload(&self) -> String {
        let identity = self.identity;
        let target = identity.target;
        let retained = self.retained;
        let counts = self.counts;
        let values: [u64; REPORT_FIELD_COUNT] = [
            u64::from(identity.path_is_barrier),
            u64::from(identity.topology_is_shared_non_final),
            u64::from(identity.unmap_is_not_applicable),
            u64::from(identity.node_is_live),
            u64::from(identity.variant),
            u64::from(identity.pre_shared_mask),
            u64::from(identity.pre_exclusive_mask),
            u64::from(identity.phase as u8),
            u64::from(identity.cause_phase_is_none),
            u64::from(identity.timing as u8),
            u64::from(identity.class as u8),
            u64::from(target.scope_is_route_main),
            target.registration_id,
            target.route_ordinal,
            target.runtime_generation,
            target.shm_connection_id,
            u64::from(target.role_is_main),
            u64::from(target.callback_is_shm),
            u64::from(target.occurrence),
            u64::from(identity.sqlite_outcome_is_void_no_result_code),
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
            return Err("Barrier report payload is not bounded ASCII");
        }
        let mut parts = input.split(',');
        if parts.next() != Some(REPORT_VERSION) {
            return Err("Barrier report payload version is unsupported");
        }
        let selector = BarrierSelector::from_report_name(
            parts
                .next()
                .ok_or("Barrier report payload has no selector")?,
        )
        .ok_or("Barrier report payload selector is unsupported")?;
        let mut fields = Fields { parts: &mut parts };
        let actual = Self {
            selector,
            identity: BarrierActualIdentity {
                path_is_barrier: fields.boolean()?,
                topology_is_shared_non_final: fields.boolean()?,
                unmap_is_not_applicable: fields.boolean()?,
                node_is_live: fields.boolean()?,
                variant: fields.u8()?,
                pre_shared_mask: fields.u8()?,
                pre_exclusive_mask: fields.u8()?,
                phase: BarrierPhase::try_from(fields.u64()?)?,
                cause_phase_is_none: fields.boolean()?,
                timing: BarrierTiming::try_from(fields.u64()?)?,
                class: BarrierFailureClass::try_from(fields.u64()?)?,
                target: BarrierActualTarget {
                    scope_is_route_main: fields.boolean()?,
                    registration_id: fields.u64()?,
                    route_ordinal: fields.u64()?,
                    runtime_generation: fields.u64()?,
                    shm_connection_id: fields.u64()?,
                    role_is_main: fields.boolean()?,
                    callback_is_shm: fields.boolean()?,
                    occurrence: fields.u32()?,
                },
                sqlite_outcome_is_void_no_result_code: fields.boolean()?,
            },
            mutation_may_have_occurred: fields.boolean()?,
            lock_outcome_uncertain: fields.boolean()?,
            domain_terminal: fields.boolean()?,
            registry_route_phase: BarrierRegistryRoutePhase::try_from(fields.u64()?)?,
            logical_route_phase: BarrierLogicalRoutePhase::try_from(fields.u64()?)?,
            registration_phase: BarrierRegistrationPhase::try_from(fields.u64()?)?,
            later_callback_allowed: fields.boolean()?,
            pre: fields.topology()?,
            post: fields.topology()?,
            retained: fields.custody()?,
            counts: fields.counts()?,
        };
        if fields.parts.next().is_some() || actual.to_report_payload() != input {
            return Err("Barrier report payload is not canonical");
        }
        Ok(actual)
    }
}

struct Fields<'a, 'b> {
    parts: &'a mut std::str::Split<'b, char>,
}

impl Fields<'_, '_> {
    fn u64(&mut self) -> Result<u64, &'static str> {
        let value = self
            .parts
            .next()
            .ok_or("Barrier report payload is truncated")?;
        if value.is_empty()
            || !value.bytes().all(|byte| byte.is_ascii_digit())
            || (value.len() > 1 && value.starts_with('0'))
        {
            return Err("Barrier report field is not canonical unsigned decimal");
        }
        value
            .parse()
            .map_err(|_| "Barrier report field exceeds u64")
    }

    fn boolean(&mut self) -> Result<bool, &'static str> {
        match self.u64()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("Barrier report boolean is outside 0..=1"),
        }
    }

    fn u8(&mut self) -> Result<u8, &'static str> {
        self.u64()
            .and_then(|value| u8::try_from(value).map_err(|_| "Barrier report field exceeds u8"))
    }

    fn u32(&mut self) -> Result<u32, &'static str> {
        self.u64()
            .and_then(|value| u32::try_from(value).map_err(|_| "Barrier report field exceeds u32"))
    }

    fn topology(&mut self) -> Result<BarrierActualTopology, &'static str> {
        Ok(BarrierActualTopology {
            sqlite_connections: self.u8()?,
            shm_connections: self.u8()?,
            registry_routes: self.u8()?,
            logical_names: self.u8()?,
        })
    }

    fn custody(&mut self) -> Result<BarrierActualCustody, &'static str> {
        Ok(BarrierActualCustody {
            node: self.boolean()?,
            views: self.u8()?,
            mappings: self.u8()?,
            dms: BarrierDmsCustody::try_from(self.u64()?)?,
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

    fn counts(&mut self) -> Result<BarrierActualCounts, &'static str> {
        Ok(BarrierActualCounts {
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
