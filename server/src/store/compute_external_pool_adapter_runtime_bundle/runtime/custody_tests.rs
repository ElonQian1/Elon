use chrono::{Duration as ChronoDuration, SecondsFormat, Utc};

use super::ExternalPoolAdapterProviderRuntimeReadinessProcessCustody;

const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

#[test]
fn provider_readiness_seal_requires_post_commit_promotion_and_exact_material() {
    let custody = ExternalPoolAdapterProviderRuntimeReadinessProcessCustody::generate()
        .expect("process custody should generate");
    let expires_at = live_expiry(5_000);

    custody
        .remember_readiness_seal("receipt-1", DIGEST_A, DIGEST_B, DIGEST_C, &expires_at)
        .expect("pending readiness seal should be remembered");
    assert!(!custody
        .attests_readiness_seal("receipt-1", DIGEST_A, DIGEST_B, DIGEST_C, &expires_at)
        .expect("pending attestation should evaluate"));
    assert!(custody
        .commit_readiness_seal("receipt-1", DIGEST_A)
        .expect("exact seal should commit"));
    assert!(custody
        .attests_readiness_seal("receipt-1", DIGEST_A, DIGEST_B, DIGEST_C, &expires_at)
        .expect("committed attestation should evaluate"));
    assert!(!custody
        .attests_readiness_seal("receipt-1", DIGEST_A, DIGEST_C, DIGEST_B, &expires_at)
        .expect("drifted attestation should evaluate"));
}

#[test]
fn provider_readiness_seal_is_idempotent_but_rejects_identity_drift() {
    let custody = ExternalPoolAdapterProviderRuntimeReadinessProcessCustody::generate()
        .expect("process custody should generate");
    let expires_at = live_expiry(5_000);

    custody
        .remember_readiness_seal("receipt-2", DIGEST_A, DIGEST_B, DIGEST_C, &expires_at)
        .expect("first pending seal should succeed");
    custody
        .remember_readiness_seal("receipt-2", DIGEST_A, DIGEST_B, DIGEST_C, &expires_at)
        .expect("exact replay should be idempotent");
    let error = custody
        .remember_readiness_seal("receipt-2", DIGEST_B, DIGEST_B, DIGEST_C, &expires_at)
        .expect_err("same identity with drifted material must fail");
    assert!(format!("{error:#}").contains("different process seal"));
}

#[test]
fn provider_readiness_seal_cannot_cross_process_custody_epochs_or_extend_ttl() {
    let original = ExternalPoolAdapterProviderRuntimeReadinessProcessCustody::generate()
        .expect("original process custody should generate");
    let restarted = ExternalPoolAdapterProviderRuntimeReadinessProcessCustody::generate()
        .expect("replacement process custody should generate");
    assert_ne!(
        original.custody_epoch_digest(),
        restarted.custody_epoch_digest()
    );
    assert!(original.attests_custody_epoch_digest(original.custody_epoch_digest()));
    assert!(!restarted.attests_custody_epoch_digest(original.custody_epoch_digest()));

    let expires_at = live_expiry(5_000);
    original
        .remember_readiness_seal("receipt-3", DIGEST_A, DIGEST_B, DIGEST_C, &expires_at)
        .expect("original process should remember its seal");
    assert!(original
        .commit_readiness_seal("receipt-3", DIGEST_A)
        .expect("original process should commit its seal"));
    assert!(!restarted
        .attests_readiness_seal("receipt-3", DIGEST_A, DIGEST_B, DIGEST_C, &expires_at)
        .expect("replacement process attestation should evaluate"));

    let too_long = live_expiry(15_500);
    let error = original
        .remember_readiness_seal("receipt-4", DIGEST_A, DIGEST_B, DIGEST_C, &too_long)
        .expect_err("seal must not extend beyond the fixed 15 second window");
    assert!(format!("{error:#}").contains("outside its fixed live window"));
}

fn live_expiry(milliseconds: i64) -> String {
    (Utc::now() + ChronoDuration::milliseconds(milliseconds))
        .to_rfc3339_opts(SecondsFormat::Nanos, true)
}
