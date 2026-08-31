use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
mod test_support;

use super::super::source_leaf_authority::{
    digest_included_member_pair_set, Digest32, FrozenStaticBindingV1, LeafSealOutcomeV1,
    RootOperationV1, StreamedLeafV1,
};
use super::super::terminal_descriptor::CapabilityGapV1;
use super::descriptor_binding::{
    checked_in_authority_v1, digest_descriptor_binding_v1, DescriptorBindingContextDriftV1,
    DescriptorBindingEntryV1, FrozenDescriptorBindingAuthorityV1,
};
use super::membership_commitment::digest_projected_membership_v1;
use super::{
    project_validated_dynamic_terminal_v1, DynamicClassKeyV1, ProjectionErrorV1,
    StaticMemberSealV1, DYNAMIC_PROJECTOR_SCHEMA_V1,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DynamicClassV1 {
    key: DynamicClassKeyV1,
    class_key_sha256: Digest32,
    class_id: Digest32,
    members: Vec<StaticMemberSealV1>,
    representative: StaticMemberSealV1,
}

impl DynamicClassV1 {
    pub(super) const fn key(&self) -> DynamicClassKeyV1 {
        self.key
    }

    pub(super) const fn class_key_sha256(&self) -> Digest32 {
        self.class_key_sha256
    }

    pub(super) const fn class_id(&self) -> Digest32 {
        self.class_id
    }

    pub(super) fn members(&self) -> &[StaticMemberSealV1] {
        &self.members
    }

    pub(super) const fn representative(&self) -> StaticMemberSealV1 {
        self.representative
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DynamicCatalogV1 {
    root: RootOperationV1,
    member_count: u64,
    member_pair_set_sha256: Digest32,
    classes: Vec<DynamicClassV1>,
    projected_membership_sha256: Digest32,
    descriptor_binding_sha256: Digest32,
}

impl DynamicCatalogV1 {
    pub(super) const fn root(&self) -> RootOperationV1 {
        self.root
    }

    pub(super) const fn member_count(&self) -> u64 {
        self.member_count
    }

    pub(super) const fn member_pair_set_sha256(&self) -> Digest32 {
        self.member_pair_set_sha256
    }

    pub(super) fn classes(&self) -> &[DynamicClassV1] {
        &self.classes
    }

    pub(super) const fn projected_membership_sha256(&self) -> Digest32 {
        self.projected_membership_sha256
    }

    pub(super) const fn descriptor_binding_sha256(&self) -> Digest32 {
        self.descriptor_binding_sha256
    }
}

#[cfg(test)]
impl DynamicCatalogV1 {
    pub(super) fn tamper_first_class_key_for_test(&mut self, digest: Digest32) {
        self.classes[0].class_key_sha256 = digest;
    }

    pub(super) fn tamper_first_member_full_record_for_test(&mut self, digest: Digest32) {
        self.classes[0].members[0].full_record_sha256 = digest;
        self.classes[0].members.sort_unstable();
        self.classes[0].representative = self.classes[0].members[0];
    }

    pub(super) fn reverse_first_class_members_for_test(&mut self) {
        self.classes[0].members.reverse();
    }

    pub(super) fn swap_first_members_for_test(&mut self) {
        assert!(self.classes.len() >= 2);
        let first = self.classes[0].members[0];
        let second = self.classes[1].members[0];
        self.classes[0].members[0] = second;
        self.classes[1].members[0] = first;
        self.classes[0].members.sort_unstable();
        self.classes[1].members.sort_unstable();
        self.classes[0].representative = self.classes[0].members[0];
        self.classes[1].representative = self.classes[1].members[0];
    }

    pub(super) fn merge_second_class_into_first_for_test(&mut self) {
        assert!(self.classes.len() >= 2);
        let second = self.classes.remove(1);
        self.classes[0].members.extend(second.members);
        self.classes[0].members.sort_unstable();
        self.classes[0].representative = self.classes[0].members[0];
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionFailureV1 {
    pub(crate) member: StaticMemberSealV1,
    pub(crate) error: ProjectionErrorV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CatalogErrorV1 {
    RootMismatch,
    OutcomeMismatch,
    DuplicateStaticMember(StaticMemberSealV1),
    ExcludedMemberProjected(StaticMemberSealV1),
    ProjectedMemberDigestMismatch(StaticMemberSealV1),
    ClassDigestCollision(Digest32),
    DescriptorBindingContextDrift(DescriptorBindingContextDriftV1),
    DescriptorBindingCommitmentDrift {
        expected: Digest32,
        actual: Digest32,
    },
    ProjectionFailed {
        count: u64,
        first: ProjectionFailureV1,
    },
    RunnerCapabilityMissing {
        count: u64,
        gap: CapabilityGapV1,
        first_member: StaticMemberSealV1,
    },
    MixedRunnerCapabilityState {
        supported: u64,
        missing: u64,
        first_missing: ProjectionFailureV1,
    },
    MixedRunnerCapabilityGaps {
        count: u64,
        first: ProjectionFailureV1,
        conflicting: ProjectionFailureV1,
    },
    MissingProjection(u64),
    ExtraProjection(u64),
    EmptyClass,
    CountOverflow,
}

pub(crate) struct DynamicCatalogBuilderV1 {
    root: RootOperationV1,
    static_members: BTreeSet<StaticMemberSealV1>,
    excluded_members: BTreeSet<StaticMemberSealV1>,
    projected_members: BTreeSet<StaticMemberSealV1>,
    classes: BTreeMap<DynamicClassKeyV1, BTreeSet<StaticMemberSealV1>>,
    class_digests: BTreeMap<DynamicClassKeyV1, Digest32>,
    digest_owners: BTreeMap<Digest32, DynamicClassKeyV1>,
    projected_membership: BTreeMap<StaticMemberSealV1, Digest32>,
    descriptor_bindings: BTreeMap<StaticMemberSealV1, Digest32>,
    frozen_descriptor_binding: Option<FrozenDescriptorBindingAuthorityV1>,
    projection_failures: Vec<ProjectionFailureV1>,
}

impl DynamicCatalogBuilderV1 {
    #[cfg(test)]
    pub(crate) fn new(root: RootOperationV1) -> Self {
        Self {
            root,
            static_members: BTreeSet::new(),
            excluded_members: BTreeSet::new(),
            projected_members: BTreeSet::new(),
            classes: BTreeMap::new(),
            class_digests: BTreeMap::new(),
            digest_owners: BTreeMap::new(),
            projected_membership: BTreeMap::new(),
            descriptor_bindings: BTreeMap::new(),
            frozen_descriptor_binding: None,
            projection_failures: Vec::new(),
        }
    }

    pub(crate) fn from_frozen_static_binding(
        binding: &FrozenStaticBindingV1,
    ) -> Result<Self, CatalogErrorV1> {
        let authority = checked_in_authority_v1(binding)
            .map_err(CatalogErrorV1::DescriptorBindingContextDrift)?;
        Ok(Self {
            root: binding.context.root,
            static_members: BTreeSet::new(),
            excluded_members: BTreeSet::new(),
            projected_members: BTreeSet::new(),
            classes: BTreeMap::new(),
            class_digests: BTreeMap::new(),
            digest_owners: BTreeMap::new(),
            projected_membership: BTreeMap::new(),
            descriptor_bindings: BTreeMap::new(),
            frozen_descriptor_binding: Some(authority),
            projection_failures: Vec::new(),
        })
    }

    #[cfg(test)]
    pub(super) fn freeze_descriptor_binding_for_test(
        &mut self,
        authority: FrozenDescriptorBindingAuthorityV1,
    ) {
        self.frozen_descriptor_binding = Some(authority);
    }

    pub(crate) fn observe(&mut self, leaf: StreamedLeafV1<'_>) -> Result<(), CatalogErrorV1> {
        let root = leaf.seal().root;
        let outcome = leaf.seal().outcome;
        let member = StaticMemberSealV1 {
            case_key_sha256: leaf.seal().case_key_sha256,
            full_record_sha256: leaf.seal().full_record_sha256,
        };
        if root != self.root {
            return Err(CatalogErrorV1::RootMismatch);
        }
        match leaf {
            StreamedLeafV1::Excluded { .. } => {
                if outcome != LeafSealOutcomeV1::Excluded {
                    return Err(CatalogErrorV1::OutcomeMismatch);
                }
                if self.static_members.contains(&member) {
                    return Err(CatalogErrorV1::ExcludedMemberProjected(member));
                }
                if !self.excluded_members.insert(member) {
                    return Err(CatalogErrorV1::DuplicateStaticMember(member));
                }
            }
            StreamedLeafV1::Terminal {
                record, descriptor, ..
            } => {
                if outcome != LeafSealOutcomeV1::Terminal {
                    return Err(CatalogErrorV1::OutcomeMismatch);
                }
                if self.excluded_members.contains(&member) {
                    return Err(CatalogErrorV1::ExcludedMemberProjected(member));
                }
                if !self.static_members.insert(member) {
                    return Err(CatalogErrorV1::DuplicateStaticMember(member));
                }
                match project_validated_dynamic_terminal_v1(record, descriptor) {
                    Ok(validated) => {
                        self.observe_descriptor_binding(member, validated.descriptor_binding)?;
                        match validated.projection {
                            Ok(projection) => self.observe_projection(member, projection)?,
                            Err(gap) => self.projection_failures.push(ProjectionFailureV1 {
                                member,
                                error: ProjectionErrorV1::RunnerCapabilityMissing(gap),
                            }),
                        }
                    }
                    Err(error) => self
                        .projection_failures
                        .push(ProjectionFailureV1 { member, error }),
                }
            }
        }
        Ok(())
    }

    fn observe_descriptor_binding(
        &mut self,
        expected_member: StaticMemberSealV1,
        entry: DescriptorBindingEntryV1,
    ) -> Result<(), CatalogErrorV1> {
        if entry.member != expected_member {
            return Err(CatalogErrorV1::ProjectedMemberDigestMismatch(
                expected_member,
            ));
        }
        if self
            .descriptor_bindings
            .insert(entry.member, entry.descriptor_semantic_sha256)
            .is_some()
        {
            return Err(CatalogErrorV1::DuplicateStaticMember(entry.member));
        }
        Ok(())
    }

    fn observe_projection(
        &mut self,
        expected_member: StaticMemberSealV1,
        projection: super::DynamicProjectionV1,
    ) -> Result<(), CatalogErrorV1> {
        if projection.member != expected_member {
            return Err(CatalogErrorV1::ProjectedMemberDigestMismatch(
                expected_member,
            ));
        }
        if self.excluded_members.contains(&projection.member) {
            return Err(CatalogErrorV1::ExcludedMemberProjected(projection.member));
        }
        if !self.projected_members.insert(projection.member) {
            return Err(CatalogErrorV1::DuplicateStaticMember(projection.member));
        }
        if self
            .projected_membership
            .insert(projection.member, projection.class_key_sha256)
            .is_some()
        {
            return Err(CatalogErrorV1::DuplicateStaticMember(projection.member));
        }
        if let Some(owner) = self.digest_owners.get(&projection.class_key_sha256) {
            if owner != &projection.key {
                return Err(CatalogErrorV1::ClassDigestCollision(
                    projection.class_key_sha256,
                ));
            }
        } else {
            self.digest_owners
                .insert(projection.class_key_sha256, projection.key);
        }
        self.classes
            .entry(projection.key)
            .or_default()
            .insert(projection.member);
        self.class_digests
            .insert(projection.key, projection.class_key_sha256);
        Ok(())
    }

    pub(crate) fn finish(self) -> Result<DynamicCatalogV1, CatalogErrorV1> {
        let descriptor_context = self.frozen_descriptor_binding.map_or(
            super::descriptor_binding::DescriptorBindingContextV1 {
                root: self.root,
                projector_schema_version: DYNAMIC_PROJECTOR_SCHEMA_V1,
                static_manifest_sha256: Digest32::ZERO,
                included_count: u64::try_from(self.static_members.len())
                    .map_err(|_| CatalogErrorV1::CountOverflow)?,
            },
            |authority| authority.context,
        );
        let descriptor_binding_sha256 = digest_descriptor_binding_v1(
            descriptor_context,
            self.descriptor_bindings
                .iter()
                .map(|(member, digest)| DescriptorBindingEntryV1 {
                    member: *member,
                    descriptor_semantic_sha256: *digest,
                }),
        );
        if let Some(authority) = self.frozen_descriptor_binding {
            if descriptor_binding_sha256 != authority.descriptor_binding_sha256 {
                return Err(CatalogErrorV1::DescriptorBindingCommitmentDrift {
                    expected: authority.descriptor_binding_sha256,
                    actual: descriptor_binding_sha256,
                });
            }
        }
        let semantic_failure_count = u64::try_from(
            self.projection_failures
                .iter()
                .filter(|failure| {
                    !matches!(failure.error, ProjectionErrorV1::RunnerCapabilityMissing(_))
                })
                .count(),
        )
        .map_err(|_| CatalogErrorV1::CountOverflow)?;
        if let Some(first) =
            self.projection_failures.iter().copied().find(|failure| {
                !matches!(failure.error, ProjectionErrorV1::RunnerCapabilityMissing(_))
            })
        {
            return Err(CatalogErrorV1::ProjectionFailed {
                count: semantic_failure_count,
                first,
            });
        }
        if let Some(first) = self.projection_failures.first().copied() {
            let missing = u64::try_from(self.projection_failures.len())
                .map_err(|_| CatalogErrorV1::CountOverflow)?;
            let supported = u64::try_from(self.projected_members.len())
                .map_err(|_| CatalogErrorV1::CountOverflow)?;
            if supported != 0 {
                return Err(CatalogErrorV1::MixedRunnerCapabilityState {
                    supported,
                    missing,
                    first_missing: first,
                });
            }
            let ProjectionErrorV1::RunnerCapabilityMissing(gap) = first.error else {
                unreachable!("non-capability projection failure was selected above")
            };
            if let Some(conflicting) =
                self.projection_failures
                    .iter()
                    .copied()
                    .find(|failure| match failure.error {
                        ProjectionErrorV1::RunnerCapabilityMissing(other) => other != gap,
                        _ => false,
                    })
            {
                return Err(CatalogErrorV1::MixedRunnerCapabilityGaps {
                    count: missing,
                    first,
                    conflicting,
                });
            }
            return Err(CatalogErrorV1::RunnerCapabilityMissing {
                count: missing,
                gap,
                first_member: first.member,
            });
        }
        let missing = self
            .static_members
            .difference(&self.projected_members)
            .count();
        if missing != 0 {
            return Err(CatalogErrorV1::MissingProjection(
                u64::try_from(missing).map_err(|_| CatalogErrorV1::CountOverflow)?,
            ));
        }
        let extra = self
            .projected_members
            .difference(&self.static_members)
            .count();
        if extra != 0 {
            return Err(CatalogErrorV1::ExtraProjection(
                u64::try_from(extra).map_err(|_| CatalogErrorV1::CountOverflow)?,
            ));
        }
        let mut classes = Vec::with_capacity(self.classes.len());
        for (key, members) in self.classes {
            let members = members.into_iter().collect::<Vec<_>>();
            let Some(representative) = members.first().copied() else {
                return Err(CatalogErrorV1::EmptyClass);
            };
            let class_key_sha256 = self
                .class_digests
                .get(&key)
                .copied()
                .ok_or(CatalogErrorV1::EmptyClass)?;
            classes.push(DynamicClassV1 {
                key,
                class_key_sha256,
                class_id: class_key_sha256,
                members,
                representative,
            });
        }
        classes.sort_by_key(|class| class.class_key_sha256);
        Ok(DynamicCatalogV1 {
            root: self.root,
            member_count: u64::try_from(self.static_members.len())
                .map_err(|_| CatalogErrorV1::CountOverflow)?,
            member_pair_set_sha256: digest_included_member_pair_set(
                self.static_members
                    .iter()
                    .map(|member| (member.case_key_sha256, member.full_record_sha256))
                    .collect(),
            ),
            projected_membership_sha256: digest_projected_membership_v1(
                self.root,
                self.projected_membership,
            ),
            descriptor_binding_sha256,
            classes,
        })
    }
}
