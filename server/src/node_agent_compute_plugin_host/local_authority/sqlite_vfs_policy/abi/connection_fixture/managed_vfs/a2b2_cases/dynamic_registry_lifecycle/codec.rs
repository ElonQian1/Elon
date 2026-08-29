use std::fmt::Write as _;

use super::actual::*;

const REPORT_VERSION: &str = "a2b2rl1";
const REPORT_FIELD_COUNT: usize = 81;
const MAX_REPORT_PAYLOAD_BYTES: usize = 1_024;

impl RegistryLifecycleActual {
    pub(in super::super::super) fn to_report_payload(&self) -> String {
        let identity = self.identity;
        let target = identity.target;
        let retained = self.retained;
        let counts = self.counts;
        let values: [u64; REPORT_FIELD_COUNT] = [
            u64::from(identity.path_is_registry_lifecycle),
            u64::from(identity.topology_is_shared_non_final),
            u64::from(identity.unmap_is_keep),
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
            u64::from(target.callback_is_close),
            u64::from(target.occurrence),
            u64::from(identity.sqlite_outcome as u8),
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
            return Err("RegistryLifecycle report payload is not bounded ASCII");
        }
        let mut parts = input.split(',');
        if parts.next() != Some(REPORT_VERSION) {
            return Err("RegistryLifecycle report payload version is unsupported");
        }
        let selector = RegistryLifecycleSelector::from_report_name(
            parts
                .next()
                .ok_or("RegistryLifecycle report payload has no selector")?,
        )
        .ok_or("RegistryLifecycle report payload selector is unsupported")?;
        let mut fields = Fields { parts };
        let identity = RegistryLifecycleActualIdentity {
            path_is_registry_lifecycle: fields.boolean()?,
            topology_is_shared_non_final: fields.boolean()?,
            unmap_is_keep: fields.boolean()?,
            node_is_live: fields.boolean()?,
            variant: fields.u8()?,
            pre_shared_mask: fields.u8()?,
            pre_exclusive_mask: fields.u8()?,
            phase: RegistryLifecyclePhase::try_from(fields.u64()?)?,
            cause_phase_is_none: fields.boolean()?,
            timing: RegistryLifecycleTiming::try_from(fields.u64()?)?,
            class: RegistryLifecycleFailureClass::try_from(fields.u64()?)?,
            target: RegistryLifecycleActualTarget {
                scope_is_route_main: fields.boolean()?,
                registration_id: fields.u64()?,
                route_ordinal: fields.u64()?,
                runtime_generation: fields.u64()?,
                shm_connection_id: fields.u64()?,
                role_is_main: fields.boolean()?,
                callback_is_close: fields.boolean()?,
                occurrence: fields.u32()?,
            },
            sqlite_outcome: RegistryLifecycleSqliteOutcome::try_from(fields.u64()?)?,
        };
        let actual = Self {
            selector,
            identity,
            mutation_may_have_occurred: fields.boolean()?,
            lock_outcome_uncertain: fields.boolean()?,
            domain_terminal: fields.boolean()?,
            registry_route_phase: RegistryLifecycleRegistryRoutePhase::try_from(fields.u64()?)?,
            logical_route_phase: RegistryLifecycleLogicalRoutePhase::try_from(fields.u64()?)?,
            registration_phase: RegistryLifecycleRegistrationPhase::try_from(fields.u64()?)?,
            later_callback_allowed: fields.boolean()?,
            pre: fields.topology()?,
            post: fields.topology()?,
            retained: fields.custody()?,
            counts: fields.counts()?,
        };
        if fields.parts.next().is_some() || actual.to_report_payload() != input {
            return Err("RegistryLifecycle report payload is not canonical");
        }
        Ok(actual)
    }
}

struct Fields<'a> {
    parts: std::str::Split<'a, char>,
}

impl Fields<'_> {
    fn u64(&mut self) -> Result<u64, &'static str> {
        self.parts
            .next()
            .ok_or("RegistryLifecycle report payload is truncated")?
            .parse()
            .map_err(|_| "RegistryLifecycle report field is not an unsigned integer")
    }

    fn u32(&mut self) -> Result<u32, &'static str> {
        self.u64()?
            .try_into()
            .map_err(|_| "RegistryLifecycle report field exceeds u32")
    }

    fn u8(&mut self) -> Result<u8, &'static str> {
        self.u64()?
            .try_into()
            .map_err(|_| "RegistryLifecycle report field exceeds u8")
    }

    fn boolean(&mut self) -> Result<bool, &'static str> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("RegistryLifecycle report boolean is not zero or one"),
        }
    }

    fn topology(&mut self) -> Result<RegistryLifecycleActualTopology, &'static str> {
        Ok(RegistryLifecycleActualTopology {
            sqlite_connections: self.u8()?,
            shm_connections: self.u8()?,
            registry_routes: self.u8()?,
            logical_names: self.u8()?,
        })
    }

    fn custody(&mut self) -> Result<RegistryLifecycleActualCustody, &'static str> {
        Ok(RegistryLifecycleActualCustody {
            node: self.boolean()?,
            views: self.u8()?,
            mappings: self.u8()?,
            dms: RegistryLifecycleDmsCustody::try_from(self.u64()?)?,
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

    fn counts(&mut self) -> Result<RegistryLifecycleActualCounts, &'static str> {
        Ok(RegistryLifecycleActualCounts {
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
