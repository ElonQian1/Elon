//! Atomic 49-member evidence gate for the Windows Unmap family.
//!
//! A member remains non-renderable. Only a complete, same-run, same-environment family that also
//! consumes an exact-clean-checkout receipt can create the formal report text.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    fmt::Write as _,
    path::Path,
};

use sha2::{Digest, Sha256};

use super::super::a2b2_cases::{
    CaseKey, UnmapSelector, ValidatedUnmapObservation, ValidatedUnmapReportPayload,
};
use super::{
    child::{SanitizedPayloadFamily, ValidatedChildProcessReceipt},
    cleanup::ValidatedParentCleanupReceipt,
    environment::{validate_git_sha, WindowsDynamicEnvironment},
};

const MEMBER_COUNT: usize = 49;
const FORMAL_RECORD_MARKER: &str = "A2_WINDOWS_DYNAMIC_V2";
const FAMILY_MARKER: &str = "A2_UNMAP_WINDOWS_DYNAMIC_FAMILY_V1";

/// Parent-local identity shared by all child cases in one atomic family attempt.
///
/// The nonce is never rendered. Members retain only its domain-separated commitment.
pub(in super::super) struct UnmapFamilyCohort {
    nonce: String,
    commitment: [u8; 32],
}

/// Linear, non-renderable member receipt. It intentionally implements neither `Clone`, serde
/// traits, `Debug` nor `Display` and retains all validated observation and process witnesses.
#[must_use = "an Unmap family member must enter the atomic 49-member reducer"]
pub(in super::super) struct ValidatedUnmapFamilyMemberReceipt {
    selector: UnmapSelector,
    canonical_name: &'static str,
    registration_id: u64,
    _case_key: CaseKey,
    validated_payload: ValidatedUnmapReportPayload,
    environment: WindowsDynamicEnvironment,
    child: ValidatedChildProcessReceipt,
    cleanup: ValidatedParentCleanupReceipt,
    cohort_commitment: [u8; 32],
}

/// Linear proof that the compiled SHA matched HEAD and the exact checkout was clean.
#[must_use = "the clean-checkout receipt must be consumed by the Unmap family reducer"]
pub(in super::super) struct ValidatedUnmapCleanCheckoutReceipt {
    git_sha: String,
    cohort_commitment: [u8; 32],
}

/// Complete formal Unmap family. Construction is possible only through the atomic reducer.
#[must_use = "the complete Unmap family must be rendered atomically"]
pub(in super::super) struct ValidatedUnmapFamily {
    members: Box<[ValidatedUnmapFamilyMemberReceipt; MEMBER_COUNT]>,
    _cohort: UnmapFamilyCohort,
    _checkout: ValidatedUnmapCleanCheckoutReceipt,
    cohort_digest: [u8; 32],
    family_seal: [u8; 32],
}

/// Fully materialized output. Partial member sets have no API that can create this type.
pub(in super::super) struct RenderedUnmapFamilyReport {
    text: String,
}

#[derive(Clone, PartialEq, Eq)]
struct EnvironmentKey {
    git_sha: String,
    target: &'static str,
    windows_build: String,
    architecture: &'static str,
    volume_kind: &'static str,
    filesystem: String,
    bundled_sqlite: String,
}

#[derive(Clone)]
struct MemberFact {
    selector: UnmapSelector,
    canonical_name: &'static str,
    environment: EnvironmentKey,
    child: [u8; 32],
    root: [u8; 32],
    registration: [u8; 32],
    payload: [u8; 32],
    cohort: [u8; 32],
    family: SanitizedPayloadFamily,
    child_exit_code: i32,
}

struct FamilyDigests {
    cohort: [u8; 32],
    seal: [u8; 32],
}

impl UnmapFamilyCohort {
    pub(in super::super) fn new() -> Self {
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let commitment = domain_commitment(b"elon-a2-unmap-family-cohort-v1\0", nonce.as_bytes());
        Self { nonce, commitment }
    }

    fn is_valid(&self) -> bool {
        self.nonce.len() == 32
            && self
                .nonce
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            && self.commitment
                == domain_commitment(b"elon-a2-unmap-family-cohort-v1\0", self.nonce.as_bytes())
    }
}

impl ValidatedUnmapFamilyMemberReceipt {
    /// Consumes the exact validated observation, environment, child exit and cleanup witnesses.
    pub(in super::super) fn validate(
        observation: ValidatedUnmapObservation,
        environment: WindowsDynamicEnvironment,
        child: ValidatedChildProcessReceipt,
        cleanup: ValidatedParentCleanupReceipt,
        cohort: &UnmapFamilyCohort,
    ) -> Result<Self, &'static str> {
        if !cohort.is_valid() {
            return Err("A2_UNMAP_FAMILY_COHORT_INVALID");
        }
        let selector = observation.selector();
        let registration_id = observation.registration_id();
        let (case_key, validated_payload) = observation.into_evidence_parts();
        let receipt = Self {
            selector,
            canonical_name: selector.report_name(),
            registration_id,
            _case_key: case_key,
            validated_payload,
            environment,
            child,
            cleanup,
            cohort_commitment: cohort.commitment,
        };
        receipt.validate_integrity()?;
        Ok(receipt)
    }

    fn validate_integrity(&self) -> Result<(), &'static str> {
        let child_fingerprint = self.child.fingerprint();
        if self.environment.child_fingerprint != child_fingerprint
            || self.cleanup.child_fingerprint != child_fingerprint
        {
            return Err("A2_DYNAMIC_CHILD_RECEIPT_BINDING_MISMATCH");
        }
        if self.environment.root_commitment != self.child.root_commitment
            || self.cleanup.root_commitment != self.child.root_commitment
        {
            return Err("A2_DYNAMIC_ROOT_RECEIPT_BINDING_MISMATCH");
        }
        if self.environment.registration_commitment != self.child.registration_commitment
            || self.cleanup.registration_commitment != self.child.registration_commitment
        {
            return Err("A2_DYNAMIC_REGISTRATION_RECEIPT_BINDING_MISMATCH");
        }
        if !self.child.matches_registration_id(self.registration_id) {
            return Err("A2_DYNAMIC_REGISTRATION_ID_BINDING_MISMATCH");
        }
        if !self.child.matches_family(SanitizedPayloadFamily::Unmap) {
            return Err("A2_DYNAMIC_PAYLOAD_FAMILY_BINDING_MISMATCH");
        }
        if !self
            .validated_payload
            .matches_exact(&self.child.actual_payload)
            || !self
                .validated_payload
                .matches_commitment(&self.child.payload_commitment)
        {
            return Err("A2_DYNAMIC_ACTUAL_PAYLOAD_BINDING_MISMATCH");
        }
        let mut fields = self.child.actual_payload.split(',');
        if fields.next() != Some("a2b2un1")
            || fields.next() != Some(self.selector.report_name())
            || self.canonical_name != self.selector.report_name()
        {
            return Err("A2_UNMAP_FAMILY_SELECTOR_BINDING_MISMATCH");
        }
        if self.child.exit_code != 0 {
            return Err("A2_UNMAP_FAMILY_CHILD_EXIT_INVALID");
        }
        Ok(())
    }

    fn fact(&self) -> MemberFact {
        MemberFact {
            selector: self.selector,
            canonical_name: self.canonical_name,
            environment: EnvironmentKey::from(&self.environment),
            child: self.child.fingerprint().0,
            root: self.child.root_commitment.0,
            registration: self.child.registration_commitment.0,
            payload: self.child.payload_commitment.0,
            cohort: self.cohort_commitment,
            family: SanitizedPayloadFamily::Unmap,
            child_exit_code: self.child.exit_code,
        }
    }
}

impl ValidatedUnmapCleanCheckoutReceipt {
    /// Observes HEAD and complete porcelain status itself; callers cannot declare cleanliness.
    pub(in super::super) fn capture(cohort: &UnmapFamilyCohort) -> Result<Self, &'static str> {
        if !cohort.is_valid() {
            return Err("A2_UNMAP_FAMILY_COHORT_INVALID");
        }
        let expected = validate_git_sha(
            option_env!("ELON_NODE_AGENT_GIT_SHA")
                .ok_or("A2_UNMAP_FAMILY_COMPILED_GIT_SHA_MISSING")?,
        )?;
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = manifest
            .parent()
            .ok_or("A2_UNMAP_FAMILY_CHECKOUT_ROOT_INVALID")?;
        let head = git_output(root, &["rev-parse", "HEAD"])?;
        if head.trim() != expected {
            return Err("A2_UNMAP_FAMILY_CHECKOUT_HEAD_MISMATCH");
        }
        let status = git_output(root, &["status", "--porcelain=v1", "--untracked-files=all"])?;
        if !status.trim().is_empty() {
            return Err("A2_UNMAP_FAMILY_CHECKOUT_NOT_CLEAN");
        }
        Ok(Self {
            git_sha: expected.to_owned(),
            cohort_commitment: cohort.commitment,
        })
    }
}

impl ValidatedUnmapFamily {
    /// Atomically consumes exactly 49 sealed members, their shared cohort and clean checkout.
    pub(in super::super) fn reduce(
        cohort: UnmapFamilyCohort,
        members: Vec<ValidatedUnmapFamilyMemberReceipt>,
        checkout: ValidatedUnmapCleanCheckoutReceipt,
    ) -> Result<Self, &'static str> {
        if !cohort.is_valid() {
            return Err("A2_UNMAP_FAMILY_COHORT_INVALID");
        }
        if checkout.cohort_commitment != cohort.commitment {
            return Err("A2_UNMAP_FAMILY_CHECKOUT_COHORT_MISMATCH");
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
                return Err("A2_UNMAP_FAMILY_SELECTOR_DUPLICATE");
            }
        }
        let mut ordered = Vec::with_capacity(MEMBER_COUNT);
        for selector in UnmapSelector::ALL {
            ordered.push(
                by_selector
                    .remove(&selector)
                    .ok_or("A2_UNMAP_FAMILY_SELECTOR_MISSING")?,
            );
        }
        let members: Box<[ValidatedUnmapFamilyMemberReceipt; MEMBER_COUNT]> = Box::new(
            ordered
                .try_into()
                .map_err(|_| "A2_UNMAP_FAMILY_MEMBER_COUNT_INVALID")?,
        );
        Ok(Self {
            members,
            _cohort: cohort,
            _checkout: checkout,
            cohort_digest: digests.cohort,
            family_seal: digests.seal,
        })
    }

    /// Materializes all 49 formal records and the one family seal into one printable value.
    pub(in super::super) fn render_atomic(self) -> RenderedUnmapFamilyReport {
        let mut text = String::new();
        for member in self.members.iter() {
            let environment = &member.environment;
            let _ = writeln!(
                text,
                "{FORMAL_RECORD_MARKER} case={} commit={} target={} windows_build={} arch={} volume={} filesystem={} bundled_sqlite={} child={} root={} registration={} child_exit={} parent_cleanup=deleted actual={} actual_commitment={}",
                member.canonical_name,
                environment.git_sha,
                environment.target,
                environment.windows_build,
                environment.architecture,
                environment.volume_kind,
                environment.filesystem,
                environment.bundled_sqlite,
                opaque_commitment(&member.child.fingerprint().0),
                opaque_commitment(&member.child.root_commitment.0),
                opaque_commitment(&member.child.registration_commitment.0),
                member.child.exit_code,
                member.validated_payload.exact_payload(),
                opaque_commitment(&member.child.payload_commitment.0),
            );
        }
        let _ = write!(
            text,
            "{FAMILY_MARKER} cases={MEMBER_COUNT} commit={} cohort={} seal={} checkout=clean",
            self.members[0].environment.git_sha,
            opaque_commitment(&self.cohort_digest),
            opaque_commitment(&self.family_seal),
        );
        RenderedUnmapFamilyReport { text }
    }
}

impl RenderedUnmapFamilyReport {
    pub(in super::super) fn as_str(&self) -> &str {
        &self.text
    }
}

impl fmt::Display for RenderedUnmapFamilyReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
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

fn validate_family_facts(
    facts: &[MemberFact],
    cohort: [u8; 32],
    checkout_git_sha: &str,
) -> Result<FamilyDigests, &'static str> {
    if facts.len() != MEMBER_COUNT {
        return Err("A2_UNMAP_FAMILY_MEMBER_COUNT_INVALID");
    }
    let baseline = &facts[0].environment;
    if checkout_git_sha != baseline.git_sha {
        return Err("A2_UNMAP_FAMILY_CHECKOUT_COMMIT_MISMATCH");
    }
    let mut selectors = BTreeSet::new();
    let mut children = BTreeSet::new();
    let mut roots = BTreeSet::new();
    let mut registrations = BTreeSet::new();
    for fact in facts {
        if fact.family != SanitizedPayloadFamily::Unmap {
            return Err("A2_UNMAP_FAMILY_CROSS_FAMILY_MEMBER");
        }
        if fact.canonical_name != fact.selector.report_name() {
            return Err("A2_UNMAP_FAMILY_SELECTOR_ALIAS_INVALID");
        }
        if !selectors.insert(fact.selector) {
            return Err("A2_UNMAP_FAMILY_SELECTOR_DUPLICATE");
        }
        if fact.environment.git_sha != baseline.git_sha {
            return Err("A2_UNMAP_FAMILY_COMMIT_MISMATCH");
        }
        if &fact.environment != baseline {
            return Err("A2_UNMAP_FAMILY_ENVIRONMENT_MISMATCH");
        }
        if fact.cohort != cohort {
            return Err("A2_UNMAP_FAMILY_COHORT_MISMATCH");
        }
        if fact.child_exit_code != 0 {
            return Err("A2_UNMAP_FAMILY_CHILD_EXIT_INVALID");
        }
        if !children.insert(fact.child) {
            return Err("A2_UNMAP_FAMILY_CHILD_IDENTITY_DUPLICATE");
        }
        if !roots.insert(fact.root) {
            return Err("A2_UNMAP_FAMILY_ROOT_IDENTITY_DUPLICATE");
        }
        if !registrations.insert(fact.registration) {
            return Err("A2_UNMAP_FAMILY_REGISTRATION_IDENTITY_DUPLICATE");
        }
    }
    if selectors.len() != MEMBER_COUNT
        || UnmapSelector::ALL
            .into_iter()
            .any(|selector| !selectors.contains(&selector))
    {
        return Err("A2_UNMAP_FAMILY_SELECTOR_MISSING");
    }

    let mut ordered = facts.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|fact| {
        UnmapSelector::ALL
            .iter()
            .position(|selector| *selector == fact.selector)
            .unwrap_or(MEMBER_COUNT)
    });
    let cohort_digest = digest_cohort(cohort, &ordered);
    let family_seal = digest_family(baseline, checkout_git_sha, cohort_digest, &ordered);
    Ok(FamilyDigests {
        cohort: cohort_digest,
        seal: family_seal,
    })
}

fn digest_cohort(cohort: [u8; 32], facts: &[&MemberFact]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-a2-unmap-family-cohort-digest-v1\0");
    hasher.update(cohort);
    for fact in facts {
        hash_text(&mut hasher, fact.canonical_name);
        hasher.update(fact.child);
        hasher.update(fact.root);
        hasher.update(fact.registration);
    }
    hasher.finalize().into()
}

fn digest_family(
    environment: &EnvironmentKey,
    checkout_git_sha: &str,
    cohort_digest: [u8; 32],
    facts: &[&MemberFact],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-a2-unmap-formal-family-v1\0");
    hasher.update(cohort_digest);
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

fn hash_text(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn domain_commitment(domain: &[u8], value: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value);
    hasher.finalize().into()
}

fn opaque_commitment(value: &[u8; 32]) -> String {
    format!("sha256:{}", hex::encode(value))
}

fn git_output(root: &Path, arguments: &[&str]) -> Result<String, &'static str> {
    let output = crate::git_command_error::git_command()
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .map_err(|_| "A2_UNMAP_FAMILY_CHECKOUT_GIT_FAILED")?;
    if !output.status.success() {
        return Err("A2_UNMAP_FAMILY_CHECKOUT_GIT_FAILED");
    }
    String::from_utf8(output.stdout).map_err(|_| "A2_UNMAP_FAMILY_CHECKOUT_GIT_NON_UTF8")
}

#[cfg(test)]
mod tests;
