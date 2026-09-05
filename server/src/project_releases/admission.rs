use std::io::{Cursor, Read};

use sha2::{Digest, Sha256};
use thiserror::Error;
use zip::ZipArchive;

pub(crate) const OFFICIAL_QUANT_PROJECT_ID: &str = "yilong-quant";
pub(crate) const OFFICIAL_QUANT_PACKAGE_NAME: &str = "com.elon.quant";
pub(crate) const OFFICIAL_QUANT_MIN_VERSION_CODE: i64 = 5;
pub(crate) const OFFICIAL_QUANT_MIN_VERSION_NAME: &str = "0.5.0";
pub(crate) const OFFICIAL_QUANT_CHANNEL: &str = "paper";
pub(crate) const OFFICIAL_QUANT_ADMISSION_SCHEMA: &str =
    "yilong.official_quant_release_admission.v1";

const APK_SIGNING_BLOCK_MAGIC: &[u8; 16] = b"APK Sig Block 42";
const APK_SIGNATURE_SCHEME_V2_ID: u32 = 0x7109_871a;
const APK_SIGNATURE_SCHEME_V3_ID: u32 = 0xf053_68c0;
const APK_SIGNATURE_SCHEME_V31_ID: u32 = 0x1b93_ad61;
const ZIP_EOCD_MIN_BYTES: usize = 22;
const ZIP_MAX_COMMENT_BYTES: usize = u16::MAX as usize;
const MAX_APK_ENTRY_COUNT: usize = 4_096;
const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PRIMARY_DEX_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub(crate) struct OfficialQuantReleaseDeclaration<'a> {
    pub project_id: &'a str,
    pub package_name: Option<&'a str>,
    pub version_code: Option<i64>,
    pub version_name: Option<&'a str>,
    pub channel: Option<&'a str>,
    pub source_git_sha: Option<&'a str>,
}

/// Capability token created only after the uploaded APK bytes pass the
/// structural gate. The private field prevents metadata-only store callers
/// from registering an official quant release by accident. Its identity is
/// bound to the exact bytes that were validated so it cannot authorize a
/// different artifact later in the call chain.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ValidatedOfficialQuantApk {
    sha256: String,
    size_bytes: i64,
}

impl ValidatedOfficialQuantApk {
    pub(crate) fn sha256(&self) -> &str {
        &self.sha256
    }

    pub(crate) fn size_bytes(&self) -> i64 {
        self.size_bytes
    }

    pub(crate) fn matches_artifact(&self, sha256: &str, size_bytes: i64) -> bool {
        self.sha256 == sha256 && self.size_bytes == size_bytes
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum OfficialQuantReleaseError {
    #[error("official quant release metadata is incomplete or invalid")]
    InvalidMetadata,
    #[error("official quant APK structure is invalid")]
    InvalidApkStructure,
    #[error("official quant release version would move backwards")]
    VersionRollback,
    #[error("official quant release version already identifies another artifact")]
    VersionConflict,
    #[error("official quant APK bytes are already registered as another version")]
    ArtifactRelabeled,
    #[error("official quant APK proof does not match the registered artifact")]
    ArtifactProofMismatch,
}

impl OfficialQuantReleaseError {
    pub(crate) fn is_conflict(&self) -> bool {
        matches!(
            self,
            Self::VersionRollback | Self::VersionConflict | Self::ArtifactRelabeled
        )
    }
}

pub(crate) fn is_official_quant_project(project_id: &str) -> bool {
    project_id == OFFICIAL_QUANT_PROJECT_ID
}

pub(crate) fn validate_release_declaration(
    declaration: OfficialQuantReleaseDeclaration<'_>,
) -> Result<(), OfficialQuantReleaseError> {
    let version_name = declaration
        .version_name
        .and_then(parse_canonical_semver)
        .ok_or(OfficialQuantReleaseError::InvalidMetadata)?;
    let minimum_version = parse_canonical_semver(OFFICIAL_QUANT_MIN_VERSION_NAME)
        .expect("official minimum version is a canonical semantic version");
    let source_git_sha = declaration
        .source_git_sha
        .ok_or(OfficialQuantReleaseError::InvalidMetadata)?;

    if declaration.project_id != OFFICIAL_QUANT_PROJECT_ID
        || declaration.package_name != Some(OFFICIAL_QUANT_PACKAGE_NAME)
        || declaration.version_code < Some(OFFICIAL_QUANT_MIN_VERSION_CODE)
        || version_name < minimum_version
        || declaration.channel != Some(OFFICIAL_QUANT_CHANNEL)
        || !is_lowercase_git_sha(source_git_sha)
    {
        return Err(OfficialQuantReleaseError::InvalidMetadata);
    }
    Ok(())
}

pub(crate) fn validate_apk_payload(
    payload: &[u8],
) -> Result<ValidatedOfficialQuantApk, OfficialQuantReleaseError> {
    validate_apk_signing_block(payload)?;

    let mut archive = ZipArchive::new(Cursor::new(payload))
        .map_err(|_| OfficialQuantReleaseError::InvalidApkStructure)?;
    if archive.is_empty() || archive.len() > MAX_APK_ENTRY_COUNT {
        return Err(OfficialQuantReleaseError::InvalidApkStructure);
    }
    let mut manifest_entries = 0;
    let mut primary_dex_entries = 0;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|_| OfficialQuantReleaseError::InvalidApkStructure)?;
        match entry.name() {
            "AndroidManifest.xml" => manifest_entries += 1,
            "classes.dex" => primary_dex_entries += 1,
            _ => {}
        }
    }
    if manifest_entries != 1 || primary_dex_entries != 1 {
        return Err(OfficialQuantReleaseError::InvalidApkStructure);
    }

    let (manifest_header, manifest_entry_size) =
        read_entry_prefix(&mut archive, "AndroidManifest.xml", 8, MAX_MANIFEST_BYTES)?;
    let manifest_size =
        read_u32(&manifest_header, 4).ok_or(OfficialQuantReleaseError::InvalidApkStructure)? as u64;
    if manifest_header[..4] != [0x03, 0x00, 0x08, 0x00]
        || manifest_size < 8
        || manifest_size > manifest_entry_size
    {
        return Err(OfficialQuantReleaseError::InvalidApkStructure);
    }

    let (dex_header, _) = read_entry_prefix(&mut archive, "classes.dex", 8, MAX_PRIMARY_DEX_BYTES)?;
    if &dex_header[..4] != b"dex\n"
        || !dex_header[4..7].iter().all(u8::is_ascii_digit)
        || dex_header[7] != 0
    {
        return Err(OfficialQuantReleaseError::InvalidApkStructure);
    }
    let size_bytes =
        i64::try_from(payload.len()).map_err(|_| OfficialQuantReleaseError::InvalidApkStructure)?;
    Ok(ValidatedOfficialQuantApk {
        sha256: format!("{:x}", Sha256::digest(payload)),
        size_bytes,
    })
}

#[cfg(test)]
pub(crate) fn validated_apk_for_test(
    sha256: impl Into<String>,
    size_bytes: i64,
) -> ValidatedOfficialQuantApk {
    ValidatedOfficialQuantApk {
        sha256: sha256.into(),
        size_bytes,
    }
}

fn parse_canonical_semver(value: &str) -> Option<(u64, u64, u64)> {
    if value.is_empty() || value.trim() != value {
        return None;
    }
    let mut parts = value.split('.');
    let mut component = || {
        let part = parts.next()?;
        if part.is_empty()
            || !part.bytes().all(|byte| byte.is_ascii_digit())
            || (part.len() > 1 && part.starts_with('0'))
        {
            return None;
        }
        part.parse::<u64>().ok()
    };
    let parsed = (component()?, component()?, component()?);
    parts.next().is_none().then_some(parsed)
}

fn is_lowercase_git_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn read_entry_prefix(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
    prefix_len: usize,
    max_size: u64,
) -> Result<(Vec<u8>, u64), OfficialQuantReleaseError> {
    let mut entry = archive
        .by_name(name)
        .map_err(|_| OfficialQuantReleaseError::InvalidApkStructure)?;
    if entry.is_dir() || entry.size() < prefix_len as u64 || entry.size() > max_size {
        return Err(OfficialQuantReleaseError::InvalidApkStructure);
    }
    let mut prefix = vec![0; prefix_len];
    entry
        .read_exact(&mut prefix)
        .map_err(|_| OfficialQuantReleaseError::InvalidApkStructure)?;
    let entry_size = entry.size();
    Ok((prefix, entry_size))
}

fn validate_apk_signing_block(payload: &[u8]) -> Result<(), OfficialQuantReleaseError> {
    let eocd_offset =
        zip_eocd_offset(payload).ok_or(OfficialQuantReleaseError::InvalidApkStructure)?;
    let central_directory_size = read_u32(payload, eocd_offset + 12)
        .ok_or(OfficialQuantReleaseError::InvalidApkStructure)?
        as usize;
    let central_directory_offset = read_u32(payload, eocd_offset + 16)
        .ok_or(OfficialQuantReleaseError::InvalidApkStructure)?
        as usize;
    let entries_on_disk =
        read_u16(payload, eocd_offset + 8).ok_or(OfficialQuantReleaseError::InvalidApkStructure)?;
    let total_entries = read_u16(payload, eocd_offset + 10)
        .ok_or(OfficialQuantReleaseError::InvalidApkStructure)?;
    if entries_on_disk == u16::MAX
        || total_entries == u16::MAX
        || entries_on_disk != total_entries
        || read_u32(payload, eocd_offset + 12) == Some(u32::MAX)
        || read_u32(payload, eocd_offset + 16) == Some(u32::MAX)
        || eocd_offset
            .checked_sub(20)
            .and_then(|offset| payload.get(offset..offset + 4))
            == Some(b"PK\x06\x07")
        || central_directory_offset.checked_add(central_directory_size) != Some(eocd_offset)
        || central_directory_offset < 24
        || payload.get(central_directory_offset..central_directory_offset + 4)
            != Some(b"PK\x01\x02")
    {
        return Err(OfficialQuantReleaseError::InvalidApkStructure);
    }

    let footer_offset = central_directory_offset - 24;
    let block_size = read_u64(payload, footer_offset)
        .and_then(|size| usize::try_from(size).ok())
        .ok_or(OfficialQuantReleaseError::InvalidApkStructure)?;
    if payload.get(footer_offset + 8..central_directory_offset) != Some(APK_SIGNING_BLOCK_MAGIC)
        || block_size < 36
    {
        return Err(OfficialQuantReleaseError::InvalidApkStructure);
    }
    let total_block_size = block_size
        .checked_add(8)
        .ok_or(OfficialQuantReleaseError::InvalidApkStructure)?;
    let block_start = central_directory_offset
        .checked_sub(total_block_size)
        .ok_or(OfficialQuantReleaseError::InvalidApkStructure)?;
    if read_u64(payload, block_start) != Some(block_size as u64) {
        return Err(OfficialQuantReleaseError::InvalidApkStructure);
    }

    let mut cursor = block_start + 8;
    let pairs_end = footer_offset;
    let mut has_v2 = false;
    let mut has_v3 = false;
    let mut has_v31 = false;
    while cursor < pairs_end {
        let pair_size = read_u64(payload, cursor)
            .and_then(|size| usize::try_from(size).ok())
            .ok_or(OfficialQuantReleaseError::InvalidApkStructure)?;
        if pair_size <= 4 {
            return Err(OfficialQuantReleaseError::InvalidApkStructure);
        }
        let pair_end = cursor
            .checked_add(8)
            .and_then(|offset| offset.checked_add(pair_size))
            .filter(|offset| *offset <= pairs_end)
            .ok_or(OfficialQuantReleaseError::InvalidApkStructure)?;
        let scheme_id =
            read_u32(payload, cursor + 8).ok_or(OfficialQuantReleaseError::InvalidApkStructure)?;
        let already_seen = match scheme_id {
            APK_SIGNATURE_SCHEME_V2_ID => std::mem::replace(&mut has_v2, true),
            APK_SIGNATURE_SCHEME_V3_ID => std::mem::replace(&mut has_v3, true),
            APK_SIGNATURE_SCHEME_V31_ID => std::mem::replace(&mut has_v31, true),
            _ => false,
        };
        if already_seen {
            return Err(OfficialQuantReleaseError::InvalidApkStructure);
        }
        cursor = pair_end;
    }
    if cursor != pairs_end || !(has_v2 || has_v3) || (has_v31 && !has_v3) {
        return Err(OfficialQuantReleaseError::InvalidApkStructure);
    }
    Ok(())
}

fn zip_eocd_offset(payload: &[u8]) -> Option<usize> {
    if payload.len() < ZIP_EOCD_MIN_BYTES {
        return None;
    }
    let search_start = payload
        .len()
        .saturating_sub(ZIP_EOCD_MIN_BYTES + ZIP_MAX_COMMENT_BYTES);
    for offset in (search_start..=payload.len() - ZIP_EOCD_MIN_BYTES).rev() {
        if payload.get(offset..offset + 4) != Some(b"PK\x05\x06") {
            continue;
        }
        let comment_len = read_u16(payload, offset + 20)? as usize;
        if offset + ZIP_EOCD_MIN_BYTES + comment_len == payload.len()
            && read_u16(payload, offset + 4) == Some(0)
            && read_u16(payload, offset + 6) == Some(0)
        {
            return Some(offset);
        }
    }
    None
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        bytes.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

    fn declaration() -> OfficialQuantReleaseDeclaration<'static> {
        OfficialQuantReleaseDeclaration {
            project_id: OFFICIAL_QUANT_PROJECT_ID,
            package_name: Some(OFFICIAL_QUANT_PACKAGE_NAME),
            version_code: Some(OFFICIAL_QUANT_MIN_VERSION_CODE),
            version_name: Some(OFFICIAL_QUANT_MIN_VERSION_NAME),
            channel: Some(OFFICIAL_QUANT_CHANNEL),
            source_git_sha: Some("0123456789abcdef0123456789abcdef01234567"),
        }
    }

    fn unsigned_apk_fixture() -> Vec<u8> {
        apk_fixture_with_decoys(false, false)
    }

    fn apk_fixture_with_decoys(magic_in_comment: bool, magic_in_entry: bool) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        if magic_in_comment {
            writer.set_comment("APK Sig Block 42").unwrap();
        }
        writer.start_file("AndroidManifest.xml", options).unwrap();
        writer
            .write_all(&[0x03, 0x00, 0x08, 0x00, 0x08, 0x00, 0x00, 0x00])
            .unwrap();
        writer.start_file("classes.dex", options).unwrap();
        writer.write_all(b"dex\n035\0").unwrap();
        if magic_in_entry {
            writer
                .start_file("assets/signing-block-decoy", options)
                .unwrap();
            writer.write_all(APK_SIGNING_BLOCK_MAGIC).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn signed_apk_fixture() -> Vec<u8> {
        let unsigned = unsigned_apk_fixture();
        let eocd = zip_eocd_offset(&unsigned).unwrap();
        let central_directory_offset = read_u32(&unsigned, eocd + 16).unwrap() as usize;
        let value = b"structural-test-signature";
        let pair_size = 4 + value.len();
        let block_size = 8 + pair_size + 24;
        let mut block = Vec::with_capacity(block_size + 8);
        block.extend_from_slice(&(block_size as u64).to_le_bytes());
        block.extend_from_slice(&(pair_size as u64).to_le_bytes());
        block.extend_from_slice(&APK_SIGNATURE_SCHEME_V2_ID.to_le_bytes());
        block.extend_from_slice(value);
        block.extend_from_slice(&(block_size as u64).to_le_bytes());
        block.extend_from_slice(APK_SIGNING_BLOCK_MAGIC);

        let mut signed = Vec::with_capacity(unsigned.len() + block.len());
        signed.extend_from_slice(&unsigned[..central_directory_offset]);
        signed.extend_from_slice(&block);
        signed.extend_from_slice(&unsigned[central_directory_offset..]);
        let signed_eocd = eocd + block.len();
        let updated_offset = (central_directory_offset + block.len()) as u32;
        signed[signed_eocd + 16..signed_eocd + 20].copy_from_slice(&updated_offset.to_le_bytes());
        signed
    }

    #[test]
    fn declaration_requires_exact_new_only_identity() {
        validate_release_declaration(declaration()).unwrap();

        let mut candidate = declaration();
        candidate.project_id = "yilong-quant-old";
        assert_eq!(
            validate_release_declaration(candidate),
            Err(OfficialQuantReleaseError::InvalidMetadata)
        );
        let mut candidate = declaration();
        candidate.package_name = Some("com.elon.quant.debug");
        assert!(validate_release_declaration(candidate).is_err());
        let mut candidate = declaration();
        candidate.version_code = Some(4);
        assert!(validate_release_declaration(candidate).is_err());
        let mut candidate = declaration();
        candidate.version_name = Some("0.4.0");
        assert!(validate_release_declaration(candidate).is_err());
        let mut candidate = declaration();
        candidate.version_name = Some("0.5");
        assert!(validate_release_declaration(candidate).is_err());
        let mut candidate = declaration();
        candidate.channel = Some("live");
        assert!(validate_release_declaration(candidate).is_err());
        let mut candidate = declaration();
        candidate.source_git_sha = Some("ABCDEF0123456789abcdef0123456789abcdef01");
        assert!(validate_release_declaration(candidate).is_err());
    }

    #[test]
    fn apk_payload_requires_manifest_dex_and_canonical_signing_block() {
        validate_apk_payload(&signed_apk_fixture()).unwrap();
        assert_eq!(
            validate_apk_payload(b"not an apk"),
            Err(OfficialQuantReleaseError::InvalidApkStructure)
        );
        assert_eq!(
            validate_apk_payload(&unsigned_apk_fixture()),
            Err(OfficialQuantReleaseError::InvalidApkStructure)
        );

        assert_eq!(
            validate_apk_payload(&apk_fixture_with_decoys(true, false)),
            Err(OfficialQuantReleaseError::InvalidApkStructure)
        );
        assert_eq!(
            validate_apk_payload(&apk_fixture_with_decoys(false, true)),
            Err(OfficialQuantReleaseError::InvalidApkStructure)
        );
    }

    #[test]
    fn malformed_signing_block_lengths_fail_closed() {
        let mut apk = signed_apk_fixture();
        let eocd = zip_eocd_offset(&apk).unwrap();
        let central_directory_offset = read_u32(&apk, eocd + 16).unwrap() as usize;
        apk[central_directory_offset - 24..central_directory_offset - 16]
            .copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(
            validate_apk_payload(&apk),
            Err(OfficialQuantReleaseError::InvalidApkStructure)
        );
    }

    #[test]
    fn conflict_classification_is_stable() {
        assert!(OfficialQuantReleaseError::VersionRollback.is_conflict());
        assert!(OfficialQuantReleaseError::VersionConflict.is_conflict());
        assert!(OfficialQuantReleaseError::ArtifactRelabeled.is_conflict());
        assert!(!OfficialQuantReleaseError::ArtifactProofMismatch.is_conflict());
        assert!(!OfficialQuantReleaseError::InvalidMetadata.is_conflict());
        assert!(!OfficialQuantReleaseError::InvalidApkStructure.is_conflict());
    }
}
