//! Atomic 36-member evidence gate for the Windows JointClose family.
//!
//! Individual candidates and members are deliberately non-renderable. Formal output exists only
//! after the reducer consumes the exact selector set, one shared cohort and an exact-clean-commit
//! receipt.

use std::{collections::BTreeMap, fmt, fmt::Write as _};

use sha2::{Digest, Sha256};

use super::super::a2b2_cases::JointCloseSelector;

mod checkout;
mod digest;
mod member;
#[cfg(test)]
mod tests;

pub(in super::super) use checkout::ValidatedJointCloseCleanCheckoutReceipt;
use digest::validate_family_facts;
pub(in super::super) use member::{
    JointCloseCandidateReportView, ValidatedJointCloseCandidateRecord,
    ValidatedJointCloseFamilyMemberReceipt,
};

pub(super) const MEMBER_COUNT: usize = 36;
const FORMAL_RECORD_MARKER: &str = "A2_WINDOWS_DYNAMIC_V2";
const FAMILY_MARKER: &str = "A2_JOINT_CLOSE_WINDOWS_DYNAMIC_FAMILY_V1";

/// Parent-local identity shared by every child in one atomic family attempt.
pub(in super::super) struct JointCloseFamilyCohort {
    nonce: String,
    pub(super) commitment: [u8; 32],
}

/// Complete formal JointClose family. Only the exact reducer can construct it.
#[must_use = "the complete JointClose family must be rendered atomically"]
pub(in super::super) struct ValidatedJointCloseFamily {
    members: Box<[ValidatedJointCloseFamilyMemberReceipt; MEMBER_COUNT]>,
    _cohort: JointCloseFamilyCohort,
    _checkout: ValidatedJointCloseCleanCheckoutReceipt,
    cohort_digest: [u8; 32],
    family_seal: [u8; 32],
    clean_commit_fingerprint: [u8; 32],
}

/// Fully materialized 36/36 output. Partial member sets have no printable formal-record API.
pub(in super::super) struct RenderedJointCloseFamilyReport {
    text: String,
    clean_commit_fingerprint: String,
}

impl JointCloseFamilyCohort {
    pub(in super::super) fn new() -> Self {
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let commitment =
            domain_commitment(b"elon-a2-joint-close-family-cohort-v1\0", nonce.as_bytes());
        Self { nonce, commitment }
    }

    pub(super) fn is_valid(&self) -> bool {
        self.nonce.len() == 32
            && self
                .nonce
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            && self.commitment
                == domain_commitment(
                    b"elon-a2-joint-close-family-cohort-v1\0",
                    self.nonce.as_bytes(),
                )
    }
}

impl ValidatedJointCloseFamily {
    /// Atomically consumes exactly one sealed member for every frozen JointClose selector.
    pub(in super::super) fn reduce(
        cohort: JointCloseFamilyCohort,
        members: Vec<ValidatedJointCloseFamilyMemberReceipt>,
        checkout: ValidatedJointCloseCleanCheckoutReceipt,
    ) -> Result<Self, &'static str> {
        if !cohort.is_valid() {
            return Err("A2_JOINT_CLOSE_FAMILY_COHORT_INVALID");
        }
        if checkout.cohort_commitment != cohort.commitment {
            return Err("A2_JOINT_CLOSE_FAMILY_CHECKOUT_COHORT_MISMATCH");
        }
        for member in &members {
            member.validate_integrity()?;
        }
        let facts = members
            .iter()
            .map(|member| member.fact())
            .collect::<Vec<_>>();
        let digests = validate_family_facts(&facts, cohort.commitment, &checkout.git_sha)?;

        let mut by_selector = BTreeMap::new();
        for member in members {
            if by_selector.insert(member.selector, member).is_some() {
                return Err("A2_JOINT_CLOSE_FAMILY_SELECTOR_DUPLICATE");
            }
        }
        let mut ordered = Vec::with_capacity(MEMBER_COUNT);
        for selector in JointCloseSelector::ALL {
            ordered.push(
                by_selector
                    .remove(&selector)
                    .ok_or("A2_JOINT_CLOSE_FAMILY_SELECTOR_MISSING")?,
            );
        }
        if !by_selector.is_empty() {
            return Err("A2_JOINT_CLOSE_FAMILY_SELECTOR_UNKNOWN");
        }
        let members = Box::new(
            ordered
                .try_into()
                .map_err(|_| "A2_JOINT_CLOSE_FAMILY_MEMBER_COUNT_INVALID")?,
        );
        Ok(Self {
            members,
            _cohort: cohort,
            _checkout: checkout,
            cohort_digest: digests.cohort,
            family_seal: digests.seal,
            clean_commit_fingerprint: digests.clean_commit_fingerprint,
        })
    }

    /// Materializes 36 formal records and the family seal as one indivisible value.
    pub(in super::super) fn render_atomic(self) -> RenderedJointCloseFamilyReport {
        let clean_commit_fingerprint = opaque_commitment(&self.clean_commit_fingerprint);
        let mut text = String::new();
        for member in self.members.iter() {
            member.write_formal_record(&mut text, FORMAL_RECORD_MARKER);
        }
        let _ = write!(
            text,
            "{FAMILY_MARKER} cases={MEMBER_COUNT}/{MEMBER_COUNT} commit={} cohort={} seal={} clean_commit_fingerprint={} checkout=clean",
            self.members[0].environment.git_sha,
            opaque_commitment(&self.cohort_digest),
            opaque_commitment(&self.family_seal),
            clean_commit_fingerprint,
        );
        RenderedJointCloseFamilyReport {
            text,
            clean_commit_fingerprint,
        }
    }
}

impl RenderedJointCloseFamilyReport {
    pub(in super::super) fn as_str(&self) -> &str {
        &self.text
    }

    /// Stable for the same clean commit, platform tuple and ordered canonical payloads.
    pub(in super::super) fn clean_commit_fingerprint(&self) -> &str {
        &self.clean_commit_fingerprint
    }
}

impl fmt::Display for RenderedJointCloseFamilyReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

pub(super) fn opaque_commitment(value: &[u8; 32]) -> String {
    format!("sha256:{}", hex::encode(value))
}

fn domain_commitment(domain: &[u8], value: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value);
    hasher.finalize().into()
}
