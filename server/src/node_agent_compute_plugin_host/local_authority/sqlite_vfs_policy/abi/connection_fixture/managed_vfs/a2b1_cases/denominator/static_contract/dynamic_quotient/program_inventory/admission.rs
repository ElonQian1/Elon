//! Reviewed pre-manifest source-program admission.
//!
//! This authority is deliberately separate from runner execution admission. It may authorize a
//! complete semantic catalog only after every inventory program is source-present and the exact
//! inventory digest has been independently frozen. It cannot create `Supported`, execute a child,
//! or stand in for a post-manifest Windows receipt.

use std::collections::{BTreeMap, BTreeSet};

mod canonical;
mod validation;

use super::super::super::source_leaf_authority::{
    Digest32, FrozenStaticBindingV1, RootOperationV1,
};
use super::super::{
    digest_normalized_descriptor_semantics_v1,
    runner_admission::{self, ExecutionProgramInventoryStatusV1},
    DynamicClassKeyV1, StaticMemberSealV1,
};
use super::review::REVIEWED_MAP_EXECUTION_PROGRAM_INVENTORY_SHA256_V1;
use super::ExecutionProgramInventoryBundleV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ProgramCatalogAdmissionReceiptV1 {
    member: StaticMemberSealV1,
    normalized_key: DynamicClassKeyV1,
    normalized_descriptor_sha256: Digest32,
    program_id: Digest32,
    plan_sha256: Digest32,
    implementation_sha256: Digest32,
    inventory_sha256: Digest32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ProgramCatalogBindingV1 {
    pub(in super::super) inventory_sha256: Digest32,
    pub(in super::super) inventory_membership_sha256: Digest32,
    pub(in super::super) inventory_catalog_sha256: Digest32,
    pub(in super::super) admission_binding_sha256: Digest32,
}

pub(in super::super) struct ReviewedExecutionProgramInventoryV1 {
    provider: ProgramCatalogReceiptProviderV1,
}

pub(in super::super) struct ProgramCatalogReceiptProviderV1 {
    root: RootOperationV1,
    inventory_sha256: Digest32,
    all_members: BTreeSet<StaticMemberSealV1>,
    remaining: BTreeMap<StaticMemberSealV1, ProgramCatalogAdmissionReceiptV1>,
    binding: ProgramCatalogBindingV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProgramCatalogAdmissionErrorV1 {
    RootMismatch,
    StaticContextMismatch,
    ProjectorContextMismatch,
    InventoryDigestMismatch,
    InventoryMembershipMismatch,
    InventoryCatalogMismatch,
    InventoryCountMismatch,
    DuplicateProgram(Digest32),
    EmptyProgram(Digest32),
    ProgramIdentityMismatch(Digest32),
    ProgramContractMismatch(Digest32),
    ProgramMemberSetMismatch(Digest32),
    DuplicateMember(StaticMemberSealV1),
    PlannedProgramsMissing {
        member_count: u64,
        group_count: u64,
        first_program_id: Digest32,
    },
    ReviewNotFrozen {
        inventory_sha256: Digest32,
    },
    ReviewDigestMismatch {
        expected: Digest32,
        actual: Digest32,
    },
    ReceiptMissing(StaticMemberSealV1),
    ReceiptAlreadyConsumed(StaticMemberSealV1),
    ReceiptBindingMismatch(StaticMemberSealV1),
    CurrentClassifierMismatch(StaticMemberSealV1),
    UnconsumedReceipts(u64),
}

pub(in super::super) fn review_map_execution_program_inventory_v1(
    bundle: ExecutionProgramInventoryBundleV1,
    binding: &FrozenStaticBindingV1,
) -> Result<ReviewedExecutionProgramInventoryV1, ProgramCatalogAdmissionErrorV1> {
    let validated = validation::validate_complete_inventory_v1(bundle, binding)?;
    let actual = validated.binding.inventory_sha256;
    let Some(expected) = REVIEWED_MAP_EXECUTION_PROGRAM_INVENTORY_SHA256_V1 else {
        return Err(ProgramCatalogAdmissionErrorV1::ReviewNotFrozen {
            inventory_sha256: actual,
        });
    };
    if expected != actual {
        return Err(ProgramCatalogAdmissionErrorV1::ReviewDigestMismatch { expected, actual });
    }
    Ok(ReviewedExecutionProgramInventoryV1 {
        provider: ProgramCatalogReceiptProviderV1::new(validated),
    })
}

#[cfg(test)]
pub(in super::super) fn provider_for_source_program_for_test(
    member: StaticMemberSealV1,
    key: &DynamicClassKeyV1,
) -> Result<ProgramCatalogReceiptProviderV1, ProgramCatalogAdmissionErrorV1> {
    let current = runner_admission::inventory_v1(key)
        .map_err(|_| ProgramCatalogAdmissionErrorV1::CurrentClassifierMismatch(member))?;
    let implementation_sha256 = match current.status() {
        ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired {
            implementation_sha256,
        } => implementation_sha256,
        ExecutionProgramInventoryStatusV1::PlannedMissing(_) => {
            return Err(ProgramCatalogAdmissionErrorV1::CurrentClassifierMismatch(
                member,
            ))
        }
    };
    let inventory_sha256 = Digest32([0xa1; 32]);
    let receipt = ProgramCatalogAdmissionReceiptV1 {
        member,
        normalized_key: current.normalized_key(),
        normalized_descriptor_sha256: current.normalized_descriptor_sha256(),
        program_id: current.program_id(),
        plan_sha256: current.plan_sha256(),
        implementation_sha256,
        inventory_sha256,
    };
    Ok(ProgramCatalogReceiptProviderV1::new(
        validation::ValidatedCompleteInventoryV1 {
            root: key.root,
            binding: ProgramCatalogBindingV1 {
                inventory_sha256,
                inventory_membership_sha256: Digest32([0xa2; 32]),
                inventory_catalog_sha256: Digest32([0xa3; 32]),
                admission_binding_sha256: Digest32::ZERO,
            },
            receipts: [(member, receipt)].into_iter().collect(),
        },
    ))
}

impl ReviewedExecutionProgramInventoryV1 {
    pub(in super::super) fn into_provider(self) -> ProgramCatalogReceiptProviderV1 {
        self.provider
    }
}

impl ProgramCatalogReceiptProviderV1 {
    fn new(validated: validation::ValidatedCompleteInventoryV1) -> Self {
        let all_members = validated.receipts.keys().copied().collect::<BTreeSet<_>>();
        let admission_binding_sha256 = canonical::digest_program_catalog_admission_binding_v1(
            validated.root,
            validated.binding.inventory_sha256,
            validated.receipts.values().copied(),
        );
        Self {
            root: validated.root,
            inventory_sha256: validated.binding.inventory_sha256,
            all_members,
            remaining: validated.receipts,
            binding: ProgramCatalogBindingV1 {
                admission_binding_sha256,
                ..validated.binding
            },
        }
    }

    pub(in super::super) fn take_for(
        &mut self,
        member: StaticMemberSealV1,
        key: &DynamicClassKeyV1,
    ) -> Result<ProgramCatalogAdmissionReceiptV1, ProgramCatalogAdmissionErrorV1> {
        if key.root != self.root {
            return Err(ProgramCatalogAdmissionErrorV1::RootMismatch);
        }
        let Some(receipt) = self.remaining.remove(&member) else {
            return Err(if self.all_members.contains(&member) {
                ProgramCatalogAdmissionErrorV1::ReceiptAlreadyConsumed(member)
            } else {
                ProgramCatalogAdmissionErrorV1::ReceiptMissing(member)
            });
        };
        let current = runner_admission::inventory_v1(key)
            .map_err(|_| ProgramCatalogAdmissionErrorV1::CurrentClassifierMismatch(member))?;
        let current_implementation = match current.status() {
            ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired {
                implementation_sha256,
            } => implementation_sha256,
            ExecutionProgramInventoryStatusV1::PlannedMissing(_) => {
                return Err(ProgramCatalogAdmissionErrorV1::CurrentClassifierMismatch(
                    member,
                ))
            }
        };
        if receipt.member != member
            || receipt.normalized_key != current.normalized_key()
            || receipt.normalized_descriptor_sha256
                != digest_normalized_descriptor_semantics_v1(key)
            || receipt.normalized_descriptor_sha256 != current.normalized_descriptor_sha256()
            || receipt.program_id != current.program_id()
            || receipt.plan_sha256 != current.plan_sha256()
            || receipt.implementation_sha256 != current_implementation
            || receipt.inventory_sha256 != self.inventory_sha256
        {
            return Err(ProgramCatalogAdmissionErrorV1::ReceiptBindingMismatch(
                member,
            ));
        }
        Ok(receipt)
    }

    pub(in super::super) fn finish(
        self,
    ) -> Result<ProgramCatalogBindingV1, ProgramCatalogAdmissionErrorV1> {
        if !self.remaining.is_empty() {
            return Err(ProgramCatalogAdmissionErrorV1::UnconsumedReceipts(
                u64::try_from(self.remaining.len()).unwrap_or(u64::MAX),
            ));
        }
        Ok(self.binding)
    }
}
