use std::collections::{BTreeMap, BTreeSet};

use super::super::super::super::source_leaf_authority::{
    digest_included_member_pair_set, Digest32, FrozenStaticBindingV1, RootOperationV1,
};
use super::super::super::{
    descriptor_binding::checked_in_authority_v1,
    digest_normalized_descriptor_semantics_v1,
    manifest_canonical::{
        digest_member_set_v1, digest_projector_schema_v1, digest_projector_source_scope_v1,
    },
    program_inventory_canonical::{
        digest_execution_program_catalog_v1, digest_execution_program_inventory_body_v1,
        digest_execution_program_inventory_source_scope_v1, digest_execution_program_membership_v1,
    },
    runner_admission::{self, ExecutionProgramInventoryStatusV1},
    StaticMemberSealV1,
};
use super::super::{
    ExecutionProgramInventoryBundleV1, ExecutionProgramMembershipV1,
    EXECUTION_PROGRAM_INVENTORY_SCHEMA_V1,
};
use super::{
    ProgramCatalogAdmissionErrorV1, ProgramCatalogAdmissionReceiptV1, ProgramCatalogBindingV1,
};

pub(super) struct ValidatedCompleteInventoryV1 {
    pub(super) root: RootOperationV1,
    pub(super) binding: ProgramCatalogBindingV1,
    pub(super) receipts: BTreeMap<StaticMemberSealV1, ProgramCatalogAdmissionReceiptV1>,
}

pub(super) fn validate_complete_inventory_v1(
    bundle: ExecutionProgramInventoryBundleV1,
    binding: &FrozenStaticBindingV1,
) -> Result<ValidatedCompleteInventoryV1, ProgramCatalogAdmissionErrorV1> {
    let inventory = &bundle.inventory;
    let context = &inventory.context;
    if context.schema_version != EXECUTION_PROGRAM_INVENTORY_SCHEMA_V1
        || context.root != RootOperationV1::Map
        || binding.context.root != RootOperationV1::Map
    {
        return Err(ProgramCatalogAdmissionErrorV1::RootMismatch);
    }
    let descriptor_authority = checked_in_authority_v1(binding)
        .map_err(|_| ProgramCatalogAdmissionErrorV1::StaticContextMismatch)?;
    if context.static_source_baseline_sha1 != binding.context.source_baseline_commit_sha1
        || context.static_source_scope_sha256 != binding.context.source_scope_sha256
        || context.static_ledger_sha256 != binding.context.ledger_sha256
        || context.static_manifest_sha256 != binding.static_manifest_sha256
        || context.static_member_pair_set_sha256 != binding.included_member_pair_set_sha256
        || context.static_included_count != binding.included_count
        || context.static_excluded_count != binding.excluded_count
        || context.static_source_universe_count != binding.source_universe_count
        || context.descriptor_binding_sha256 != descriptor_authority.descriptor_binding_sha256
    {
        return Err(ProgramCatalogAdmissionErrorV1::StaticContextMismatch);
    }
    if context.projector_schema_sha256 != digest_projector_schema_v1()
        || context.projector_source_scope_sha256 != digest_projector_source_scope_v1()
        || context.inventory_source_scope_sha256
            != digest_execution_program_inventory_source_scope_v1()
    {
        return Err(ProgramCatalogAdmissionErrorV1::ProjectorContextMismatch);
    }
    if digest_execution_program_inventory_body_v1(inventory) != inventory.inventory_sha256 {
        return Err(ProgramCatalogAdmissionErrorV1::InventoryDigestMismatch);
    }

    let mut program_ids = BTreeSet::new();
    let mut members = BTreeSet::new();
    let mut rebuilt_index = Vec::new();
    let mut receipts = BTreeMap::new();
    let mut source_members = 0_u64;
    let mut source_groups = 0_u64;
    let mut missing_members = 0_u64;
    let mut missing_groups = 0_u64;
    let mut first_missing = None;
    for group in &bundle.groups {
        if group.members.is_empty() {
            return Err(ProgramCatalogAdmissionErrorV1::EmptyProgram(
                group.program_id,
            ));
        }
        if !program_ids.insert(group.program_id) {
            return Err(ProgramCatalogAdmissionErrorV1::DuplicateProgram(
                group.program_id,
            ));
        }
        if group.normalized_key.root != context.root
            || group.member_count != checked_len(group.members.len())?
            || !group.members.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err(ProgramCatalogAdmissionErrorV1::ProgramContractMismatch(
                group.program_id,
            ));
        }
        if digest_member_set_v1(&group.members) != group.member_set_sha256 {
            return Err(ProgramCatalogAdmissionErrorV1::ProgramMemberSetMismatch(
                group.program_id,
            ));
        }
        let current = runner_admission::inventory_v1(&group.normalized_key).map_err(|_| {
            ProgramCatalogAdmissionErrorV1::ProgramContractMismatch(group.program_id)
        })?;
        let normalized_descriptor_sha256 =
            digest_normalized_descriptor_semantics_v1(&group.normalized_key);
        if current.normalized_key() != group.normalized_key
            || current.normalized_descriptor_sha256() != normalized_descriptor_sha256
            || current.program_id() != group.program_id
            || current.plan_sha256() != group.plan_sha256
            || current.status() != group.status
            || runner_admission::execution_program_id_v1(
                context.root,
                normalized_descriptor_sha256,
                group.plan_sha256,
            ) != group.program_id
        {
            return Err(ProgramCatalogAdmissionErrorV1::ProgramIdentityMismatch(
                group.program_id,
            ));
        }

        let implementation_sha256 = match group.status {
            ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired {
                implementation_sha256,
            } => {
                source_groups = checked_add(source_groups, 1)?;
                source_members = checked_add(source_members, group.member_count)?;
                Some(implementation_sha256)
            }
            ExecutionProgramInventoryStatusV1::PlannedMissing(_) => {
                missing_groups = checked_add(missing_groups, 1)?;
                missing_members = checked_add(missing_members, group.member_count)?;
                first_missing.get_or_insert(group.program_id);
                None
            }
        };
        for member in &group.members {
            if !members.insert(*member) {
                return Err(ProgramCatalogAdmissionErrorV1::DuplicateMember(*member));
            }
            rebuilt_index.push(ExecutionProgramMembershipV1 {
                member: *member,
                program_id: group.program_id,
            });
            if let Some(implementation_sha256) = implementation_sha256 {
                let receipt = ProgramCatalogAdmissionReceiptV1 {
                    member: *member,
                    normalized_key: group.normalized_key,
                    normalized_descriptor_sha256,
                    program_id: group.program_id,
                    plan_sha256: group.plan_sha256,
                    implementation_sha256,
                    inventory_sha256: inventory.inventory_sha256,
                };
                if receipts.insert(*member, receipt).is_some() {
                    return Err(ProgramCatalogAdmissionErrorV1::DuplicateMember(*member));
                }
            }
        }
    }

    rebuilt_index.sort_unstable();
    if rebuilt_index != bundle.reverse_index
        || digest_execution_program_membership_v1(context.root, &rebuilt_index)
            != inventory.membership_sha256
    {
        return Err(ProgramCatalogAdmissionErrorV1::InventoryMembershipMismatch);
    }
    if digest_execution_program_catalog_v1(context.root, &bundle.groups)
        != inventory.program_catalog_sha256
    {
        return Err(ProgramCatalogAdmissionErrorV1::InventoryCatalogMismatch);
    }
    let member_pair_set_sha256 = digest_included_member_pair_set(
        members
            .iter()
            .map(|member| (member.case_key_sha256, member.full_record_sha256))
            .collect(),
    );
    if member_pair_set_sha256 != binding.included_member_pair_set_sha256
        || checked_len(members.len())? != inventory.member_count
        || checked_len(bundle.groups.len())? != inventory.program_group_count
        || source_members != inventory.source_present_member_count
        || source_groups != inventory.source_present_group_count
        || missing_members != inventory.planned_missing_member_count
        || missing_groups != inventory.planned_missing_group_count
        || inventory.member_count != binding.included_count
        || source_members.checked_add(missing_members) != Some(inventory.member_count)
        || source_groups.checked_add(missing_groups) != Some(inventory.program_group_count)
    {
        return Err(ProgramCatalogAdmissionErrorV1::InventoryCountMismatch);
    }
    if missing_members != 0 || missing_groups != 0 {
        return Err(ProgramCatalogAdmissionErrorV1::PlannedProgramsMissing {
            member_count: missing_members,
            group_count: missing_groups,
            first_program_id: first_missing.unwrap_or(Digest32::ZERO),
        });
    }
    if checked_len(receipts.len())? != inventory.member_count {
        return Err(ProgramCatalogAdmissionErrorV1::InventoryCountMismatch);
    }

    Ok(ValidatedCompleteInventoryV1 {
        root: context.root,
        binding: ProgramCatalogBindingV1 {
            inventory_sha256: inventory.inventory_sha256,
            inventory_membership_sha256: inventory.membership_sha256,
            inventory_catalog_sha256: inventory.program_catalog_sha256,
            admission_binding_sha256: Digest32::ZERO,
        },
        receipts,
    })
}

fn checked_len(value: usize) -> Result<u64, ProgramCatalogAdmissionErrorV1> {
    u64::try_from(value).map_err(|_| ProgramCatalogAdmissionErrorV1::InventoryCountMismatch)
}

fn checked_add(left: u64, right: u64) -> Result<u64, ProgramCatalogAdmissionErrorV1> {
    left.checked_add(right)
        .ok_or(ProgramCatalogAdmissionErrorV1::InventoryCountMismatch)
}
