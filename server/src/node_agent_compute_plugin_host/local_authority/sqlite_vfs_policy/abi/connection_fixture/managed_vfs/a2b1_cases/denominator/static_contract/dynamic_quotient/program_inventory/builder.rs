use std::collections::{BTreeMap, BTreeSet};

use super::super::super::{
    invariants,
    model::ContractGraph,
    source_leaf_authority::{
        digest_included_member_pair_set, Digest32, FrozenStaticBindingV1, LeafSealOutcomeV1,
        RootOperationV1, StreamedLeafV1,
    },
    validate_source_owner_authority,
};
use super::super::{
    candidate::validate_frozen_pass,
    descriptor_binding::{
        checked_in_authority_v1, digest_descriptor_binding_v1, DescriptorBindingEntryV1,
        FrozenDescriptorBindingAuthorityV1,
    },
    digest_normalized_descriptor_semantics_v1,
    manifest_canonical::{
        digest_member_set_v1, digest_projector_schema_v1, digest_projector_source_scope_v1,
    },
    prepare_dynamic_terminal_v1,
    program_inventory_canonical::{
        digest_execution_program_catalog_v1, digest_execution_program_inventory_body_v1,
        digest_execution_program_inventory_source_scope_v1, digest_execution_program_membership_v1,
    },
    runner_admission::{
        self, ExecutionProgramInventoryReceiptV1, ExecutionProgramInventoryStatusV1,
    },
    DynamicClassKeyV1, StaticMemberSealV1,
};
use super::model::*;

struct ProgramGroupAccumulatorV1 {
    normalized_key: DynamicClassKeyV1,
    program_id: Digest32,
    plan_sha256: Digest32,
    status: ExecutionProgramInventoryStatusV1,
    members: BTreeSet<StaticMemberSealV1>,
}

struct ExecutionProgramInventoryBuilderV1 {
    root: RootOperationV1,
    static_members: BTreeSet<StaticMemberSealV1>,
    excluded_members: BTreeSet<StaticMemberSealV1>,
    descriptor_bindings: BTreeMap<StaticMemberSealV1, Digest32>,
    membership: BTreeMap<StaticMemberSealV1, Digest32>,
    groups: BTreeMap<DynamicClassKeyV1, ProgramGroupAccumulatorV1>,
    digest_owners: BTreeMap<Digest32, DynamicClassKeyV1>,
    frozen_descriptor_binding: FrozenDescriptorBindingAuthorityV1,
}

pub(in super::super) fn build_map_execution_program_inventory_v1(
    graph: &ContractGraph,
) -> Result<ExecutionProgramInventoryBundleV1, ExecutionProgramInventoryErrorV1> {
    build_execution_program_inventory_v1(graph, RootOperationV1::Map)
}

pub(in super::super) fn build_lock_execution_program_inventory_v1(
    graph: &ContractGraph,
) -> Result<ExecutionProgramInventoryBundleV1, ExecutionProgramInventoryErrorV1> {
    build_execution_program_inventory_v1(graph, RootOperationV1::Lock)
}

fn build_execution_program_inventory_v1(
    graph: &ContractGraph,
    root: RootOperationV1,
) -> Result<ExecutionProgramInventoryBundleV1, ExecutionProgramInventoryErrorV1> {
    validate_source_owner_authority().map_err(ExecutionProgramInventoryErrorV1::StaticIngress)?;
    let invariant_count = invariants::validate_graph(graph)
        .map_err(ExecutionProgramInventoryErrorV1::StaticIngress)?;
    let trusted_binding = validate_frozen_pass(graph, root, |_| Ok(()))
        .map_err(|error| ExecutionProgramInventoryErrorV1::StaticIngress(format!("{error:?}")))?;
    let invariant_count = u64::try_from(invariant_count)
        .map_err(|_| ExecutionProgramInventoryErrorV1::CountOverflow)?;
    if trusted_binding.included_count != invariant_count {
        return Err(ExecutionProgramInventoryErrorV1::StaticBindingDrift);
    }

    let mut builder = ExecutionProgramInventoryBuilderV1::new(&trusted_binding)?;
    let mut inventory_error = None;
    let observed_binding = validate_frozen_pass(graph, root, |leaf| {
        builder.observe(leaf).map_err(|error| {
            let message = format!("execution program inventory rejected a frozen leaf: {error:?}");
            inventory_error = Some(error);
            message
        })
    });
    if let Some(error) = inventory_error {
        return Err(error);
    }
    let observed_binding = observed_binding
        .map_err(|error| ExecutionProgramInventoryErrorV1::StaticIngress(format!("{error:?}")))?;
    if observed_binding != trusted_binding {
        return Err(ExecutionProgramInventoryErrorV1::StaticBindingDrift);
    }
    builder.finish(&trusted_binding)
}

impl ExecutionProgramInventoryBuilderV1 {
    fn new(binding: &FrozenStaticBindingV1) -> Result<Self, ExecutionProgramInventoryErrorV1> {
        let frozen_descriptor_binding = checked_in_authority_v1(binding)
            .map_err(ExecutionProgramInventoryErrorV1::DescriptorBindingContextDrift)?;
        Ok(Self {
            root: binding.context.root,
            static_members: BTreeSet::new(),
            excluded_members: BTreeSet::new(),
            descriptor_bindings: BTreeMap::new(),
            membership: BTreeMap::new(),
            groups: BTreeMap::new(),
            digest_owners: BTreeMap::new(),
            frozen_descriptor_binding,
        })
    }

    fn observe(
        &mut self,
        leaf: StreamedLeafV1<'_>,
    ) -> Result<(), ExecutionProgramInventoryErrorV1> {
        let root = leaf.seal().root;
        let outcome = leaf.seal().outcome;
        let member = StaticMemberSealV1 {
            case_key_sha256: leaf.seal().case_key_sha256,
            full_record_sha256: leaf.seal().full_record_sha256,
        };
        if root != self.root {
            return Err(ExecutionProgramInventoryErrorV1::RootMismatch);
        }
        match leaf {
            StreamedLeafV1::Excluded { .. } => self.observe_excluded(outcome, member),
            StreamedLeafV1::Terminal {
                record, descriptor, ..
            } => {
                if outcome != LeafSealOutcomeV1::Terminal {
                    return Err(ExecutionProgramInventoryErrorV1::OutcomeMismatch);
                }
                self.register_terminal(member)?;
                let prepared =
                    prepare_dynamic_terminal_v1(record, descriptor).map_err(|error| {
                        ExecutionProgramInventoryErrorV1::ProjectionFailed(
                            ExecutionProgramProjectionFailureV1 { member, error },
                        )
                    })?;
                if prepared.member != member || prepared.descriptor_binding.member != member {
                    return Err(
                        ExecutionProgramInventoryErrorV1::ProjectedMemberDigestMismatch(member),
                    );
                }
                if self
                    .descriptor_bindings
                    .insert(
                        member,
                        prepared.descriptor_binding.descriptor_semantic_sha256,
                    )
                    .is_some()
                {
                    return Err(ExecutionProgramInventoryErrorV1::DuplicateStaticMember(
                        member,
                    ));
                }
                let receipt = runner_admission::inventory_v1(&prepared.key).map_err(|error| {
                    ExecutionProgramInventoryErrorV1::ProgramInventoryAdmissionFailed {
                        member,
                        error,
                    }
                })?;
                self.observe_program(member, prepared.descriptor_binding, receipt)
            }
        }
    }

    fn observe_excluded(
        &mut self,
        outcome: LeafSealOutcomeV1,
        member: StaticMemberSealV1,
    ) -> Result<(), ExecutionProgramInventoryErrorV1> {
        if outcome != LeafSealOutcomeV1::Excluded {
            return Err(ExecutionProgramInventoryErrorV1::OutcomeMismatch);
        }
        if self.static_members.contains(&member) {
            return Err(ExecutionProgramInventoryErrorV1::ExcludedMemberProjected(
                member,
            ));
        }
        if !self.excluded_members.insert(member) {
            return Err(ExecutionProgramInventoryErrorV1::DuplicateStaticMember(
                member,
            ));
        }
        Ok(())
    }

    fn register_terminal(
        &mut self,
        member: StaticMemberSealV1,
    ) -> Result<(), ExecutionProgramInventoryErrorV1> {
        if self.excluded_members.contains(&member) {
            return Err(ExecutionProgramInventoryErrorV1::ExcludedMemberProjected(
                member,
            ));
        }
        if !self.static_members.insert(member) {
            return Err(ExecutionProgramInventoryErrorV1::DuplicateStaticMember(
                member,
            ));
        }
        Ok(())
    }

    fn observe_program(
        &mut self,
        member: StaticMemberSealV1,
        descriptor_binding: DescriptorBindingEntryV1,
        receipt: ExecutionProgramInventoryReceiptV1,
    ) -> Result<(), ExecutionProgramInventoryErrorV1> {
        let normalized_key = receipt.normalized_key();
        let program_id = receipt.program_id();
        let normalized_descriptor_sha256 = receipt.normalized_descriptor_sha256();
        if normalized_key.root != self.root
            || descriptor_binding.descriptor_semantic_sha256 != normalized_descriptor_sha256
            || digest_normalized_descriptor_semantics_v1(&normalized_key)
                != normalized_descriptor_sha256
            || runner_admission::execution_program_id_v1(
                normalized_key.root,
                normalized_descriptor_sha256,
                receipt.plan_sha256(),
            ) != program_id
        {
            return Err(ExecutionProgramInventoryErrorV1::ProgramIdentityMismatch(
                member,
            ));
        }
        if let Some(owner) = self.digest_owners.get(&program_id) {
            if owner != &normalized_key {
                return Err(ExecutionProgramInventoryErrorV1::ProgramDigestCollision(
                    program_id,
                ));
            }
        } else {
            self.digest_owners.insert(program_id, normalized_key);
        }
        if self.membership.insert(member, program_id).is_some() {
            return Err(ExecutionProgramInventoryErrorV1::DuplicateStaticMember(
                member,
            ));
        }
        let group =
            self.groups
                .entry(normalized_key)
                .or_insert_with(|| ProgramGroupAccumulatorV1 {
                    normalized_key,
                    program_id,
                    plan_sha256: receipt.plan_sha256(),
                    status: receipt.status(),
                    members: BTreeSet::new(),
                });
        if group.program_id != program_id
            || group.plan_sha256 != receipt.plan_sha256()
            || group.status != receipt.status()
        {
            return Err(ExecutionProgramInventoryErrorV1::ProgramContractMismatch(
                program_id,
            ));
        }
        if !group.members.insert(member) {
            return Err(ExecutionProgramInventoryErrorV1::DuplicateStaticMember(
                member,
            ));
        }
        Ok(())
    }

    fn finish(
        self,
        binding: &FrozenStaticBindingV1,
    ) -> Result<ExecutionProgramInventoryBundleV1, ExecutionProgramInventoryErrorV1> {
        self.validate_static_binding(binding)?;
        let member_count = u64::try_from(self.static_members.len())
            .map_err(|_| ExecutionProgramInventoryErrorV1::CountOverflow)?;
        let descriptor_binding_sha256 = self.validate_descriptor_binding()?;
        let root = self.root;
        let observed_membership = self.membership.clone();
        let (groups, counts) = self.finish_groups(member_count)?;
        let mut rebuilt_membership = BTreeMap::new();
        for group in &groups {
            for member in &group.members {
                if rebuilt_membership
                    .insert(*member, group.program_id)
                    .is_some()
                {
                    return Err(ExecutionProgramInventoryErrorV1::ProgramMembershipMismatch);
                }
            }
        }
        if rebuilt_membership != observed_membership {
            return Err(ExecutionProgramInventoryErrorV1::ProgramMembershipMismatch);
        }
        let mut reverse_index = rebuilt_membership
            .into_iter()
            .map(|(member, program_id)| ExecutionProgramMembershipV1 { member, program_id })
            .collect::<Vec<_>>();
        reverse_index.sort_unstable();
        let membership_sha256 = digest_execution_program_membership_v1(root, &reverse_index);
        let program_catalog_sha256 = digest_execution_program_catalog_v1(root, &groups);
        let context = ExecutionProgramInventoryContextV1 {
            schema_version: EXECUTION_PROGRAM_INVENTORY_SCHEMA_V1,
            root,
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
            descriptor_binding_sha256,
            inventory_source_scope_sha256: digest_execution_program_inventory_source_scope_v1(),
        };
        let mut inventory = ExecutionProgramInventoryV1 {
            context,
            member_count,
            program_group_count: counts.group_count,
            source_present_member_count: counts.source_present_members,
            source_present_group_count: counts.source_present_groups,
            planned_missing_member_count: counts.planned_missing_members,
            planned_missing_group_count: counts.planned_missing_groups,
            membership_sha256,
            program_catalog_sha256,
            inventory_sha256: Digest32::ZERO,
        };
        inventory.inventory_sha256 = digest_execution_program_inventory_body_v1(&inventory);
        Ok(ExecutionProgramInventoryBundleV1 {
            inventory,
            groups,
            reverse_index,
        })
    }

    fn validate_static_binding(
        &self,
        binding: &FrozenStaticBindingV1,
    ) -> Result<(), ExecutionProgramInventoryErrorV1> {
        if binding.context.root != self.root
            || binding.included_count.checked_add(binding.excluded_count)
                != Some(binding.source_universe_count)
        {
            return Err(ExecutionProgramInventoryErrorV1::StaticUniverseMismatch);
        }
        if u64::try_from(self.static_members.len()).ok() != Some(binding.included_count)
            || self.membership.len() != self.static_members.len()
        {
            return Err(ExecutionProgramInventoryErrorV1::StaticMemberCountMismatch);
        }
        if u64::try_from(self.excluded_members.len()).ok() != Some(binding.excluded_count) {
            return Err(ExecutionProgramInventoryErrorV1::StaticExcludedCountMismatch);
        }
        let pair_set = digest_included_member_pair_set(
            self.static_members
                .iter()
                .map(|member| (member.case_key_sha256, member.full_record_sha256))
                .collect(),
        );
        if pair_set != binding.included_member_pair_set_sha256 {
            return Err(ExecutionProgramInventoryErrorV1::StaticMemberSetMismatch);
        }
        Ok(())
    }

    fn validate_descriptor_binding(&self) -> Result<Digest32, ExecutionProgramInventoryErrorV1> {
        let actual = digest_descriptor_binding_v1(
            self.frozen_descriptor_binding.context,
            self.descriptor_bindings
                .iter()
                .map(|(member, digest)| DescriptorBindingEntryV1 {
                    member: *member,
                    descriptor_semantic_sha256: *digest,
                }),
        );
        let expected = self.frozen_descriptor_binding.descriptor_binding_sha256;
        if actual != expected {
            return Err(
                ExecutionProgramInventoryErrorV1::DescriptorBindingCommitmentDrift {
                    expected,
                    actual,
                },
            );
        }
        Ok(actual)
    }

    fn finish_groups(
        self,
        member_count: u64,
    ) -> Result<(Vec<ExecutionProgramGroupV1>, InventoryCountsV1), ExecutionProgramInventoryErrorV1>
    {
        let mut counts = InventoryCountsV1::default();
        let mut groups = Vec::with_capacity(self.groups.len());
        for group in self.groups.into_values() {
            if group.members.is_empty() {
                return Err(ExecutionProgramInventoryErrorV1::EmptyProgramGroup);
            }
            let members = group.members.into_iter().collect::<Vec<_>>();
            let group_member_count = u64::try_from(members.len())
                .map_err(|_| ExecutionProgramInventoryErrorV1::CountOverflow)?;
            counts.observe(group.status, group_member_count)?;
            groups.push(ExecutionProgramGroupV1 {
                normalized_key: group.normalized_key,
                program_id: group.program_id,
                plan_sha256: group.plan_sha256,
                status: group.status,
                member_count: group_member_count,
                member_set_sha256: digest_member_set_v1(&members),
                members,
            });
        }
        groups.sort_by_key(|group| group.program_id);
        counts.group_count = u64::try_from(groups.len())
            .map_err(|_| ExecutionProgramInventoryErrorV1::CountOverflow)?;
        if counts
            .source_present_members
            .checked_add(counts.planned_missing_members)
            != Some(member_count)
            || counts
                .source_present_groups
                .checked_add(counts.planned_missing_groups)
                != Some(counts.group_count)
        {
            return Err(ExecutionProgramInventoryErrorV1::StaticMemberCountMismatch);
        }
        Ok((groups, counts))
    }
}

#[derive(Default)]
struct InventoryCountsV1 {
    group_count: u64,
    source_present_members: u64,
    source_present_groups: u64,
    planned_missing_members: u64,
    planned_missing_groups: u64,
}

impl InventoryCountsV1 {
    fn observe(
        &mut self,
        status: ExecutionProgramInventoryStatusV1,
        member_count: u64,
    ) -> Result<(), ExecutionProgramInventoryErrorV1> {
        let (groups, members) = match status {
            ExecutionProgramInventoryStatusV1::PlannedMissing(_) => (
                &mut self.planned_missing_groups,
                &mut self.planned_missing_members,
            ),
            ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. } => (
                &mut self.source_present_groups,
                &mut self.source_present_members,
            ),
        };
        *groups = groups
            .checked_add(1)
            .ok_or(ExecutionProgramInventoryErrorV1::CountOverflow)?;
        *members = members
            .checked_add(member_count)
            .ok_or(ExecutionProgramInventoryErrorV1::CountOverflow)?;
        Ok(())
    }
}
