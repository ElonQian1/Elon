//! Canonical commitment to the projector-produced member-to-class assignment.
//!
//! Static member-set equality proves coverage, but not the exact quotient partition. This
//! commitment binds every complete static member seal to the class-key digest produced for that
//! member. Catalog construction records it directly from projector output; manifest construction
//! independently recomputes it from the emitted class membership.

use sha2::{Digest as _, Sha256};

use super::super::source_leaf_authority::{Digest32, RootOperationV1};
use super::{StaticMemberSealV1, DYNAMIC_PROJECTOR_SCHEMA_V1};

const PROJECTED_MEMBERSHIP_DOMAIN_V1: &str = "ELON-A2-MAP-LOCK-DYNAMIC-PROJECTED-MEMBERSHIP-V1";

pub(super) fn digest_projected_membership_v1(
    root: RootOperationV1,
    entries: impl IntoIterator<Item = (StaticMemberSealV1, Digest32)>,
) -> Digest32 {
    let mut entries = entries.into_iter().collect::<Vec<_>>();
    entries.sort_unstable();

    let mut out = Sha256::new();
    add_bytes(
        &mut out,
        "domain",
        PROJECTED_MEMBERSHIP_DOMAIN_V1.as_bytes(),
    );
    add_bytes(
        &mut out,
        "schema_version",
        &DYNAMIC_PROJECTOR_SCHEMA_V1.to_be_bytes(),
    );
    add_bytes(&mut out, "root", root.canonical_name().as_bytes());
    add_bytes(
        &mut out,
        "entry_count",
        &(entries.len() as u64).to_be_bytes(),
    );
    for (member, class_key_sha256) in entries {
        let mut member_bytes = [0_u8; 64];
        member_bytes[..32].copy_from_slice(&member.case_key_sha256.0);
        member_bytes[32..].copy_from_slice(&member.full_record_sha256.0);
        add_bytes(&mut out, "member", &member_bytes);
        add_bytes(&mut out, "class_key_sha256", &class_key_sha256.0);
    }
    Digest32(out.finalize().into())
}

fn add_bytes(out: &mut Sha256, label: &str, value: &[u8]) {
    out.update(label.as_bytes());
    out.update([0]);
    out.update((value.len() as u64).to_be_bytes());
    out.update(value);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(case: u8, full: u8) -> StaticMemberSealV1 {
        StaticMemberSealV1 {
            case_key_sha256: Digest32([case; 32]),
            full_record_sha256: Digest32([full; 32]),
        }
    }

    #[test]
    fn projected_membership_commitment_is_order_independent() {
        let first = (member(1, 2), Digest32([3; 32]));
        let second = (member(4, 5), Digest32([6; 32]));
        assert_eq!(
            digest_projected_membership_v1(RootOperationV1::Map, [first, second]),
            digest_projected_membership_v1(RootOperationV1::Map, [second, first]),
        );
    }

    #[test]
    fn projected_membership_commitment_rejects_swap_merge_and_root_drift() {
        let first = member(1, 2);
        let second = member(4, 5);
        let class_a = Digest32([3; 32]);
        let class_b = Digest32([6; 32]);
        let baseline = digest_projected_membership_v1(
            RootOperationV1::Map,
            [(first, class_a), (second, class_b)],
        );

        assert_ne!(
            baseline,
            digest_projected_membership_v1(
                RootOperationV1::Map,
                [(first, class_b), (second, class_a)],
            ),
        );
        assert_ne!(
            baseline,
            digest_projected_membership_v1(
                RootOperationV1::Map,
                [(first, class_a), (second, class_a)],
            ),
        );
        assert_ne!(
            baseline,
            digest_projected_membership_v1(
                RootOperationV1::Lock,
                [(first, class_a), (second, class_b)],
            ),
        );
    }
}
