//! Canonical text form for the root and all 256 shard manifests.

use super::{
    accumulator::validate_context,
    canonical,
    leaf_seal::{optional_digest, parse_digest, parse_optional_digest},
    model::{ManifestContextV1, RootManifestV1, RootOperationV1, ShardManifestV1},
    MANIFEST_SHARDS,
};

pub(crate) const MANIFEST_TSV_HEADER_V1: &str = "ELON-A2B1-VFS-ROOT-MANIFEST-TSV-V1";

pub(crate) fn encode_manifest_tsv(manifest: &RootManifestV1) -> Result<String, String> {
    validate_context(&manifest.context)?;
    validate_shards(&manifest.shards)?;
    if canonical::digest_manifest_body(manifest) != manifest.manifest_sha256 {
        return Err("cannot encode a root manifest with an invalid self-digest".to_owned());
    }
    let context = &manifest.context;
    let mut result = String::new();
    result.push_str(MANIFEST_TSV_HEADER_V1);
    result.push('\n');
    result.push_str(&format!(
        "context\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
        context.schema,
        context.root.canonical_name(),
        context.target_scope,
        context.source_baseline_commit_sha1,
        context.source_scope_sha256.to_lower_hex(),
        context.ledger_sha256.to_lower_hex(),
        optional_digest(context.map_profile_set_sha256),
        optional_digest(context.map_ordinal_domain_sha256),
        optional_digest(context.lock_range_set_sha256),
        context
            .lock_range_count
            .map_or_else(|| "-".to_owned(), |value| value.to_string()),
    ));
    result.push_str(&format!(
        "global\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
        manifest.included_count,
        manifest.excluded_count,
        manifest.source_leaf_identity_set_sha256.to_lower_hex(),
        manifest.case_key_set_sha256.to_lower_hex(),
        manifest.source_branch_map_sha256.to_lower_hex(),
        manifest.expected_map_sha256.to_lower_hex(),
        manifest.exclusion_map_sha256.to_lower_hex(),
        manifest.full_record_set_sha256.to_lower_hex(),
        manifest.manifest_sha256.to_lower_hex(),
    ));
    for shard in &manifest.shards {
        result.push_str(&format!(
            "shard\t{:03}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            shard.index,
            shard.included_count,
            shard.excluded_count,
            shard.source_leaf_identity_set_sha256.to_lower_hex(),
            shard.case_key_set_sha256.to_lower_hex(),
            shard.source_branch_map_sha256.to_lower_hex(),
            shard.expected_map_sha256.to_lower_hex(),
            shard.exclusion_map_sha256.to_lower_hex(),
            shard.full_record_set_sha256.to_lower_hex(),
        ));
    }
    Ok(result)
}

pub(crate) fn parse_manifest_tsv(input: &str) -> Result<RootManifestV1, String> {
    if input.starts_with('\u{feff}') {
        return Err("manifest TSV must be UTF-8 without BOM".to_owned());
    }
    let mut lines = input.lines();
    if lines.next() != Some(MANIFEST_TSV_HEADER_V1) {
        return Err("root manifest TSV header/schema mismatch".to_owned());
    }
    let context = parse_context(lines.next().ok_or("manifest TSV lacks context row")?)?;
    let (
        included_count,
        excluded_count,
        identity,
        case_key,
        source,
        expected,
        exclusion,
        full,
        seal,
    ) = parse_global(lines.next().ok_or("manifest TSV lacks global row")?)?;
    let mut shards = Vec::with_capacity(MANIFEST_SHARDS);
    for expected_index in 0..MANIFEST_SHARDS {
        let line = lines
            .next()
            .ok_or_else(|| format!("manifest TSV lacks shard {expected_index:03}"))?;
        let shard = parse_shard(line)?;
        if usize::from(shard.index) != expected_index {
            return Err(format!(
                "manifest TSV shard order mismatch: expected {expected_index:03}, found {:03}",
                shard.index
            ));
        }
        shards.push(shard);
    }
    if lines.next().is_some() {
        return Err("manifest TSV has trailing rows".to_owned());
    }
    let manifest = RootManifestV1 {
        context,
        included_count,
        excluded_count,
        source_leaf_identity_set_sha256: identity,
        case_key_set_sha256: case_key,
        source_branch_map_sha256: source,
        expected_map_sha256: expected,
        exclusion_map_sha256: exclusion,
        full_record_set_sha256: full,
        shards,
        manifest_sha256: seal,
    };
    validate_context(&manifest.context)?;
    if canonical::digest_manifest_body(&manifest) != manifest.manifest_sha256 {
        return Err("root manifest TSV self-digest mismatch".to_owned());
    }
    if encode_manifest_tsv(&manifest)? != input {
        return Err(
            "root manifest TSV bytes are not the canonical LF/trailing-newline form".to_owned(),
        );
    }
    Ok(manifest)
}

fn parse_context(line: &str) -> Result<ManifestContextV1, String> {
    let fields = line.split('\t').collect::<Vec<_>>();
    let ["context", schema, root, target, baseline, source, ledger, map_profiles, map_ordinals, lock_ranges, lock_count] =
        fields.as_slice()
    else {
        return Err("manifest context row must contain exactly 11 columns".to_owned());
    };
    Ok(ManifestContextV1 {
        schema: (*schema).to_owned(),
        root: parse_root(root)?,
        target_scope: (*target).to_owned(),
        source_baseline_commit_sha1: (*baseline).to_owned(),
        source_scope_sha256: parse_digest(source)?,
        ledger_sha256: parse_digest(ledger)?,
        map_profile_set_sha256: parse_optional_digest(map_profiles)?,
        map_ordinal_domain_sha256: parse_optional_digest(map_ordinals)?,
        lock_range_set_sha256: parse_optional_digest(lock_ranges)?,
        lock_range_count: parse_optional_u64(lock_count)?,
    })
}

#[allow(clippy::type_complexity)]
fn parse_global(
    line: &str,
) -> Result<
    (
        u64,
        u64,
        super::Digest32,
        super::Digest32,
        super::Digest32,
        super::Digest32,
        super::Digest32,
        super::Digest32,
        super::Digest32,
    ),
    String,
> {
    let fields = line.split('\t').collect::<Vec<_>>();
    let ["global", included, excluded, identity, case_key, source, expected, exclusion, full, seal] =
        fields.as_slice()
    else {
        return Err("manifest global row must contain exactly 10 columns".to_owned());
    };
    Ok((
        parse_u64(included)?,
        parse_u64(excluded)?,
        parse_digest(identity)?,
        parse_digest(case_key)?,
        parse_digest(source)?,
        parse_digest(expected)?,
        parse_digest(exclusion)?,
        parse_digest(full)?,
        parse_digest(seal)?,
    ))
}

fn parse_shard(line: &str) -> Result<ShardManifestV1, String> {
    let fields = line.split('\t').collect::<Vec<_>>();
    let ["shard", index, included, excluded, identity, case_key, source, expected, exclusion, full] =
        fields.as_slice()
    else {
        return Err("manifest shard row must contain exactly 10 columns".to_owned());
    };
    if index.len() != 3 {
        return Err("manifest shard index must use three decimal digits".to_owned());
    }
    Ok(ShardManifestV1 {
        index: index
            .parse::<u8>()
            .map_err(|_| "manifest shard index is outside 000..255".to_owned())?,
        included_count: parse_u64(included)?,
        excluded_count: parse_u64(excluded)?,
        source_leaf_identity_set_sha256: parse_digest(identity)?,
        case_key_set_sha256: parse_digest(case_key)?,
        source_branch_map_sha256: parse_digest(source)?,
        expected_map_sha256: parse_digest(expected)?,
        exclusion_map_sha256: parse_digest(exclusion)?,
        full_record_set_sha256: parse_digest(full)?,
    })
}

fn validate_shards(shards: &[ShardManifestV1]) -> Result<(), String> {
    if shards.len() != MANIFEST_SHARDS
        || shards
            .iter()
            .enumerate()
            .any(|(index, shard)| usize::from(shard.index) != index)
    {
        return Err("root manifest must contain shards 000..255 in order".to_owned());
    }
    Ok(())
}

fn parse_root(value: &str) -> Result<RootOperationV1, String> {
    match value {
        "map" => Ok(RootOperationV1::Map),
        "lock" => Ok(RootOperationV1::Lock),
        _ => Err("manifest root is not map or lock".to_owned()),
    }
}

fn parse_optional_u64(value: &str) -> Result<Option<u64>, String> {
    if value == "-" {
        Ok(None)
    } else {
        parse_u64(value).map(Some)
    }
}

fn parse_u64(value: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| "manifest count is not an unsigned decimal".to_owned())?;
    if parsed.to_string() != value {
        return Err("manifest count is not canonically encoded".to_owned());
    }
    Ok(parsed)
}
