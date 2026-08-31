use sha2::{Digest as _, Sha256};

use super::super::super::super::source_leaf_authority::{Digest32, RootOperationV1};
use super::ProgramCatalogAdmissionReceiptV1;

const ADMISSION_BINDING_DOMAIN: &str =
    "ELON-A2-MAP-LOCK-PROGRAM-CATALOG-SOURCE-ADMISSION-BINDING-V1";

pub(super) fn digest_program_catalog_admission_binding_v1(
    root: RootOperationV1,
    inventory_sha256: Digest32,
    receipts: impl IntoIterator<Item = ProgramCatalogAdmissionReceiptV1>,
) -> Digest32 {
    let mut receipts = receipts.into_iter().collect::<Vec<_>>();
    receipts.sort_by_key(|receipt| receipt.member);
    let mut out = Sha256::new();
    add_bytes(&mut out, "domain", ADMISSION_BINDING_DOMAIN.as_bytes());
    add_bytes(&mut out, "root", root.canonical_name().as_bytes());
    add_digest(&mut out, "inventory_sha256", inventory_sha256);
    add_u64(&mut out, "receipt_count", receipts.len() as u64);
    for receipt in receipts {
        add_digest(&mut out, "case_key_sha256", receipt.member.case_key_sha256);
        add_digest(
            &mut out,
            "full_record_sha256",
            receipt.member.full_record_sha256,
        );
        add_digest(
            &mut out,
            "normalized_descriptor_sha256",
            receipt.normalized_descriptor_sha256,
        );
        add_digest(&mut out, "program_id", receipt.program_id);
        add_digest(&mut out, "plan_sha256", receipt.plan_sha256);
        add_digest(
            &mut out,
            "implementation_sha256",
            receipt.implementation_sha256,
        );
        add_digest(
            &mut out,
            "receipt_inventory_sha256",
            receipt.inventory_sha256,
        );
    }
    Digest32(out.finalize().into())
}

fn add_digest(out: &mut Sha256, label: &str, value: Digest32) {
    add_bytes(out, label, &value.0);
}

fn add_u64(out: &mut Sha256, label: &str, value: u64) {
    add_bytes(out, label, &value.to_le_bytes());
}

fn add_bytes(out: &mut Sha256, label: &str, value: &[u8]) {
    out.update((label.len() as u64).to_le_bytes());
    out.update(label.as_bytes());
    out.update((value.len() as u64).to_le_bytes());
    out.update(value);
}
