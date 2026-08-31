//! Identity-erasing, fail-closed projection from static terminals to dynamic execution classes.

mod candidate;
mod canonical;
mod canonical_tags;
mod catalog;
mod descriptor_binding;
mod manifest;
mod manifest_canonical;
mod membership_commitment;
mod model;
mod producer_coherence;
mod projector;

pub(crate) use candidate::{
    build_lock_dynamic_candidate_v1, build_map_dynamic_candidate_v1, DynamicCandidateErrorV1,
};
use canonical::{
    digest_dynamic_class_key_v1, digest_dynamic_expected_v1,
    digest_normalized_descriptor_semantics_v1, digest_static_member_seal_v1,
};
pub(crate) use catalog::CatalogErrorV1;
use catalog::{DynamicCatalogBuilderV1, DynamicCatalogV1, DynamicClassV1, ProjectionFailureV1};
pub(crate) use manifest::DynamicManifestBundleV1;
use manifest::{
    build_dynamic_manifest_v1, DynamicClassSealV1, DynamicManifestContextV1,
    DynamicQuotientManifestV1, ManifestBuildErrorV1, ReverseIndexEntryV1,
};
use model::{
    DynamicAxesV1, DynamicClassKeyV1, DynamicExpectedV1, DynamicOperationV1, DynamicProjectionV1,
    StaticMemberSealV1, DYNAMIC_PROJECTOR_SCHEMA_V1,
};
use projector::{
    project_dynamic_class_v1, project_validated_dynamic_terminal_v1, ProjectionErrorV1,
    ProjectionViolationV1,
};

#[cfg(test)]
mod tests;
