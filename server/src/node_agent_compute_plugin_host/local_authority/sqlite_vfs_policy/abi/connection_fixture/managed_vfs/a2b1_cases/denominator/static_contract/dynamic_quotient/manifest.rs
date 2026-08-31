use super::super::source_leaf_authority::{
    digest_included_member_pair_set, Digest32, FrozenStaticBindingV1, RootOperationV1,
};
use super::manifest_canonical::{
    digest_class_catalog_v1, digest_class_key_set_v1, digest_class_record_v1,
    digest_dynamic_manifest_body_v1, digest_erasure_proof_v1, digest_member_set_v1,
    digest_membership_map_v1, digest_projector_schema_v1, digest_projector_source_scope_v1,
    digest_representative_map_v1, digest_retained_axes_v1,
};
use super::membership_commitment::digest_projected_membership_v1;
use super::{
    digest_dynamic_class_key_v1, DynamicCatalogV1, DynamicClassKeyV1, StaticMemberSealV1,
    DYNAMIC_PROJECTOR_SCHEMA_V1,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DynamicManifestContextV1 {
    pub(crate) schema_version: u16,
    pub(crate) root: RootOperationV1,
    pub(crate) static_source_baseline_sha1: String,
    pub(crate) static_source_scope_sha256: Digest32,
    pub(crate) static_ledger_sha256: Digest32,
    pub(crate) static_manifest_sha256: Digest32,
    pub(crate) static_member_pair_set_sha256: Digest32,
    pub(crate) static_included_count: u64,
    pub(crate) static_excluded_count: u64,
    pub(crate) static_source_universe_count: u64,
    pub(crate) projector_schema_sha256: Digest32,
    pub(crate) projector_source_scope_sha256: Digest32,
    pub(crate) descriptor_binding_sha256: Digest32,
    pub(crate) runner_admission_binding_sha256: Digest32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DynamicClassSealV1 {
    pub(crate) key: DynamicClassKeyV1,
    pub(crate) class_key_sha256: Digest32,
    pub(crate) class_id: Digest32,
    pub(crate) member_count: u64,
    pub(crate) member_set_sha256: Digest32,
    pub(crate) representative: StaticMemberSealV1,
    pub(crate) retained_axes_sha256: Digest32,
    pub(crate) erased_axes_proof_sha256: Digest32,
    pub(crate) class_record_sha256: Digest32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ReverseIndexEntryV1 {
    pub(crate) member: StaticMemberSealV1,
    pub(crate) class_key_sha256: Digest32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DynamicQuotientManifestV1 {
    pub(crate) context: DynamicManifestContextV1,
    pub(crate) class_count: u64,
    pub(crate) member_count: u64,
    pub(crate) class_key_set_sha256: Digest32,
    pub(crate) membership_map_sha256: Digest32,
    pub(crate) representative_map_sha256: Digest32,
    pub(crate) class_catalog_sha256: Digest32,
    pub(crate) reverse_index_sha256: Digest32,
    pub(crate) manifest_sha256: Digest32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DynamicManifestBundleV1 {
    pub(crate) manifest: DynamicQuotientManifestV1,
    pub(crate) classes: Vec<DynamicClassSealV1>,
    pub(crate) reverse_index: Vec<ReverseIndexEntryV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManifestBuildErrorV1 {
    RootMismatch,
    StaticUniverseMismatch,
    StaticMemberCountMismatch,
    StaticMemberSetMismatch,
    EmptyCatalog,
    EmptyClass,
    DuplicateClassDigest,
    ClassKeyMismatch,
    MemberOrderMismatch,
    DuplicateMember,
    ProjectedMembershipMismatch,
    RepresentativeMismatch,
    CountOverflow,
}

pub(crate) fn build_dynamic_manifest_v1(
    binding: &FrozenStaticBindingV1,
    catalog: &DynamicCatalogV1,
) -> Result<DynamicManifestBundleV1, ManifestBuildErrorV1> {
    let catalog_root = catalog.root();
    let catalog_member_pair_set = catalog.member_pair_set_sha256();
    let catalog_classes = catalog.classes();
    if binding.context.root != catalog_root {
        return Err(ManifestBuildErrorV1::RootMismatch);
    }
    if binding.included_count.checked_add(binding.excluded_count)
        != Some(binding.source_universe_count)
    {
        return Err(ManifestBuildErrorV1::StaticUniverseMismatch);
    }
    if binding.included_count != catalog.member_count() {
        return Err(ManifestBuildErrorV1::StaticMemberCountMismatch);
    }
    if binding.included_member_pair_set_sha256 != catalog_member_pair_set {
        return Err(ManifestBuildErrorV1::StaticMemberSetMismatch);
    }
    if catalog_classes.is_empty() {
        return Err(ManifestBuildErrorV1::EmptyCatalog);
    }

    let erasure = digest_erasure_proof_v1();
    let mut classes = Vec::with_capacity(catalog_classes.len());
    let mut reverse_index = Vec::new();
    let mut class_digests = std::collections::BTreeSet::new();
    let mut members = std::collections::BTreeSet::new();
    for class in catalog_classes {
        let class_key = class.key();
        let class_key_sha256 = class.class_key_sha256();
        let class_id = class.class_id();
        let class_members = class.members();
        let representative = class.representative();
        if class_members.is_empty() {
            return Err(ManifestBuildErrorV1::EmptyClass);
        }
        if class_key.root != catalog_root
            || class_key.schema_version != DYNAMIC_PROJECTOR_SCHEMA_V1
            || digest_dynamic_class_key_v1(&class_key) != class_key_sha256
            || class_id != class_key_sha256
        {
            return Err(ManifestBuildErrorV1::ClassKeyMismatch);
        }
        if !class_members.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(ManifestBuildErrorV1::MemberOrderMismatch);
        }
        if !class_digests.insert(class_key_sha256) {
            return Err(ManifestBuildErrorV1::DuplicateClassDigest);
        }
        if class_members.iter().min().copied() != Some(representative) {
            return Err(ManifestBuildErrorV1::RepresentativeMismatch);
        }
        for member in class_members {
            if !members.insert(*member) {
                return Err(ManifestBuildErrorV1::DuplicateMember);
            }
            reverse_index.push(ReverseIndexEntryV1 {
                member: *member,
                class_key_sha256,
            });
        }
        let member_set_sha256 = digest_member_set_v1(class_members);
        let retained_axes_sha256 = digest_retained_axes_v1(&class_key);
        let member_count =
            u64::try_from(class_members.len()).map_err(|_| ManifestBuildErrorV1::CountOverflow)?;
        let mut seal = DynamicClassSealV1 {
            key: class_key,
            class_key_sha256,
            class_id,
            member_count,
            member_set_sha256,
            representative,
            retained_axes_sha256,
            erased_axes_proof_sha256: erasure,
            class_record_sha256: Digest32::ZERO,
        };
        seal.class_record_sha256 = digest_class_record_v1(&seal);
        classes.push(seal);
    }
    classes.sort_by_key(|class| class.class_key_sha256);
    reverse_index.sort_unstable();
    if u64::try_from(reverse_index.len()).map_err(|_| ManifestBuildErrorV1::CountOverflow)?
        != binding.included_count
    {
        return Err(ManifestBuildErrorV1::StaticMemberCountMismatch);
    }
    let actual_member_pair_set_sha256 = digest_included_member_pair_set(
        members
            .iter()
            .map(|member| (member.case_key_sha256, member.full_record_sha256))
            .collect(),
    );
    if actual_member_pair_set_sha256 != catalog_member_pair_set
        || actual_member_pair_set_sha256 != binding.included_member_pair_set_sha256
    {
        return Err(ManifestBuildErrorV1::StaticMemberSetMismatch);
    }
    let projected_membership_sha256 = digest_projected_membership_v1(
        catalog_root,
        reverse_index
            .iter()
            .map(|entry| (entry.member, entry.class_key_sha256)),
    );
    if projected_membership_sha256 != catalog.projected_membership_sha256() {
        return Err(ManifestBuildErrorV1::ProjectedMembershipMismatch);
    }

    let context = DynamicManifestContextV1 {
        schema_version: super::DYNAMIC_PROJECTOR_SCHEMA_V1,
        root: catalog_root,
        static_source_baseline_sha1: binding.context.source_baseline_commit_sha1.clone(),
        static_source_scope_sha256: binding.context.source_scope_sha256,
        static_ledger_sha256: binding.context.ledger_sha256,
        static_manifest_sha256: binding.static_manifest_sha256,
        static_member_pair_set_sha256: binding.included_member_pair_set_sha256,
        static_included_count: binding.included_count,
        static_excluded_count: binding.excluded_count,
        static_source_universe_count: binding.source_universe_count,
        projector_schema_sha256: digest_projector_schema_v1(),
        projector_source_scope_sha256: digest_projector_source_scope_v1(),
        descriptor_binding_sha256: catalog.descriptor_binding_sha256(),
        runner_admission_binding_sha256: catalog.runner_admission_binding_sha256(),
    };
    let mut manifest = DynamicQuotientManifestV1 {
        context,
        class_count: u64::try_from(classes.len())
            .map_err(|_| ManifestBuildErrorV1::CountOverflow)?,
        member_count: binding.included_count,
        class_key_set_sha256: digest_class_key_set_v1(&classes),
        membership_map_sha256: digest_membership_map_v1(&classes),
        representative_map_sha256: digest_representative_map_v1(&classes),
        class_catalog_sha256: digest_class_catalog_v1(&classes),
        reverse_index_sha256: projected_membership_sha256,
        manifest_sha256: Digest32::ZERO,
    };
    manifest.manifest_sha256 = digest_dynamic_manifest_body_v1(&manifest);
    Ok(DynamicManifestBundleV1 {
        manifest,
        classes,
        reverse_index,
    })
}
