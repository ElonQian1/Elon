//! Construction of the current authority context from reviewed, live literals only.

use super::{
    canonical,
    lock_ranges::{validate_lock_ranges, LOCK_RANGES},
    map_profiles::{validate_map_profiles, MAP_LOOP_PROFILES},
    source_scope::{source_scope_files, validate_source_scope, SOURCE_BASELINE_COMMIT_SHA1},
    Digest32, ManifestContextV1, RootOperationV1, AUTHORITY_SCHEMA_V1,
    PRODUCTION_WINDOWS_X64_SCOPE,
};

pub(crate) fn trusted_current_context(
    root: RootOperationV1,
    ledger_sha256: Digest32,
) -> Result<ManifestContextV1, String> {
    // Validate every independent input even when this root uses only one domain. A malformed
    // sibling ledger must not coexist with a context described as trusted-current.
    validate_source_scope()?;
    validate_map_profiles()?;
    validate_lock_ranges()?;

    let digests = TrustedLiteralDigests {
        source_scope: canonical::digest_source_scope(&source_scope_files())?,
        map_profiles: canonical::digest_map_profile_set(MAP_LOOP_PROFILES),
        map_ordinals: canonical::digest_map_ordinal_domains(MAP_LOOP_PROFILES),
        lock_ranges: canonical::digest_lock_range_set(LOCK_RANGES),
    };
    let context = assemble_context(root, ledger_sha256, digests);
    validate_context_binding(&context, root, ledger_sha256, digests)?;
    Ok(context)
}

#[derive(Clone, Copy)]
struct TrustedLiteralDigests {
    source_scope: Digest32,
    map_profiles: Digest32,
    map_ordinals: Digest32,
    lock_ranges: Digest32,
}

fn assemble_context(
    root: RootOperationV1,
    ledger_sha256: Digest32,
    digests: TrustedLiteralDigests,
) -> ManifestContextV1 {
    let (map_profiles, map_ordinals, lock_ranges, lock_range_count) = match root {
        RootOperationV1::Map => (
            Some(digests.map_profiles),
            Some(digests.map_ordinals),
            None,
            None,
        ),
        RootOperationV1::Lock => (
            None,
            None,
            Some(digests.lock_ranges),
            Some(LOCK_RANGES.len() as u64),
        ),
    };
    ManifestContextV1 {
        schema: AUTHORITY_SCHEMA_V1.to_owned(),
        root,
        target_scope: PRODUCTION_WINDOWS_X64_SCOPE.to_owned(),
        source_baseline_commit_sha1: SOURCE_BASELINE_COMMIT_SHA1.to_owned(),
        source_scope_sha256: digests.source_scope,
        ledger_sha256,
        map_profile_set_sha256: map_profiles,
        map_ordinal_domain_sha256: map_ordinals,
        lock_range_set_sha256: lock_ranges,
        lock_range_count,
    }
}

fn validate_context_binding(
    context: &ManifestContextV1,
    expected_root: RootOperationV1,
    expected_ledger: Digest32,
    digests: TrustedLiteralDigests,
) -> Result<(), String> {
    if context.schema != AUTHORITY_SCHEMA_V1
        || context.root != expected_root
        || context.target_scope != PRODUCTION_WINDOWS_X64_SCOPE
        || context.source_baseline_commit_sha1 != SOURCE_BASELINE_COMMIT_SHA1
        || context.source_scope_sha256 != digests.source_scope
        || context.ledger_sha256 == Digest32::ZERO
        || context.ledger_sha256 != expected_ledger
    {
        return Err("trusted current context identity drifted".to_owned());
    }
    let domain_matches = match expected_root {
        RootOperationV1::Map => {
            context.map_profile_set_sha256 == Some(digests.map_profiles)
                && context.map_ordinal_domain_sha256 == Some(digests.map_ordinals)
                && context.lock_range_set_sha256.is_none()
                && context.lock_range_count.is_none()
        }
        RootOperationV1::Lock => {
            context.map_profile_set_sha256.is_none()
                && context.map_ordinal_domain_sha256.is_none()
                && context.lock_range_set_sha256 == Some(digests.lock_ranges)
                && context.lock_range_count == Some(LOCK_RANGES.len() as u64)
        }
    };
    if !domain_matches {
        return Err("trusted current context domain drifted".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_digests() -> TrustedLiteralDigests {
        TrustedLiteralDigests {
            source_scope: Digest32([1; 32]),
            map_profiles: Digest32([2; 32]),
            map_ordinals: Digest32([3; 32]),
            lock_ranges: Digest32([4; 32]),
        }
    }

    #[test]
    fn pure_binding_rejects_root_and_domain_drift() {
        let ledger = Digest32([5; 32]);
        let digests = fake_digests();
        let mut context = assemble_context(RootOperationV1::Map, ledger, digests);
        assert_eq!(
            validate_context_binding(&context, RootOperationV1::Map, ledger, digests),
            Ok(())
        );

        context.root = RootOperationV1::Lock;
        assert!(validate_context_binding(&context, RootOperationV1::Map, ledger, digests).is_err());

        context.root = RootOperationV1::Map;
        context.map_ordinal_domain_sha256 = Some(Digest32([9; 32]));
        assert!(validate_context_binding(&context, RootOperationV1::Map, ledger, digests).is_err());
    }

    #[test]
    fn pure_binding_rejects_foreign_baseline_and_zero_ledger() {
        let digests = fake_digests();
        let ledger = Digest32([5; 32]);
        let mut context = assemble_context(RootOperationV1::Lock, ledger, digests);
        context.source_baseline_commit_sha1 = "a".repeat(40);
        assert!(
            validate_context_binding(&context, RootOperationV1::Lock, ledger, digests).is_err()
        );

        context.source_baseline_commit_sha1 = SOURCE_BASELINE_COMMIT_SHA1.to_owned();
        context.ledger_sha256 = Digest32::ZERO;
        assert!(
            validate_context_binding(&context, RootOperationV1::Lock, ledger, digests).is_err()
        );
    }
}
