use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use super::super::super::{
    a2_dynamic_evidence::{child::SanitizedPayloadFamily, environment::WindowsDynamicEnvironment},
    a2b2_cases::JointCloseSelector,
};
use super::MEMBER_COUNT;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct EnvironmentKey {
    pub(super) git_sha: String,
    pub(super) target: &'static str,
    pub(super) windows_build: String,
    pub(super) architecture: &'static str,
    pub(super) volume_kind: &'static str,
    pub(super) filesystem: String,
    pub(super) bundled_sqlite: String,
}

#[derive(Clone)]
pub(super) struct MemberFact {
    pub(super) selector: JointCloseSelector,
    pub(super) canonical_name: &'static str,
    pub(super) environment: EnvironmentKey,
    pub(super) child: [u8; 32],
    pub(super) root: [u8; 32],
    pub(super) registration: [u8; 32],
    pub(super) payload: [u8; 32],
    pub(super) cohort: [u8; 32],
    pub(super) family: SanitizedPayloadFamily,
    pub(super) child_exit_code: i32,
}

pub(super) struct FamilyDigests {
    pub(super) cohort: [u8; 32],
    pub(super) seal: [u8; 32],
    pub(super) clean_commit_fingerprint: [u8; 32],
}

impl From<&WindowsDynamicEnvironment> for EnvironmentKey {
    fn from(environment: &WindowsDynamicEnvironment) -> Self {
        Self {
            git_sha: environment.git_sha.clone(),
            target: environment.target,
            windows_build: environment.windows_build.clone(),
            architecture: environment.architecture,
            volume_kind: environment.volume_kind,
            filesystem: environment.filesystem.clone(),
            bundled_sqlite: environment.bundled_sqlite.clone(),
        }
    }
}

pub(super) fn validate_family_facts(
    facts: &[MemberFact],
    cohort: [u8; 32],
    checkout_git_sha: &str,
) -> Result<FamilyDigests, &'static str> {
    if facts.len() < MEMBER_COUNT {
        return Err("A2_JOINT_CLOSE_FAMILY_SELECTOR_MISSING");
    }
    if facts.len() > MEMBER_COUNT {
        return Err("A2_JOINT_CLOSE_FAMILY_MEMBER_COUNT_INVALID");
    }
    let baseline = &facts[0].environment;
    if checkout_git_sha != baseline.git_sha {
        return Err("A2_JOINT_CLOSE_FAMILY_CHECKOUT_COMMIT_MISMATCH");
    }

    let mut selectors = BTreeSet::new();
    let mut children = BTreeSet::new();
    let mut roots = BTreeSet::new();
    let mut registrations = BTreeSet::new();
    let mut payloads = BTreeSet::new();
    for fact in facts {
        if fact.family != SanitizedPayloadFamily::JointClose {
            return Err("A2_JOINT_CLOSE_FAMILY_CROSS_FAMILY_MEMBER");
        }
        if fact.canonical_name != fact.selector.report_name() {
            return Err("A2_JOINT_CLOSE_FAMILY_SELECTOR_ALIAS_INVALID");
        }
        if !selectors.insert(fact.selector) {
            return Err("A2_JOINT_CLOSE_FAMILY_SELECTOR_DUPLICATE");
        }
        if fact.environment.git_sha != baseline.git_sha {
            return Err("A2_JOINT_CLOSE_FAMILY_COMMIT_MISMATCH");
        }
        if &fact.environment != baseline {
            return Err("A2_JOINT_CLOSE_FAMILY_ENVIRONMENT_MISMATCH");
        }
        if fact.cohort != cohort {
            return Err("A2_JOINT_CLOSE_FAMILY_COHORT_MISMATCH");
        }
        if fact.child_exit_code != 0 {
            return Err("A2_JOINT_CLOSE_FAMILY_CHILD_EXIT_INVALID");
        }
        if !children.insert(fact.child) {
            return Err("A2_JOINT_CLOSE_FAMILY_CHILD_IDENTITY_DUPLICATE");
        }
        if !roots.insert(fact.root) {
            return Err("A2_JOINT_CLOSE_FAMILY_ROOT_IDENTITY_DUPLICATE");
        }
        if !registrations.insert(fact.registration) {
            return Err("A2_JOINT_CLOSE_FAMILY_REGISTRATION_IDENTITY_DUPLICATE");
        }
        if !payloads.insert(fact.payload) {
            return Err("A2_JOINT_CLOSE_FAMILY_PAYLOAD_DUPLICATE");
        }
    }
    if selectors.len() != MEMBER_COUNT
        || JointCloseSelector::ALL
            .into_iter()
            .any(|selector| !selectors.contains(&selector))
    {
        return Err("A2_JOINT_CLOSE_FAMILY_SELECTOR_MISSING");
    }

    let mut ordered = facts.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|fact| selector_position(fact.selector));
    let cohort_digest = digest_cohort(cohort, &ordered);
    let clean_commit_fingerprint = digest_clean_commit(baseline, checkout_git_sha, &ordered);
    let family_seal = digest_family(cohort_digest, clean_commit_fingerprint, &ordered);
    Ok(FamilyDigests {
        cohort: cohort_digest,
        seal: family_seal,
        clean_commit_fingerprint,
    })
}

fn selector_position(selector: JointCloseSelector) -> usize {
    JointCloseSelector::ALL
        .iter()
        .position(|candidate| *candidate == selector)
        .unwrap_or(MEMBER_COUNT)
}

fn digest_cohort(cohort: [u8; 32], facts: &[&MemberFact]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-a2-joint-close-family-cohort-digest-v1\0");
    hasher.update(cohort);
    for fact in facts {
        hash_text(&mut hasher, fact.canonical_name);
        hasher.update(fact.child);
        hasher.update(fact.root);
        hasher.update(fact.registration);
    }
    hasher.finalize().into()
}

fn digest_clean_commit(
    environment: &EnvironmentKey,
    checkout_git_sha: &str,
    facts: &[&MemberFact],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-a2-joint-close-clean-commit-fingerprint-v1\0");
    hasher.update((MEMBER_COUNT as u64).to_le_bytes());
    for value in [
        environment.git_sha.as_str(),
        environment.target,
        environment.windows_build.as_str(),
        environment.architecture,
        environment.volume_kind,
        environment.filesystem.as_str(),
        environment.bundled_sqlite.as_str(),
        checkout_git_sha,
    ] {
        hash_text(&mut hasher, value);
    }
    for fact in facts {
        hash_text(&mut hasher, fact.canonical_name);
        hasher.update(fact.payload);
        hasher.update(fact.child_exit_code.to_le_bytes());
    }
    hasher.finalize().into()
}

fn digest_family(
    cohort_digest: [u8; 32],
    clean_commit_fingerprint: [u8; 32],
    facts: &[&MemberFact],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-a2-joint-close-formal-family-v1\0");
    hasher.update(cohort_digest);
    hasher.update(clean_commit_fingerprint);
    for fact in facts {
        hash_text(&mut hasher, fact.canonical_name);
        hasher.update(fact.payload);
    }
    hasher.finalize().into()
}

fn hash_text(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}
