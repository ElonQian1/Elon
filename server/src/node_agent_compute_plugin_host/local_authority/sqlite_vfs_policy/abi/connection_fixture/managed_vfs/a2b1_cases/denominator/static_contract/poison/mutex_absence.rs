//! Independent, source-bound proof that production cannot create either modeled Mutex poison.
//!
//! Rust still exposes `PoisonError` and the ABI catches unwinds, so the defensive runtime arms
//! remain correct.  They are not denominator terminals, however: every production holder and its
//! transitive operation surface is frozen below, and the only explicit owner panics are private
//! same-guard invariants. Deliberate poison producers live only behind `cfg(test)`.

mod candidates;
mod sources;

use super::super::model::ExclusionProof;

pub(crate) fn validate_mutex_poison_absence() {
    sources::validate().unwrap_or_else(|error| panic!("Mutex-poison authority drift: {error}"));
}

pub(crate) fn owner_mutex_poison_proof() -> ExclusionProof {
    ExclusionProof::ControlFlow(
        "the independent Mutex-poison source ledger freezes every production routes-Mutex holder and transitive operation; its only explicit panics are same-guard entry-removal/token invariants, while deliberate poisoning exists only under cfg(test)",
    )
}

pub(crate) fn coordinator_mutex_poison_proof() -> ExclusionProof {
    ExclusionProof::ControlFlow(
        "the independent Mutex-poison source ledger freezes every production SHM coordinator-state holder and transitive native operation; all input, arithmetic, custody and native failures return Result, while deliberate poisoning exists only under cfg(test)",
    )
}

#[cfg(test)]
mod tests {
    use super::sources;

    #[test]
    fn reviewed_mutex_poison_absence_ledger_matches_compiled_sources() {
        sources::validate().expect("reviewed Mutex-poison source ledger");
    }

    #[test]
    fn source_digest_drift_is_rejected() {
        let error =
            sources::validate_parts_for_test("synthetic.rs", "00", &[("guard", 1)], "guard")
                .expect_err("invalid digest must not pass");
        assert!(error.contains("SHA-256"));
    }

    #[test]
    fn source_sentinel_occurrence_drift_is_rejected() {
        let digest = sources::normalized_sha256_for_test("guard guard");
        let error = sources::validate_parts_for_test(
            "synthetic.rs",
            &digest,
            &[("guard", 1)],
            "guard guard",
        )
        .expect_err("ambiguous sentinel must not pass");
        assert!(error.contains("sentinel"));
    }
}
