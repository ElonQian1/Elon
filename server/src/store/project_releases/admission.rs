use anyhow::Result;
use rusqlite::{params, TransactionBehavior};

use super::{
    project_release_from_row, release_file_name, ProjectRelease, ProjectReleaseAdmissionOutcome,
    ProjectReleaseWrite, PROJECT_RELEASE_SELECT,
};
use crate::{
    project_releases::admission::{
        validate_release_declaration, OfficialQuantReleaseDeclaration, OfficialQuantReleaseError,
        ValidatedOfficialQuantApk, OFFICIAL_QUANT_ADMISSION_SCHEMA, OFFICIAL_QUANT_PROJECT_ID,
    },
    store::{clean_optional, new_id, now, Store},
};

pub(super) fn create_official_quant_release(
    store: &Store,
    write: ProjectReleaseWrite<'_>,
    official_apk: &ValidatedOfficialQuantApk,
) -> Result<ProjectReleaseAdmissionOutcome> {
    validate_official_write(&write)?;

    let id = clean_optional(write.id)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| new_id("rel"));
    let channel = clean_optional(write.channel).expect("validated channel");
    let status = clean_optional(write.status).expect("validated status");
    let sha256 = clean_optional(write.sha256).expect("validated server digest");
    let size_bytes = write.size_bytes.expect("validated artifact size");
    if !official_apk.matches_artifact(sha256, size_bytes) {
        return Err(OfficialQuantReleaseError::ArtifactProofMismatch.into());
    }
    let version_code = write.version_code.expect("validated version code");
    let admission_metadata = serde_json::json!({
        "schema": OFFICIAL_QUANT_ADMISSION_SCHEMA,
        "apk_signing_block_structure_present": true,
        "cryptographic_signature_verified": false,
        "artifact_sha256": sha256,
        "artifact_size_bytes": size_bytes,
    })
    .to_string();
    let now = now();

    let mut conn = store.conn()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let existing = official_quant_releases(&tx)?;

    if let Some(release) = existing
        .iter()
        .find(|release| release.version_code == Some(version_code))
    {
        if release_identity_matches(release, &write, sha256, size_bytes) {
            return Ok(ProjectReleaseAdmissionOutcome {
                release: release.clone(),
                idempotent_replay: true,
            });
        }
        return Err(OfficialQuantReleaseError::VersionConflict.into());
    }
    if existing
        .iter()
        .any(|release| release.sha256.as_deref() == Some(sha256))
    {
        return Err(OfficialQuantReleaseError::ArtifactRelabeled.into());
    }
    if existing
        .iter()
        .filter(|release| official_quant_release_is_installable(release))
        .filter_map(|release| release.version_code)
        .max()
        .is_some_and(|latest| version_code < latest)
    {
        return Err(OfficialQuantReleaseError::VersionRollback.into());
    }

    let release_number: i64 = tx.query_row(
        "SELECT COALESCE(MAX(release_number), 0) + 1
         FROM project_releases
         WHERE project_id = ?1",
        params![write.project_id],
        |row| row.get(0),
    )?;
    tx.execute(
        "INSERT INTO project_releases (
           id, project_id, task_id, uploaded_by, release_number,
           version_name, package_name, version_code, channel, status,
           apk_url, file_name, file_path, sha256, size_bytes, changelog,
           build_started_at, source_git_sha, source_worktree, metadata_json,
           created_at, updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                 ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?21)",
        params![
            id,
            write.project_id,
            clean_optional(write.task_id),
            clean_optional(write.uploaded_by),
            release_number,
            clean_optional(write.version_name),
            clean_optional(write.package_name),
            version_code,
            channel,
            status,
            write.apk_url.trim(),
            release_file_name(write.file_name),
            clean_optional(write.file_path),
            sha256,
            size_bytes,
            clean_optional(write.changelog),
            clean_optional(write.build_started_at),
            clean_optional(write.source_git_sha),
            clean_optional(write.source_worktree),
            admission_metadata,
            now,
        ],
    )?;
    let select = format!("{PROJECT_RELEASE_SELECT} WHERE id = ?1");
    let release = tx.query_row(&select, params![id], project_release_from_row)?;
    tx.commit()?;
    drop(conn);

    if let Err(error) = store.sync_project_landing_download_from_release(&release) {
        tracing::warn!(
            project_id = %release.project_id,
            release_id = %release.id,
            error = %error,
            "failed to sync admitted official quant release into landing snapshot"
        );
    }
    Ok(ProjectReleaseAdmissionOutcome {
        release,
        idempotent_replay: false,
    })
}

pub(super) fn latest_installable_official_quant_release(
    store: &Store,
) -> Result<Option<ProjectRelease>> {
    let sql = format!(
        "{PROJECT_RELEASE_SELECT} WHERE project_id = ?1
         ORDER BY COALESCE(version_code, 0) DESC,
                  COALESCE(release_number, 0) DESC,
                  updated_at DESC,
                  created_at DESC"
    );
    let conn = store.conn()?;
    let mut statement = conn.prepare(&sql)?;
    let releases =
        statement.query_map(params![OFFICIAL_QUANT_PROJECT_ID], project_release_from_row)?;
    for release in releases {
        let release = release?;
        if official_quant_release_is_installable(&release) {
            return Ok(Some(release));
        }
    }
    Ok(None)
}

pub(super) fn official_quant_release_is_installable(release: &ProjectRelease) -> bool {
    release.status == "published"
        && clean_optional(release.file_path.as_deref()).is_some()
        && clean_optional(Some(release.apk_url.as_str())).is_some()
        && clean_optional(Some(release.file_name.as_str()))
            .is_some_and(|name| name.to_ascii_lowercase().ends_with(".apk"))
        && release.size_bytes.is_some_and(|size| size > 0)
        && release.sha256.as_deref().is_some_and(is_sha256)
        && has_server_admission_receipt(
            release.metadata_json.as_deref(),
            release.sha256.as_deref(),
            release.size_bytes,
        )
        && validate_release_declaration(OfficialQuantReleaseDeclaration {
            project_id: release.project_id.as_str(),
            package_name: release.package_name.as_deref(),
            version_code: release.version_code,
            version_name: release.version_name.as_deref(),
            channel: Some(release.channel.as_str()),
            source_git_sha: release.source_git_sha.as_deref(),
        })
        .is_ok()
}

fn validate_official_write(
    write: &ProjectReleaseWrite<'_>,
) -> std::result::Result<(), OfficialQuantReleaseError> {
    validate_release_declaration(OfficialQuantReleaseDeclaration {
        project_id: write.project_id,
        package_name: clean_optional(write.package_name),
        version_code: write.version_code,
        version_name: clean_optional(write.version_name),
        channel: clean_optional(write.channel),
        source_git_sha: clean_optional(write.source_git_sha),
    })?;
    if clean_optional(write.status) != Some("published")
        || clean_optional(Some(write.apk_url)).is_none()
        || clean_optional(Some(write.file_name))
            .is_none_or(|name| !name.to_ascii_lowercase().ends_with(".apk"))
        || clean_optional(write.file_path).is_none()
        || write.sha256.is_none_or(|sha| !is_sha256(sha))
        || write.size_bytes.is_none_or(|size| size <= 0)
    {
        return Err(OfficialQuantReleaseError::InvalidMetadata);
    }
    Ok(())
}

fn release_identity_matches(
    release: &ProjectRelease,
    write: &ProjectReleaseWrite<'_>,
    sha256: &str,
    size_bytes: i64,
) -> bool {
    official_quant_release_is_installable(release)
        && release.package_name.as_deref() == clean_optional(write.package_name)
        && release.version_name.as_deref() == clean_optional(write.version_name)
        && release.version_code == write.version_code
        && release.channel == clean_optional(write.channel).unwrap_or_default()
        && release.source_git_sha.as_deref() == clean_optional(write.source_git_sha)
        && release.sha256.as_deref() == Some(sha256)
        && release.size_bytes == Some(size_bytes)
}

fn official_quant_releases(conn: &rusqlite::Connection) -> Result<Vec<ProjectRelease>> {
    let sql = format!(
        "{PROJECT_RELEASE_SELECT} WHERE project_id = ?1
         ORDER BY COALESCE(release_number, 0) DESC, updated_at DESC, created_at DESC"
    );
    let mut statement = conn.prepare(&sql)?;
    let releases = statement
        .query_map(params![OFFICIAL_QUANT_PROJECT_ID], project_release_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(releases)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn has_server_admission_receipt(
    metadata_json: Option<&str>,
    sha256: Option<&str>,
    size_bytes: Option<i64>,
) -> bool {
    metadata_json
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .is_some_and(|value| {
            value.get("schema").and_then(serde_json::Value::as_str)
                == Some(OFFICIAL_QUANT_ADMISSION_SCHEMA)
                && value
                    .get("apk_signing_block_structure_present")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true)
                && value
                    .get("cryptographic_signature_verified")
                    .and_then(serde_json::Value::as_bool)
                    == Some(false)
                && value
                    .get("artifact_sha256")
                    .and_then(serde_json::Value::as_str)
                    == sha256
                && value
                    .get("artifact_size_bytes")
                    .and_then(serde_json::Value::as_i64)
                    == size_bytes
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_quant_release_admission_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("official quant release test store should open")
    }

    fn write<'a>(
        id: &'a str,
        version_name: &'a str,
        version_code: i64,
        source_git_sha: &'a str,
        sha256: &'a str,
    ) -> ProjectReleaseWrite<'a> {
        ProjectReleaseWrite {
            id: Some(id),
            project_id: OFFICIAL_QUANT_PROJECT_ID,
            task_id: None,
            uploaded_by: Some("owner"),
            version_name: Some(version_name),
            package_name: Some("com.elon.quant"),
            version_code: Some(version_code),
            channel: Some("paper"),
            status: Some("published"),
            apk_url: "http://example.test/api/projects/yilong-quant/download/latest.apk",
            file_name: "YilongQuant-release.apk",
            file_path: Some("C:/managed/yilong-quant/release.apk"),
            sha256: Some(sha256),
            size_bytes: Some(1024),
            changelog: None,
            build_started_at: None,
            source_git_sha: Some(source_git_sha),
            source_worktree: None,
            metadata_json: None,
        }
    }

    fn proof(sha256: &str) -> ValidatedOfficialQuantApk {
        crate::project_releases::admission::validated_apk_for_test(sha256, 1024)
    }

    #[test]
    fn versions_advance_and_replays_are_idempotent() {
        let store = temp_store();
        let sha5 = "5".repeat(64);
        let git5 = "a".repeat(40);
        let proof5 = proof(&sha5);
        let first = store
            .create_project_release_with_admission(
                write("rel_v5", "0.5.0", 5, &git5, &sha5),
                Some(&proof5),
            )
            .unwrap();
        assert!(!first.idempotent_replay);

        let replay = store
            .create_project_release_with_admission(
                write("rel_retry", "0.5.0", 5, &git5, &sha5),
                Some(&proof5),
            )
            .unwrap();
        assert!(replay.idempotent_replay);
        assert_eq!(replay.release.id, "rel_v5");

        let sha6 = "6".repeat(64);
        let git6 = "b".repeat(40);
        let proof6 = proof(&sha6);
        let next = store
            .create_project_release_with_admission(
                write("rel_v6", "0.6.0", 6, &git6, &sha6),
                Some(&proof6),
            )
            .unwrap();
        assert_eq!(next.release.release_number, Some(2));
        assert_eq!(
            store
                .list_project_releases(OFFICIAL_QUANT_PROJECT_ID, 10)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn rollback_version_conflict_and_relabel_fail_without_release_number_holes() {
        let store = temp_store();
        let sha6 = "6".repeat(64);
        let git6 = "b".repeat(40);
        let proof6 = proof(&sha6);
        store
            .create_project_release_with_admission(
                write("rel_v6", "0.6.0", 6, &git6, &sha6),
                Some(&proof6),
            )
            .unwrap();

        let rollback_sha = "5".repeat(64);
        let rollback_git = "a".repeat(40);
        let rollback_proof = proof(&rollback_sha);
        let rollback = store
            .create_project_release_with_admission(
                write("rel_v5", "0.5.0", 5, &rollback_git, &rollback_sha),
                Some(&rollback_proof),
            )
            .unwrap_err();
        assert!(matches!(
            rollback.downcast_ref::<OfficialQuantReleaseError>(),
            Some(OfficialQuantReleaseError::VersionRollback)
        ));

        let conflict_sha = "7".repeat(64);
        let conflict_proof = proof(&conflict_sha);
        let conflict = store
            .create_project_release_with_admission(
                write("rel_v6_other", "0.6.0", 6, &git6, &conflict_sha),
                Some(&conflict_proof),
            )
            .unwrap_err();
        assert!(matches!(
            conflict.downcast_ref::<OfficialQuantReleaseError>(),
            Some(OfficialQuantReleaseError::VersionConflict)
        ));

        let relabel = store
            .create_project_release_with_admission(
                write("rel_v7", "0.7.0", 7, &git6, &sha6),
                Some(&proof6),
            )
            .unwrap_err();
        assert!(matches!(
            relabel.downcast_ref::<OfficialQuantReleaseError>(),
            Some(OfficialQuantReleaseError::ArtifactRelabeled)
        ));

        let sha7 = "8".repeat(64);
        let git7 = "c".repeat(40);
        let proof7 = proof(&sha7);
        let accepted = store
            .create_project_release_with_admission(
                write("rel_v7_ok", "0.7.0", 7, &git7, &sha7),
                Some(&proof7),
            )
            .unwrap();
        assert_eq!(accepted.release.release_number, Some(2));
    }

    #[test]
    fn proof_is_bound_to_the_exact_artifact_identity() {
        let store = temp_store();
        let proof_sha = "5".repeat(64);
        let write_sha = "6".repeat(64);
        let source_git_sha = "a".repeat(40);
        let proof = proof(&proof_sha);

        let error = store
            .create_project_release_with_admission(
                write("rel_mismatch", "0.5.0", 5, &source_git_sha, &write_sha),
                Some(&proof),
            )
            .unwrap_err();
        assert!(matches!(
            error.downcast_ref::<OfficialQuantReleaseError>(),
            Some(OfficialQuantReleaseError::ArtifactProofMismatch)
        ));
        assert!(store
            .list_project_releases(OFFICIAL_QUANT_PROJECT_ID, 10)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn old_audit_records_are_not_installable_or_latest() {
        let store = temp_store();
        let conn = store.conn().unwrap();
        conn.execute(
            "INSERT INTO project_releases (
               id, project_id, release_number, version_name, package_name, version_code,
               channel, status, apk_url, file_name, file_path, sha256, size_bytes,
               source_git_sha, created_at, updated_at
             ) VALUES (
               'rel_old', 'yilong-quant', 1, '0.2.0', 'com.elon.quant', 2,
               'paper', 'published', 'http://old.test/latest.apk', 'old.apk',
               'C:/managed/old.apk', ?1, 512, ?2, '2026-01-01', '2026-01-01'
             )",
            params!["2".repeat(64), "2".repeat(40)],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO project_releases (
               id, project_id, release_number, version_name, package_name, version_code,
               channel, status, apk_url, file_name, file_path, sha256, size_bytes,
               source_git_sha, created_at, updated_at
             ) VALUES (
               'rel_forged_v5', 'yilong-quant', 2, '0.5.0', 'com.elon.quant', 5,
               'paper', 'published', 'http://old.test/latest-v5.apk', 'forged-v5.apk',
               'C:/managed/forged-v5.apk', ?1, 1024, ?2, '2026-02-01', '2026-02-01'
             )",
            params!["5".repeat(64), "5".repeat(40)],
        )
        .unwrap();
        drop(conn);

        assert!(store
            .latest_project_release(OFFICIAL_QUANT_PROJECT_ID)
            .unwrap()
            .is_none());
        assert!(store
            .project_release_for_download(OFFICIAL_QUANT_PROJECT_ID, "old.apk")
            .unwrap()
            .is_none());
        assert!(store
            .project_release_for_download(OFFICIAL_QUANT_PROJECT_ID, "forged-v5.apk")
            .unwrap()
            .is_none());
        assert!(store.project_release("rel_old").is_ok());
    }

    #[test]
    fn non_official_projects_keep_generic_release_behavior() {
        let store = temp_store();
        let release = store
            .create_project_release(ProjectReleaseWrite {
                id: Some("rel_generic"),
                project_id: "another-project",
                task_id: None,
                uploaded_by: None,
                version_name: None,
                package_name: None,
                version_code: None,
                channel: None,
                status: None,
                apk_url: "http://example.test/another.apk",
                file_name: "another.apk",
                file_path: Some("C:/managed/another.apk"),
                sha256: None,
                size_bytes: None,
                changelog: None,
                build_started_at: None,
                source_git_sha: None,
                source_worktree: None,
                metadata_json: None,
            })
            .unwrap();
        assert_eq!(release.channel, "internal");
        assert_eq!(release.status, "published");
    }
}
