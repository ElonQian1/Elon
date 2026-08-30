//! Root manifest construction and frozen-baseline validation.

use super::{
    accumulator::ManifestAccumulatorV1,
    canonical,
    comparison::{compare_exact_records, RecordDiff},
    model::{Digest32, LeafRecordV1, ManifestContextV1, RootManifestV1},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AuthorityValidationError {
    InvalidAuthority(String),
    FrozenManifestSelfDigest {
        declared: Digest32,
        recomputed: Digest32,
    },
    AuthorityManifestDrift {
        frozen: Digest32,
        derived: Digest32,
    },
    ActualRecordMismatch(RecordDiff),
}

/// Builds a manifest from independently expanded authority records.
///
/// This function is suitable for an offline source-ledger tool and for validation. It must never
/// be called with graph DFS output to create or bless a frozen baseline without source review.
pub(crate) fn build_manifest(
    context: ManifestContextV1,
    authority_records: &[LeafRecordV1],
) -> Result<RootManifestV1, String> {
    let mut accumulator = ManifestAccumulatorV1::new(context)?;
    for record in authority_records {
        accumulator.push(record)?;
    }
    accumulator.finish()
}

pub(crate) fn validate_derived_manifest_against_frozen(
    derived: &RootManifestV1,
    frozen: &RootManifestV1,
) -> Result<u64, AuthorityValidationError> {
    let recomputed = canonical::digest_manifest_body(frozen);
    if recomputed != frozen.manifest_sha256 {
        return Err(AuthorityValidationError::FrozenManifestSelfDigest {
            declared: frozen.manifest_sha256,
            recomputed,
        });
    }
    if derived != frozen {
        return Err(AuthorityValidationError::AuthorityManifestDrift {
            frozen: frozen.manifest_sha256,
            derived: derived.manifest_sha256,
        });
    }
    Ok(frozen.included_count)
}

/// Validates all three independent bindings:
///
/// 1. the checked-in manifest is internally sealed;
/// 2. source-first ledger expansion still equals that frozen manifest; and
/// 3. graph DFS projection exactly equals the independently expanded records.
pub(crate) fn validate_actual_against_frozen(
    context: ManifestContextV1,
    authority_records: &[LeafRecordV1],
    actual_records: &[LeafRecordV1],
    frozen_manifest: &RootManifestV1,
) -> Result<u64, AuthorityValidationError> {
    let derived = build_manifest(context, authority_records)
        .map_err(AuthorityValidationError::InvalidAuthority)?;
    validate_derived_manifest_against_frozen(&derived, frozen_manifest)?;
    compare_exact_records(authority_records, actual_records)
        .map_err(AuthorityValidationError::ActualRecordMismatch)?;
    Ok(frozen_manifest.included_count)
}
