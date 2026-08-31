//! Stable semantic canonicalization. No class-key digest absorbs CaseKey or leaf identity.

use sha2::{Digest, Sha256};

use super::super::{
    source_leaf_authority::{Digest32, LockEffectV1},
    terminal_descriptor::*,
};
use super::canonical_tags::*;
use super::model::{
    DynamicAxesV1, DynamicClassKeyV1, DynamicExpectedV1, DynamicOperationV1, StaticMemberSealV1,
};

const EXPECTED_DOMAIN: &str = "ELON-A2-MAP-LOCK-DYNAMIC-EXPECTED-V1";
const CLASS_KEY_DOMAIN: &str = "ELON-A2-MAP-LOCK-DYNAMIC-CLASS-KEY-V1";
const MEMBER_SEAL_DOMAIN: &str = "ELON-A2-MAP-LOCK-DYNAMIC-MEMBER-SEAL-V1";

pub(crate) fn digest_dynamic_expected_v1(value: &DynamicExpectedV1) -> Digest32 {
    let mut out = StableHasher::new(EXPECTED_DOMAIN);
    out.text("sqlite", value.sqlite.canonical_name());
    out.text("disposition", value.disposition.canonical_name());
    out.text("phase", value.phase.static_name());
    out.text("failure", value.failure.canonical_name());
    out.text("mutation", value.mutation.canonical_name());
    out.boolean("lock_outcome_uncertain", value.lock_outcome_uncertain);
    encode_lock_effect(&mut out, value.lock_effect);
    out.text("dms_lock", value.dms_lock.canonical_name());
    out.text("raw_slots", value.raw_slots.canonical_name());
    out.text("route", value.route.canonical_name());
    out.text("callback", value.callback.canonical_name());
    out.text("file", value.file.canonical_name());
    out.text("mapping", value.mapping.canonical_name());
    out.text("view", value.view.canonical_name());
    out.text("payload", value.payload.canonical_name());
    out.u16("callback_begin", value.counts.callback_begin);
    out.u16("callback_complete", value.counts.callback_complete);
    out.u16("native_lock", value.counts.native_lock);
    out.u16("native_unlock", value.counts.native_unlock);
    out.u16("file_grow", value.counts.file_grow);
    out.u16("mapping_create", value.counts.mapping_create);
    out.u16("view_map", value.counts.view_map);
    out.finish()
}

pub(crate) fn digest_dynamic_class_key_v1(value: &DynamicClassKeyV1) -> Digest32 {
    let mut out = StableHasher::new(CLASS_KEY_DOMAIN);
    out.u16("schema_version", value.schema_version);
    out.text("root", value.root.canonical_name());
    out.u16("source_site", source_site_tag(value.source_site));
    encode_stimulus(&mut out, value.stimulus);
    encode_prestate(&mut out, value.prestate);
    encode_operation(&mut out, value.operation);
    out.text("phase", value.phase.static_name());
    encode_timing(&mut out, value.timing);
    encode_occurrence(&mut out, value.occurrence);
    encode_recipe(&mut out, value.recipe);
    encode_axes(&mut out, value.axes);
    out.digest(
        "dynamic_expected_sha256",
        digest_dynamic_expected_v1(&value.expected),
    );
    out.finish()
}

pub(crate) fn digest_normalized_descriptor_semantics_v1(value: &DynamicClassKeyV1) -> Digest32 {
    let mut normalized = *value;
    normalized.recipe.capability = RunnerCapabilityV1::Supported;
    digest_dynamic_class_key_v1(&normalized)
}

pub(crate) fn digest_static_member_seal_v1(value: StaticMemberSealV1) -> Digest32 {
    let mut out = StableHasher::new(MEMBER_SEAL_DOMAIN);
    out.digest("case_key_sha256", value.case_key_sha256);
    out.digest("full_record_sha256", value.full_record_sha256);
    out.finish()
}

fn encode_lock_effect(out: &mut StableHasher, value: LockEffectV1) {
    out.text("lock_effect", value.canonical_name());
    match value {
        LockEffectV1::Acquired { mode, mask, native }
        | LockEffectV1::Released { mode, mask, native } => {
            out.text("lock_effect_mode", mode.canonical_name());
            out.u8("lock_effect_mask", mask);
            out.boolean("lock_effect_native", native);
        }
        LockEffectV1::OutcomeUncertain { mode, mask } => {
            out.text("lock_effect_mode", mode.canonical_name());
            out.u8("lock_effect_mask", mask);
        }
        LockEffectV1::NotReached | LockEffectV1::Unchanged => {}
    }
}

fn source_site_tag(value: SourceSiteV1) -> u16 {
    match value {
        SourceSiteV1::MapAbiBoundary => 1,
        SourceSiteV1::LockAbiBoundary => 2,
        SourceSiteV1::RawStateAdmission => 3,
        SourceSiteV1::RawStateAbandon => 4,
        SourceSiteV1::AdapterDispatch => 5,
        SourceSiteV1::RegistryCallbackAdmission => 6,
        SourceSiteV1::ManagedRequestValidation => 7,
        SourceSiteV1::InitializationOpen => 8,
        SourceSiteV1::InitializationDms => 9,
        SourceSiteV1::CoordinatorState => 10,
        SourceSiteV1::MapFileSize => 11,
        SourceSiteV1::MapFileGrow => 12,
        SourceSiteV1::MapMappingCreate => 13,
        SourceSiteV1::MapViewMap => 14,
        SourceSiteV1::MapMappingClose => 15,
        SourceSiteV1::LockLocalState => 16,
        SourceSiteV1::LockNativeAcquire => 17,
        SourceSiteV1::LockNativeRelease => 18,
        SourceSiteV1::FailureCustody => 19,
        SourceSiteV1::CallbackCompletion => 20,
        SourceSiteV1::Quarantine => 21,
        SourceSiteV1::AbiProjection => 22,
    }
}

fn encode_stimulus(out: &mut StableHasher, value: StimulusV1) {
    match value {
        StimulusV1::MapAbi(value) => {
            out.u8("stimulus_kind", 1);
            out.u16("map_output", presence_tag(value.output));
            out.u16("map_region", validity_tag(value.region));
            out.u16("map_region_size", validity_tag(value.region_size));
            out.u16("map_extend", validity_tag(value.extend));
        }
        StimulusV1::LockAbi(value) => {
            out.u8("stimulus_kind", 2);
            out.u16("lock_offset", validity_tag(value.offset));
            out.u16("lock_count", validity_tag(value.count));
            out.u16("lock_flags", validity_tag(value.flags));
        }
        StimulusV1::MapRaw(value) => {
            out.u8("stimulus_kind", 3);
            out.u16("raw_state", raw_state_tag(value));
        }
        StimulusV1::LockRaw(value) => {
            out.u8("stimulus_kind", 4);
            out.u16("raw_state", raw_state_tag(value));
        }
        StimulusV1::MapManaged(value) => {
            out.u8("stimulus_kind", 5);
            out.u16("map_managed", map_stimulus_tag(value));
        }
        StimulusV1::LockManaged(value) => {
            out.u8("stimulus_kind", 6);
            out.u16("lock_managed", lock_stimulus_tag(value));
        }
        StimulusV1::Initialization(value) => {
            out.u8("stimulus_kind", 7);
            out.u16(
                "initialization_fault_site",
                initialization_fault_tag(value.fault_site),
            );
            out.u16("initialization_path", initialization_path_tag(value.path));
            out.boolean("initialization_cleanup_rewrite", value.cleanup_rewrite);
        }
    }
}

fn encode_prestate(out: &mut StableHasher, value: PrestateV1) {
    match value {
        PrestateV1::Map(value) => {
            out.u8("prestate_root", 1);
            match value {
                MapPrestateV1::NotReached => out.u8("map_prestate", 0),
                MapPrestateV1::NodeAbsent => out.u8("map_prestate", 1),
                MapPrestateV1::RegionsEmpty => out.u8("map_prestate", 2),
                MapPrestateV1::TargetMissing => out.u8("map_prestate", 3),
                MapPrestateV1::TargetMapped => out.u8("map_prestate", 4),
                MapPrestateV1::StoredPoison(poison) => {
                    out.u8("map_prestate", 5);
                    out.u16("map_stored_poison", map_stored_poison_tag(poison));
                }
            }
        }
        PrestateV1::Lock(value) => {
            out.u8("prestate_root", 2);
            out.u16("lock_prestate", lock_prestate_tag(value));
        }
    }
}

fn encode_operation(out: &mut StableHasher, value: DynamicOperationV1) {
    match value {
        DynamicOperationV1::Map(value) => {
            out.u8("operation_root", 1);
            out.u16("operation", map_operation_tag(value));
        }
        DynamicOperationV1::Lock(value) => {
            out.u8("operation_root", 2);
            out.u16("operation", lock_operation_tag(value));
        }
    }
}

fn encode_timing(out: &mut StableHasher, value: TimingV1) {
    out.u8(
        "timing",
        match value {
            TimingV1::NotReached => 0,
            TimingV1::Natural => 1,
            TimingV1::BeforeCall => 2,
            TimingV1::AtCall => 3,
            TimingV1::AfterSuccess => 4,
            TimingV1::Cleanup => 5,
        },
    );
}

fn encode_occurrence(out: &mut StableHasher, value: OccurrenceV1) {
    match value {
        OccurrenceV1::NotReached => out.u8("occurrence_kind", 0),
        OccurrenceV1::Natural => out.u8("occurrence_kind", 1),
        OccurrenceV1::Exact(value) => {
            out.u8("occurrence_kind", 2);
            out.u16("occurrence_exact", value);
        }
    }
}

fn encode_recipe(out: &mut StableHasher, value: ExecutionRecipeV1) {
    out.u16("fixture", fixture_tag(value.fixture));
    out.u16("callback_recipe", callback_tag(value.callback));
    out.u16("fault_seam", fault_seam_tag(value.fault_seam));
    out.u16("observer", observer_tag(value.observer));
    out.u16("cleanup", cleanup_tag(value.cleanup));
    match value.capability {
        RunnerCapabilityV1::Supported => out.u8("runner_capability", 1),
        RunnerCapabilityV1::Missing(gap) => {
            out.u8("runner_capability", 2);
            out.u16("capability_gap", gap_tag(gap));
        }
    }
}

fn encode_axes(out: &mut StableHasher, value: DynamicAxesV1) {
    match value {
        DynamicAxesV1::Map(value) => {
            out.u8("axes_root", 1);
            encode_map_axes(out, value);
        }
        DynamicAxesV1::Lock(value) => {
            out.u8("axes_root", 2);
            encode_lock_axes(out, value);
        }
    }
}

fn encode_map_axes(out: &mut StableHasher, value: MapAxesV1) {
    encode_reachable(out, "map_mode", value.mode, map_mode_tag);
    match value.profile {
        ReachabilityV1::NotReached => out.u8("map_profile_reached", 0),
        ReachabilityV1::Reached(profile) => {
            out.u8("map_profile_reached", 1);
            let mut nested = StableHasher::new("ELON-A2-MAP-LOCK-MAP-PROFILE-V1");
            nested.u16("mode", map_mode_tag(profile.mode));
            nested.u16("initialization", initialization_tag(profile.initialization));
            nested.u16("prestate", region_prestate_tag(profile.prestate));
            nested.u16("region_size_arm", region_size_tag(profile.region_size_arm));
            nested.u16("file_path", file_path_tag(profile.file_path));
            nested.boolean("prior_mutation", profile.prior_mutation);
            nested.boolean("preexisting_mapping", profile.preexisting_mapping);
            out.digest("map_profile", nested.finish());
        }
    }
    encode_reachable_u16(out, "map_ordinal", value.ordinal);
    encode_reachable_u16(out, "regions_to_create", value.regions_to_create);
    encode_reachable(out, "map_completion", value.completion, map_completion_tag);
}

fn encode_lock_axes(out: &mut StableHasher, value: LockAxesV1) {
    encode_reachable(out, "lock_action", value.action, lock_action_tag);
    encode_reachable_u8(out, "lock_first", value.first);
    encode_reachable_u8(out, "lock_count", value.count);
    encode_reachable_u8(out, "lock_mask", value.mask);
    encode_reachable(
        out,
        "lock_initialization",
        value.initialization,
        initialization_tag,
    );
    encode_reachable_u8(out, "held_shared_mask", value.held_shared_mask);
    encode_reachable_u8(out, "held_exclusive_mask", value.held_exclusive_mask);
    encode_reachable_u8(out, "sibling_shared_mask", value.sibling_shared_mask);
    encode_reachable_u8(out, "sibling_exclusive_mask", value.sibling_exclusive_mask);
    encode_reachable(
        out,
        "lock_completion",
        value.completion,
        lock_completion_tag,
    );
}

fn encode_reachable<T: Copy>(
    out: &mut StableHasher,
    label: &str,
    value: ReachabilityV1<T>,
    tag: fn(T) -> u16,
) {
    match value {
        ReachabilityV1::NotReached => out.u8(&format!("{label}_reached"), 0),
        ReachabilityV1::Reached(value) => {
            out.u8(&format!("{label}_reached"), 1);
            out.u16(label, tag(value));
        }
    }
}

fn encode_reachable_u8(out: &mut StableHasher, label: &str, value: ReachabilityV1<u8>) {
    match value {
        ReachabilityV1::NotReached => out.u8(&format!("{label}_reached"), 0),
        ReachabilityV1::Reached(value) => {
            out.u8(&format!("{label}_reached"), 1);
            out.u8(label, value);
        }
    }
}

fn encode_reachable_u16(out: &mut StableHasher, label: &str, value: ReachabilityV1<u16>) {
    match value {
        ReachabilityV1::NotReached => out.u8(&format!("{label}_reached"), 0),
        ReachabilityV1::Reached(value) => {
            out.u8(&format!("{label}_reached"), 1);
            out.u16(label, value);
        }
    }
}

struct StableHasher(Sha256);

impl StableHasher {
    fn new(domain: &str) -> Self {
        let mut value = Sha256::new();
        value.update(domain.as_bytes());
        value.update([0]);
        Self(value)
    }
    fn text(&mut self, label: &str, value: &str) {
        self.bytes(label, value.as_bytes());
    }
    fn boolean(&mut self, label: &str, value: bool) {
        self.bytes(label, &[u8::from(value)]);
    }
    fn u8(&mut self, label: &str, value: u8) {
        self.bytes(label, &[value]);
    }
    fn u16(&mut self, label: &str, value: u16) {
        self.bytes(label, &value.to_be_bytes());
    }
    fn digest(&mut self, label: &str, value: Digest32) {
        self.bytes(label, &value.0);
    }
    fn bytes(&mut self, label: &str, value: &[u8]) {
        self.0.update(label.as_bytes());
        self.0.update([0]);
        self.0.update((value.len() as u64).to_be_bytes());
        self.0.update(value);
    }
    fn finish(self) -> Digest32 {
        Digest32(self.0.finalize().into())
    }
}
