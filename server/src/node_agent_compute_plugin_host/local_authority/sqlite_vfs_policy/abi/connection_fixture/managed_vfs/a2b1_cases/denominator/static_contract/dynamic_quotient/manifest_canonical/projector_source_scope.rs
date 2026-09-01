//! Canonical projector source closure and digest.

use super::super::super::source_leaf_authority::Digest32;
use super::super::lock_callback_completion_route_unknown_source_scope::callback_completion_route_unknown_projector_delta_entries_v1;
use super::super::lock_local_protocol_rejection_source_scope::LOCAL_PROTOCOL_REJECTION_PROJECTOR_DELTA_V1;
use super::super::lock_native_acquire_busy_source_scope::NATIVE_BUSY_PROJECTOR_DELTA_V1;
use super::super::lock_stored_poison_source_scope::STORED_POISON_PROJECTOR_DELTA_V1;
use super::super::map_runtime_source_scope::{
    MAP_REGION_LOOP_SOURCE_SCOPE_V1, MAP_RUNTIME_DEPENDENCY_SOURCE_SCOPE_V1,
};
use super::super::runner_admission::{
    ABI_SCALAR_REJECTION_PROJECTOR_DELTA_V1,
    NATIVE_ACQUIRE_CREATED_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1,
    NATIVE_ACQUIRE_CREATED_FIRST_TRUNCATE_ERROR_RELEASE_FAILED_PROJECTOR_DELTA_V1,
    NATIVE_ACQUIRE_CREATED_FIRST_TRUNCATE_ERROR_RELEASE_SUCCEEDED_PROJECTOR_DELTA_V1,
    NATIVE_ACQUIRE_EXISTING_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1,
    NATIVE_ACQUIRE_EXISTING_FIRST_TRUNCATE_ERROR_RELEASE_SUCCEEDED_PROJECTOR_DELTA_V1,
    PRE_MANAGED_CALLBACK_REJECTION_PROJECTOR_DELTA_V1, RAW_STATE_REJECTION_PROJECTOR_DELTA_V1,
};
use super::{StableHasher, PROJECTOR_SOURCE_SCOPE_V1};

const PROJECTOR_SOURCE_SCOPE_DOMAIN: &str = "ELON-A2-MAP-LOCK-DYNAMIC-PROJECTOR-SOURCE-SCOPE-V1";

pub(in super::super) fn digest_projector_source_scope_v1() -> Digest32 {
    digest_projector_source_entries_v1(projector_source_scope_entries_v1())
}

pub(in super::super) fn projector_source_scope_entries_v1(
) -> impl Iterator<Item = (&'static str, &'static str)> {
    use self::{
        NATIVE_ACQUIRE_CREATED_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1 as NATIVE_RELEASE_DELTA,
        PRE_MANAGED_CALLBACK_REJECTION_PROJECTOR_DELTA_V1 as PRE_MANAGED_DELTA,
    };

    PROJECTOR_SOURCE_SCOPE_V1
        .iter()
        .copied()
        .chain(MAP_RUNTIME_DEPENDENCY_SOURCE_SCOPE_V1.iter().copied())
        .chain(MAP_REGION_LOOP_SOURCE_SCOPE_V1.iter().copied())
        .chain(STORED_POISON_PROJECTOR_DELTA_V1.iter().copied())
        .chain(NATIVE_BUSY_PROJECTOR_DELTA_V1.iter().copied())
        .chain(callback_completion_route_unknown_projector_delta_entries_v1())
        .chain(LOCAL_PROTOCOL_REJECTION_PROJECTOR_DELTA_V1.iter().copied())
        .chain(PRE_MANAGED_DELTA.iter().copied())
        .chain(ABI_SCALAR_REJECTION_PROJECTOR_DELTA_V1.iter().copied())
        .chain(RAW_STATE_REJECTION_PROJECTOR_DELTA_V1.iter().copied())
        .chain(NATIVE_RELEASE_DELTA.iter().copied())
        .chain(
            NATIVE_ACQUIRE_EXISTING_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1
                .iter()
                .copied(),
        )
        .chain(
            NATIVE_ACQUIRE_CREATED_FIRST_TRUNCATE_ERROR_RELEASE_SUCCEEDED_PROJECTOR_DELTA_V1
                .iter()
                .copied(),
        )
        .chain(
            NATIVE_ACQUIRE_EXISTING_FIRST_TRUNCATE_ERROR_RELEASE_SUCCEEDED_PROJECTOR_DELTA_V1
                .iter()
                .copied(),
        )
        .chain(
            NATIVE_ACQUIRE_CREATED_FIRST_TRUNCATE_ERROR_RELEASE_FAILED_PROJECTOR_DELTA_V1
                .iter()
                .copied(),
        )
}

pub(in super::super) fn digest_projector_source_entries_v1<'a>(
    sources: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Digest32 {
    let mut out = StableHasher::new(PROJECTOR_SOURCE_SCOPE_DOMAIN);
    for (name, source) in sources {
        out.text("source_name", name);
        out.bytes("source_lf_bytes", source.as_bytes());
    }
    out.finish()
}
