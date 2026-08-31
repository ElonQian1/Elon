use super::super::super::source_leaf_authority::digest_included_member_pair_set;
use super::super::descriptor_binding::{digest_descriptor_binding_v1, DescriptorBindingContextV1};
use super::super::membership_commitment::digest_projected_membership_v1;
use super::super::{runner_admission, DYNAMIC_PROJECTOR_SCHEMA_V1};
use super::*;

impl DynamicCatalogBuilderV1 {
    pub(crate) fn finish(mut self) -> Result<DynamicCatalogV1, CatalogErrorV1> {
        let descriptor_context = self.frozen_descriptor_binding.map_or(
            DescriptorBindingContextV1 {
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
        let runner_admission_binding_sha256 = runner_admission::digest_binding_v1(
            self.root,
            self.runner_admissions.values().copied(),
        );
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
                runner_admission_binding_sha256,
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
        let program_catalog_binding = match self.program_catalog_provider.take() {
            Some(provider) => Some(
                provider
                    .finish()
                    .map_err(CatalogErrorV1::ProgramCatalogAdmission)?,
            ),
            None => None,
        };
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
            runner_admission_binding_sha256,
            program_catalog_binding,
            classes,
        })
    }
}
