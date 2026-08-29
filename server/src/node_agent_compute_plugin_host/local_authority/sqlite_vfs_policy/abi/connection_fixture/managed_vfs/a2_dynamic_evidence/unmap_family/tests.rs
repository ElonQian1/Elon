use super::*;

const GIT_SHA: &str = "0123456789abcdef0123456789abcdef01234567";
const COHORT: [u8; 32] = [0x45; 32];

#[test]
fn exact_facts_reduce_to_order_independent_unique_digests() {
    let facts = exact_facts();
    let expected = validate_family_facts(&facts, COHORT, GIT_SHA).expect("accept exact family");
    let mut reversed = facts;
    reversed.reverse();
    let reordered =
        validate_family_facts(&reversed, COHORT, GIT_SHA).expect("accept reordered family");
    assert_eq!(expected.cohort, reordered.cohort);
    assert_eq!(expected.seal, reordered.seal);
    assert_ne!(expected.cohort, expected.seal);
}

#[test]
fn reducer_rejects_missing_duplicate_and_aliased_selectors() {
    let mut missing = exact_facts();
    missing.pop();
    assert_eq!(
        reduce_error(&missing, COHORT, GIT_SHA),
        "A2_UNMAP_FAMILY_MEMBER_COUNT_INVALID"
    );

    let mut duplicate = exact_facts();
    duplicate[48].selector = duplicate[0].selector;
    duplicate[48].canonical_name = duplicate[0].canonical_name;
    assert_eq!(
        reduce_error(&duplicate, COHORT, GIT_SHA),
        "A2_UNMAP_FAMILY_SELECTOR_DUPLICATE"
    );

    let mut alias = exact_facts();
    alias[0].canonical_name = "shared_delete_request_validation";
    assert_eq!(
        reduce_error(&alias, COHORT, GIT_SHA),
        "A2_UNMAP_FAMILY_SELECTOR_ALIAS_INVALID"
    );
}

#[test]
fn reducer_rejects_mixed_commit_environment_run_and_checkout() {
    let mut mixed_commit = exact_facts();
    mixed_commit[20].environment.git_sha = "abcdef0123456789abcdef0123456789abcdef01".to_owned();
    assert_eq!(
        reduce_error(&mixed_commit, COHORT, GIT_SHA),
        "A2_UNMAP_FAMILY_COMMIT_MISMATCH"
    );

    let mut mixed_environment = exact_facts();
    mixed_environment[20].environment.windows_build = "10.0.99999".to_owned();
    assert_eq!(
        reduce_error(&mixed_environment, COHORT, GIT_SHA),
        "A2_UNMAP_FAMILY_ENVIRONMENT_MISMATCH"
    );

    let mut mixed_run = exact_facts();
    mixed_run[20].cohort = [0x99; 32];
    assert_eq!(
        reduce_error(&mixed_run, COHORT, GIT_SHA),
        "A2_UNMAP_FAMILY_COHORT_MISMATCH"
    );

    assert_eq!(
        reduce_error(
            &exact_facts(),
            COHORT,
            "abcdef0123456789abcdef0123456789abcdef01",
        ),
        "A2_UNMAP_FAMILY_CHECKOUT_COMMIT_MISMATCH"
    );
}

#[test]
fn reducer_rejects_duplicate_child_root_and_registration_bindings() {
    let mut duplicate_child = exact_facts();
    duplicate_child[20].child = duplicate_child[19].child;
    assert_eq!(
        reduce_error(&duplicate_child, COHORT, GIT_SHA),
        "A2_UNMAP_FAMILY_CHILD_IDENTITY_DUPLICATE"
    );

    let mut duplicate_root = exact_facts();
    duplicate_root[20].root = duplicate_root[19].root;
    assert_eq!(
        reduce_error(&duplicate_root, COHORT, GIT_SHA),
        "A2_UNMAP_FAMILY_ROOT_IDENTITY_DUPLICATE"
    );

    let mut duplicate_registration = exact_facts();
    duplicate_registration[20].registration = duplicate_registration[19].registration;
    assert_eq!(
        reduce_error(&duplicate_registration, COHORT, GIT_SHA),
        "A2_UNMAP_FAMILY_REGISTRATION_IDENTITY_DUPLICATE"
    );
}

#[test]
fn reducer_rejects_cross_family_or_nonzero_child_exit() {
    let mut cross_family = exact_facts();
    cross_family[20].family = SanitizedPayloadFamily::Barrier;
    assert_eq!(
        reduce_error(&cross_family, COHORT, GIT_SHA),
        "A2_UNMAP_FAMILY_CROSS_FAMILY_MEMBER"
    );

    let mut failed_child = exact_facts();
    failed_child[20].child_exit_code = 1;
    assert_eq!(
        reduce_error(&failed_child, COHORT, GIT_SHA),
        "A2_UNMAP_FAMILY_CHILD_EXIT_INVALID"
    );
}

fn exact_facts() -> Vec<MemberFact> {
    UnmapSelector::ALL
        .into_iter()
        .enumerate()
        .map(|(index, selector)| {
            let discriminator = u8::try_from(index + 1).expect("49 fits u8");
            MemberFact {
                selector,
                canonical_name: selector.report_name(),
                environment: environment(),
                child: [discriminator; 32],
                root: [discriminator.wrapping_add(50); 32],
                registration: [discriminator.wrapping_add(100); 32],
                payload: [discriminator.wrapping_add(150); 32],
                cohort: COHORT,
                family: SanitizedPayloadFamily::Unmap,
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
