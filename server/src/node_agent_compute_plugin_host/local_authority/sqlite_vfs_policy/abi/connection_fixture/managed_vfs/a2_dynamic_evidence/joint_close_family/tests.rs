use super::super::super::a2b2_cases::JointCloseSelector;
use super::{
    super::child::SanitizedPayloadFamily,
    digest::{validate_family_facts, EnvironmentKey, MemberFact},
    MEMBER_COUNT,
};

const GIT_SHA: &str = "0123456789abcdef0123456789abcdef01234567";
const COHORT: [u8; 32] = [0x45; 32];

#[test]
fn exact_36_facts_reduce_to_order_independent_digests() {
    assert_eq!(JointCloseSelector::ALL.len(), MEMBER_COUNT);
    let facts = exact_facts(COHORT, 0);
    let expected = validate_family_facts(&facts, COHORT, GIT_SHA).expect("accept exact family");
    let mut reversed = facts;
    reversed.reverse();
    let reordered =
        validate_family_facts(&reversed, COHORT, GIT_SHA).expect("accept reordered family");
    assert_eq!(expected.cohort, reordered.cohort);
    assert_eq!(expected.seal, reordered.seal);
    assert_eq!(
        expected.clean_commit_fingerprint,
        reordered.clean_commit_fingerprint
    );
    assert_ne!(expected.cohort, expected.seal);
}

#[test]
fn clean_commit_fingerprint_excludes_random_run_bindings() {
    let first =
        validate_family_facts(&exact_facts(COHORT, 0), COHORT, GIT_SHA).expect("accept first run");
    let other_cohort = [0x91; 32];
    let second = validate_family_facts(&exact_facts(other_cohort, 37), other_cohort, GIT_SHA)
        .expect("accept second run");
    assert_eq!(
        first.clean_commit_fingerprint,
        second.clean_commit_fingerprint
    );
    assert_ne!(first.cohort, second.cohort);
    assert_ne!(first.seal, second.seal);
}

#[test]
fn reducer_rejects_missing_duplicate_and_aliased_selectors() {
    let mut missing = exact_facts(COHORT, 0);
    missing.pop();
    assert_eq!(
        reduce_error(&missing, COHORT, GIT_SHA),
        "A2_JOINT_CLOSE_FAMILY_SELECTOR_MISSING"
    );

    let mut duplicate = exact_facts(COHORT, 0);
    duplicate[MEMBER_COUNT - 1].selector = duplicate[0].selector;
    duplicate[MEMBER_COUNT - 1].canonical_name = duplicate[0].canonical_name;
    assert_eq!(
        reduce_error(&duplicate, COHORT, GIT_SHA),
        "A2_JOINT_CLOSE_FAMILY_SELECTOR_DUPLICATE"
    );

    let mut alias = exact_facts(COHORT, 0);
    alias[0].canonical_name = "raw_state_take_rejected";
    assert_eq!(
        reduce_error(&alias, COHORT, GIT_SHA),
        "A2_JOINT_CLOSE_FAMILY_SELECTOR_ALIAS_INVALID"
    );
}

#[test]
fn reducer_rejects_mixed_commit_environment_cohort_and_checkout() {
    let mut mixed_commit = exact_facts(COHORT, 0);
    mixed_commit[20].environment.git_sha = "abcdef0123456789abcdef0123456789abcdef01".to_owned();
    assert_eq!(
        reduce_error(&mixed_commit, COHORT, GIT_SHA),
        "A2_JOINT_CLOSE_FAMILY_COMMIT_MISMATCH"
    );

    let mut mixed_environment = exact_facts(COHORT, 0);
    mixed_environment[20].environment.windows_build = "10.0.99999".to_owned();
    assert_eq!(
        reduce_error(&mixed_environment, COHORT, GIT_SHA),
        "A2_JOINT_CLOSE_FAMILY_ENVIRONMENT_MISMATCH"
    );

    let mut mixed_cohort = exact_facts(COHORT, 0);
    mixed_cohort[20].cohort = [0x99; 32];
    assert_eq!(
        reduce_error(&mixed_cohort, COHORT, GIT_SHA),
        "A2_JOINT_CLOSE_FAMILY_COHORT_MISMATCH"
    );

    assert_eq!(
        reduce_error(
            &exact_facts(COHORT, 0),
            COHORT,
            "abcdef0123456789abcdef0123456789abcdef01",
        ),
        "A2_JOINT_CLOSE_FAMILY_CHECKOUT_COMMIT_MISMATCH"
    );
}

#[test]
fn reducer_rejects_duplicate_process_and_payload_bindings() {
    let mut duplicate_child = exact_facts(COHORT, 0);
    duplicate_child[20].child = duplicate_child[19].child;
    assert_eq!(
        reduce_error(&duplicate_child, COHORT, GIT_SHA),
        "A2_JOINT_CLOSE_FAMILY_CHILD_IDENTITY_DUPLICATE"
    );

    let mut duplicate_root = exact_facts(COHORT, 0);
    duplicate_root[20].root = duplicate_root[19].root;
    assert_eq!(
        reduce_error(&duplicate_root, COHORT, GIT_SHA),
        "A2_JOINT_CLOSE_FAMILY_ROOT_IDENTITY_DUPLICATE"
    );

    let mut duplicate_registration = exact_facts(COHORT, 0);
    duplicate_registration[20].registration = duplicate_registration[19].registration;
    assert_eq!(
        reduce_error(&duplicate_registration, COHORT, GIT_SHA),
        "A2_JOINT_CLOSE_FAMILY_REGISTRATION_IDENTITY_DUPLICATE"
    );

    let mut duplicate_payload = exact_facts(COHORT, 0);
    duplicate_payload[20].payload = duplicate_payload[19].payload;
    assert_eq!(
        reduce_error(&duplicate_payload, COHORT, GIT_SHA),
        "A2_JOINT_CLOSE_FAMILY_PAYLOAD_DUPLICATE"
    );
}

#[test]
fn reducer_rejects_cross_family_or_failed_child() {
    let mut cross_family = exact_facts(COHORT, 0);
    cross_family[20].family = SanitizedPayloadFamily::Unmap;
    assert_eq!(
        reduce_error(&cross_family, COHORT, GIT_SHA),
        "A2_JOINT_CLOSE_FAMILY_CROSS_FAMILY_MEMBER"
    );

    let mut failed_child = exact_facts(COHORT, 0);
    failed_child[20].child_exit_code = 1;
    assert_eq!(
        reduce_error(&failed_child, COHORT, GIT_SHA),
        "A2_JOINT_CLOSE_FAMILY_CHILD_EXIT_INVALID"
    );
}

fn exact_facts(cohort: [u8; 32], run_offset: u8) -> Vec<MemberFact> {
    JointCloseSelector::ALL
        .into_iter()
        .enumerate()
        .map(|(index, selector)| {
            let discriminator = u8::try_from(index + 1).expect("36 fits u8");
            MemberFact {
                selector,
                canonical_name: selector.report_name(),
                environment: environment(),
                child: [discriminator.wrapping_add(run_offset); 32],
                root: [discriminator.wrapping_add(50).wrapping_add(run_offset); 32],
                registration: [discriminator.wrapping_add(100).wrapping_add(run_offset); 32],
                payload: [discriminator.wrapping_add(150); 32],
                cohort,
                family: SanitizedPayloadFamily::JointClose,
                child_exit_code: 0,
            }
        })
        .collect()
}

fn environment() -> EnvironmentKey {
    EnvironmentKey {
        git_sha: GIT_SHA.to_owned(),
        target: "elon-pc-node",
        windows_build: "10.0.26100".to_owned(),
        architecture: "x86_64",
        volume_kind: "fixed",
        filesystem: "ntfs".to_owned(),
        bundled_sqlite: "3.45.0".to_owned(),
    }
}

fn reduce_error(facts: &[MemberFact], cohort: [u8; 32], checkout: &str) -> &'static str {
    validate_family_facts(facts, cohort, checkout)
        .err()
        .expect("reject family")
}
