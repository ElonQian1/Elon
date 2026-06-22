use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::{now, Store};

const MAX_DETECTED_FILES: usize = 16;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectDevProfile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_command: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub detected_files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl ProjectDevProfile {
    pub fn is_empty(&self) -> bool {
        self.project_type
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
            && self
                .package_manager
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            && self
                .run_command
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            && self
                .test_command
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            && self
                .build_command
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            && self.detected_files.is_empty()
    }
}

impl Store {
    pub fn upsert_project_dev_profile(
        &self,
        user_id: &str,
        project_id: &str,
        profile: &ProjectDevProfile,
    ) -> Result<Option<ProjectDevProfile>> {
        let mut profile = sanitize_profile(profile);
        if profile.is_empty() {
            return Ok(None);
        }

        let updated_at = now();
        profile.updated_at = Some(updated_at.clone());
        let detected_files_json = serde_json::to_string(&profile.detected_files)?;
        let conn = self.conn()?;
        let changed = conn.execute(
            "INSERT INTO project_dev_profiles (
                project_id, project_type, package_manager, run_command, test_command,
                build_command, detected_files_json, source, updated_at
             )
             SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9
             WHERE EXISTS (
                SELECT 1 FROM project_members pm
                JOIN projects p ON p.id = pm.project_id
                WHERE pm.project_id = ?1
                  AND pm.user_id = ?10
                  AND pm.role IN ('owner', 'admin', 'editor')
                  AND p.status != 'deleted'
             )
             ON CONFLICT(project_id) DO UPDATE SET
                project_type = excluded.project_type,
                package_manager = excluded.package_manager,
                run_command = excluded.run_command,
                test_command = excluded.test_command,
                build_command = excluded.build_command,
                detected_files_json = excluded.detected_files_json,
                source = excluded.source,
                updated_at = excluded.updated_at",
            params![
                project_id,
                profile.project_type,
                profile.package_manager,
                profile.run_command,
                profile.test_command,
                profile.build_command,
                detected_files_json,
                profile.source,
                updated_at,
                user_id
            ],
        )?;
        if changed == 0 {
            return Err(anyhow!("项目不存在，或当前用户无权保存项目开发命令"));
        }
        drop(conn);

        self.get_project_dev_profile_for_user(user_id, project_id)
    }

    pub fn get_project_dev_profile_for_user(
        &self,
        user_id: &str,
        project_id: &str,
    ) -> Result<Option<ProjectDevProfile>> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT pdp.project_type, pdp.package_manager, pdp.run_command,
                    pdp.test_command, pdp.build_command, pdp.detected_files_json,
                    pdp.source, pdp.updated_at
             FROM project_dev_profiles pdp
             JOIN projects p ON p.id = pdp.project_id
             JOIN project_members pm ON pm.project_id = p.id
             WHERE pdp.project_id = ?1
               AND pm.user_id = ?2
               AND p.status != 'deleted'",
            params![project_id, user_id],
            project_dev_profile_from_row,
        )
        .optional()
        .map_err(Into::into)
    }
}

fn sanitize_profile(profile: &ProjectDevProfile) -> ProjectDevProfile {
    ProjectDevProfile {
        project_type: clean_text(profile.project_type.as_deref(), 80),
        package_manager: clean_text(profile.package_manager.as_deref(), 80),
        run_command: clean_command(profile.run_command.as_deref()),
        test_command: clean_command(profile.test_command.as_deref()),
        build_command: clean_command(profile.build_command.as_deref()),
        detected_files: clean_detected_files(&profile.detected_files),
        source: clean_text(profile.source.as_deref(), 80).or_else(|| {
            if profile.is_empty() {
                None
            } else {
                Some("node_agent_project_picker".to_string())
            }
        }),
        updated_at: None,
    }
}

fn clean_text(value: Option<&str>, max_chars: usize) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(max_chars).collect())
}

fn clean_command(value: Option<&str>) -> Option<String> {
    clean_text(value, 240)
}

fn clean_detected_files(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| clean_text(Some(value.as_str()), 120))
        .take(MAX_DETECTED_FILES)
        .collect()
}

fn project_dev_profile_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectDevProfile> {
    let detected_files_json: String = row.get(5)?;
    let detected_files = serde_json::from_str::<Vec<String>>(&detected_files_json)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| clean_text(Some(value.as_str()), 120))
        .collect();
    Ok(ProjectDevProfile {
        project_type: row.get(0)?,
        package_manager: row.get(1)?,
        run_command: row.get(2)?,
        test_command: row.get(3)?,
        build_command: row.get(4)?,
        detected_files,
        source: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_store_project_dev_profile_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn upsert_project_dev_profile_round_trips_detected_commands() {
        let store = temp_store();
        let user = store
            .create_user("project-profile-owner@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .register_external_project(
                &user.id,
                None,
                "Profiled Project",
                None,
                r"D:\rust\active-projects\profiled",
                Some("node-a"),
                Some("https://example.com/profiled.git"),
                Some("main"),
            )
            .expect("project should register");

        let saved = store
            .upsert_project_dev_profile(
                &user.id,
                &project.project.id,
                &ProjectDevProfile {
                    project_type: Some("Node.js".to_string()),
                    package_manager: Some("pnpm".to_string()),
                    run_command: Some("pnpm dev".to_string()),
                    test_command: Some("pnpm test".to_string()),
                    build_command: Some("pnpm build".to_string()),
                    detected_files: vec!["package.json".to_string(), "pnpm-lock.yaml".to_string()],
                    source: None,
                    updated_at: None,
                },
            )
            .expect("profile should save")
            .expect("profile should be non-empty");

        assert_eq!(saved.project_type.as_deref(), Some("Node.js"));
        assert_eq!(saved.package_manager.as_deref(), Some("pnpm"));
        assert_eq!(saved.test_command.as_deref(), Some("pnpm test"));
        assert_eq!(saved.source.as_deref(), Some("node_agent_project_picker"));
        assert!(saved.updated_at.is_some());
        assert_eq!(saved.detected_files.len(), 2);
    }
}
