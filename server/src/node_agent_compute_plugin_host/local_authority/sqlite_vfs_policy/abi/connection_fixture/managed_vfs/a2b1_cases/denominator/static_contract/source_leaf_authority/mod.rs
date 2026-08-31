//! Independent, source-first authority primitives for the production-Windows Map/Lock ledger.
//!
//! The source scope, range/profile ledgers, frozen leaf TSV and frozen manifests remain independent
//! from graph builder helpers.  The graph adapter is comparison-only: it projects current DFS
//! paths into neutral V1 records and cannot bless or rewrite checked-in authority files.

mod accumulator;
mod adapter;
mod canonical;
mod comparison;
mod coverage;
mod expected;
mod frozen;
mod leaf_seal;
mod lock_ranges;
mod manifest;
mod manifest_tsv;
mod map_profiles;
mod model;
mod observer;
mod source_scope;
mod source_scope_support;
mod trusted_context;

pub(crate) use accumulator::ManifestAccumulatorV1;
pub(crate) use adapter::{
    exact_graph_leaf_identity, stream_graph_manifest, stream_graph_manifest_with_identity,
    stream_graph_manifest_with_identity_and_records, stream_graph_manifest_with_records,
    validate_graph_against_frozen, validate_graph_against_frozen_with_records,
};
pub(crate) use canonical::{
    digest_case_key, digest_full_record, digest_included_member_pair_set, digest_leaf_identity,
    digest_lock_range_set, digest_map_ordinal_domains, digest_map_profile_set, digest_source_scope,
};
pub(crate) use comparison::{compare_exact_records, RecordDiff};
pub(crate) use expected::{
    CustodyStateV1, DmsLockCustodyV1, ExpectedV1, FailureClassV1, LockEffectV1, LockModeV1,
    MutationStateV1, ObservableCountsV1, SqliteResultV1, TerminalDispositionV1,
};
pub(crate) use frozen::{
    validate_lock_graph, validate_lock_graph_with_records,
    validate_lock_graph_with_records_and_binding, validate_map_graph,
    validate_map_graph_with_records, validate_map_graph_with_records_and_binding,
    FrozenStaticBindingV1,
};
pub(crate) use leaf_seal::{
    encode_leaf_seal_tsv, leaf_seal_tsv_sha256, parse_leaf_seal_tsv, FrozenLeafSealV1,
    FrozenLeafSealVerifierV1, LeafSealOutcomeV1, LeafSealV1, LEAF_SEAL_TSV_HEADER_V1,
};
pub(crate) use lock_ranges::{validate_lock_ranges, LockActionV1, LockRangeV1, LOCK_RANGES};
pub(crate) use manifest::{
    build_manifest, validate_actual_against_frozen, validate_derived_manifest_against_frozen,
    AuthorityValidationError,
};
pub(crate) use manifest_tsv::{encode_manifest_tsv, parse_manifest_tsv, MANIFEST_TSV_HEADER_V1};
pub(crate) use map_profiles::{
    validate_map_profiles, MapFilePathV1, MapInitializationProfileV1, MapLoopProfileV1, MapModeV1,
    MapRegionPrestateV1, MapRegionSizeArmV1, OrdinalDomainV1, MAP_LOOP_PROFILES,
};
pub(crate) use model::{
    CaseKeyV1, CoordinateV1, DecisionStageV1, DecisionV1, Digest32, ExclusionKindV1,
    ExclusionProofV1, LeafIdentityV1, LeafOutcomeV1, LeafRecordV1, ManifestContextV1,
    RootManifestV1, RootOperationV1, ShardManifestV1, SourceScopeFileV1, SourceWitnessV1,
};
pub(crate) use observer::StreamedLeafV1;
pub(crate) use source_scope::{
    source_scope_files, source_scope_sha256, validate_baseline_path_blob,
    validate_baseline_path_blobs, validate_record_source_witnesses, validate_source_scope,
    validate_source_witness, ProductionSourceSnapshotV1, PRODUCTION_SOURCE_SCOPE,
    SOURCE_BASELINE_COMMIT_SHA1,
};
pub(crate) use trusted_context::trusted_current_context;

pub(crate) const AUTHORITY_SCHEMA_V1: &str = "elon.a2b1.vfs.source-leaf-authority.v1";
pub(crate) const PRODUCTION_WINDOWS_X64_SCOPE: &str = "production-windows-x64-no-test";
pub(crate) const MANIFEST_SHARDS: usize = 256;
pub(crate) const MAP_LEAF_LEDGER_PARTS: usize = 16;

#[cfg(test)]
mod generation_tests;
#[cfg(test)]
mod tests;
