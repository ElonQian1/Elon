use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use crate::store::{is_system_project_name, is_system_project_source_type, system_project_key_for_source_type};

use super::common::{clean_optional, hash_password, new_id, normalize_account, now, safe_external_id, validate_password, verify_password};
use super::{project_identities, pc_project_binding, project_branding, project_roles, project_runtime_permissions};
use super::project_helpers::*;
use super::store_types::*;
use super::store_types_project::*;

impl super::Store {
    pub fn create_project(
        &self,
        user_id: &str,
        name: &str,
        description: Option<&str>,
        template: Option<&str>,
    ) -> Result<CreateProjectResult> {
        let name = name.trim();
        if name.is_empty() {
            return Err(anyhow!("项目名称不能为空"));
        }
        if is_system_project_name(name) {
            return Err(anyhow!("该名称是系统保留项目，请更换项目名称"));
        }

        let template = template
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("android");
        // 目前所有受支持的模板都按 Android 脚手架处理；未来扩展时再细分。
        let template = match template {
            "android" | "android_kotlin" | "android_compose" => "android",
            _ => return Err(anyhow!("目前只支持 android 模板")),
        };

        let now = now();
        let conn = self.conn()?;

        if let Some(project) = find_owner_project_by_name(&conn, user_id, name)? {
            return Ok(CreateProjectResult {
                project,
                reused_existing: true,
            });
        }

        let id = new_id("prj");
        let workspace_key = id.clone();
        let description = clean_optional(description);
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO projects (
                id, name, description, workspace_key, template, source_type,
                status, created_by, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, 'template', 'active', ?6, ?7, ?7)",
            params![id, name, description, workspace_key, template, user_id, now],
        )?;
        tx.execute(
            "INSERT INTO project_members (project_id, user_id, role, created_at)
             VALUES (?1, ?2, 'owner', ?3)",
            params![id, user_id, now],
        )?;
        tx.execute(
            "INSERT INTO project_events (id, project_id, user_id, event_type, payload_json, created_at)
             VALUES (?1, ?2, ?3, 'project_created', ?4, ?5)",
            params![
                new_id("evt"),
                id,
                user_id,
                serde_json::json!({ "name": name, "template": template }).to_string(),
                now
            ],
        )?;
        tx.commit()?;

        let mut project = ProjectSummary {
            id,
            name: name.to_string(),
            display_name: None,
            description: description.map(ToOwned::to_owned),
            workspace_key,
            template: template.to_string(),
            source_type: "template".into(),
            repo_url: None,
            branch: None,
            workspace_path: None,
            node_id: None,
            storage_node_id: None,
            storage_repo_path: None,
            storage_repo_url: None,
            storage_worktree_path: None,
            storage_status: "none".into(),
            status: "active".into(),
            role: "owner".into(),
            member_count: 1,
            is_public: false,
            join_mode: "open".into(),
            runtime_permission: default_project_runtime_permission(),
            last_task_status: None,
            last_apk_url: None,
            icon_data_url: None,
            updated_at: now,
        };
        project_branding::apply_project_summary_branding(&mut project);

        Ok(CreateProjectResult {
            project,
            reused_existing: false,
        })
    }

    /// 注册一个指向外部本地路径的项目（如 D:\rust\active-projects\bb64a）。
    /// source_type='local_path'，workspace_path 写入项目记录。
    /// 同一代码身份优先复用现有记录（reused_existing=true）。
    pub fn register_external_project(
        &self,
        user_id: &str,
        project_id: Option<&str>,
        name: &str,
        description: Option<&str>,
        workspace_path: &str,
        node_id: Option<&str>,
        repo_url: Option<&str>,
        branch: Option<&str>,
    ) -> Result<CreateProjectResult> {
        let name = name.trim();
        if name.is_empty() {
            return Err(anyhow!("项目名称不能为空"));
        }
        if is_system_project_name(name) {
            return Err(anyhow!("该名称是系统保留项目，不能绑定为外部代码项目"));
        }
        let workspace_path = workspace_path.trim();
        if workspace_path.is_empty() {
            return Err(anyhow!("workspace_path 不能为空"));
        }
        let template = "local";
        let source_type = "local_path";
        let node_id = clean_optional(node_id);
        let repo_url = clean_optional(repo_url);
        let branch = clean_optional(branch);

        let now = now();
        let conn = self.conn()?;
        let identity_candidates =
            project_identities::identity_candidates(node_id, workspace_path, repo_url, branch);

        let requested_project_id = project_id.map(str::trim).filter(|v| !v.is_empty());
        if let Some(project_id) = requested_project_id {
            let (role, source_type): (String, String) = conn
                .query_row(
                    "SELECT pm.role, p.source_type
                     FROM projects p
                     JOIN project_members pm ON pm.project_id = p.id
                     WHERE p.id = ?1 AND pm.user_id = ?2 AND p.status != 'deleted'",
                    params![project_id, user_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?
                .ok_or_else(|| anyhow!("项目不存在，或当前用户无权访问"))?;
            if role != "owner" {
                anyhow::bail!("只有项目 owner 才能绑定 PC 本地路径");
            }
            if is_system_project_source_type(&source_type) {
                anyhow::bail!("系统归档项目不能绑定为外部代码工作区");
            }
            if let Some(existing) =
                find_owner_project_by_workspace_path(&conn, user_id, workspace_path)?
            {
                if existing.id != project_id {
                    let display = existing
                        .display_name
                        .as_deref()
                        .unwrap_or(existing.name.as_str());
                    anyhow::bail!("该本地路径已绑定到项目「{}」，请直接打开该项目", display);
                }
            }
            if let Some(existing) = project_identities::find_owner_project_by_identity(
                &conn,
                user_id,
                &identity_candidates,
            )? {
                if existing.id != project_id {
                    return Err(project_identities::identity_conflict_error(&existing));
                }
            }
            if let Some(existing) =
                project_identities::find_owner_project_by_git_remote(&conn, user_id, repo_url)?
            {
                if existing.id != project_id {
                    return Err(project_identities::identity_conflict_error(&existing));
                }
            }

            let project = update_external_project_binding(
                &conn,
                user_id,
                project_id,
                Some(name),
                clean_optional(description),
                workspace_path,
                node_id,
                repo_url,
                branch,
                &now,
                "project_bound_external",
            )?;
            return Ok(CreateProjectResult {
                project,
                reused_existing: true,
            });
        }

        if let Some(project) = project_identities::find_owner_project_by_identity(
            &conn,
            user_id,
            &identity_candidates,
        )? {
            let project = update_external_project_binding(
                &conn,
                user_id,
                &project.id,
                None,
                None,
                workspace_path,
                node_id,
                repo_url,
                branch,
                &now,
                "project_reused_external_identity",
            )?;
            return Ok(CreateProjectResult {
                project,
                reused_existing: true,
            });
        }
        if let Some(project) =
            project_identities::find_owner_project_by_git_remote(&conn, user_id, repo_url)?
        {
            let project = update_external_project_binding(
                &conn,
                user_id,
                &project.id,
                None,
                None,
                workspace_path,
                node_id,
                repo_url,
                branch,
                &now,
                "project_reused_external_git_remote",
            )?;
            return Ok(CreateProjectResult {
                project,
                reused_existing: true,
            });
        }
        if let Some(project) = find_owner_project_by_workspace_path(&conn, user_id, workspace_path)?
        {
            let project = update_external_project_binding(
                &conn,
                user_id,
                &project.id,
                None,
                None,
                workspace_path,
                node_id,
                repo_url,
                branch,
                &now,
                "project_reused_external_path",
            )?;
            return Ok(CreateProjectResult {
                project,
                reused_existing: true,
            });
        }
        if let Some(project) = find_owner_project_by_name(&conn, user_id, name)? {
            let project = update_external_project_binding(
                &conn,
                user_id,
                &project.id,
                None,
                None,
                workspace_path,
                node_id,
                repo_url,
                branch,
                &now,
                "project_reused_external_name",
            )?;
            return Ok(CreateProjectResult {
                project,
                reused_existing: true,
            });
        }

        let id = new_id("prj");
        let workspace_key = id.clone();
        let description = clean_optional(description);
        let create_result = {
            let tx = conn.unchecked_transaction()?;
            tx.execute(
                "INSERT INTO projects (
                    id, name, description, workspace_key, template, source_type, repo_url, branch,
                    workspace_path, node_id,
                    status, created_by, created_at, updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'active', ?11, ?12, ?12)",
                params![
                    id,
                    name,
                    description,
                    workspace_key,
                    template,
                    source_type,
                    repo_url,
                    branch,
                    workspace_path,
                    node_id,
                    user_id,
                    now
                ],
            )?;
            tx.execute(
                "INSERT INTO project_members (project_id, user_id, role, created_at)
                 VALUES (?1, ?2, 'owner', ?3)",
                params![id, user_id, now],
            )?;
            tx.execute(
                "INSERT INTO project_events (id, project_id, user_id, event_type, payload_json, created_at)
                 VALUES (?1, ?2, ?3, 'project_registered_external', ?4, ?5)",
                params![
                    new_id("evt"),
                    id,
                    user_id,
                    serde_json::json!({
                        "name": name,
                        "workspace_path": workspace_path,
                        "node_id": node_id,
                        "repo_url": repo_url,
                        "branch": branch,
                    })
                    .to_string(),
                    now
                ],
            )?;
            project_identities::replace_project_identities(
                &tx,
                &id,
                user_id,
                node_id,
                workspace_path,
                repo_url,
                branch,
                &now,
            )?;
            if let Some(node_id) = node_id {
                pc_project_binding::upsert_project_pc_workspace_binding_tx(
                    &tx,
                    &id,
                    user_id,
                    node_id,
                    workspace_path,
                    None,
                    repo_url,
                    branch,
                    "register_external_project",
                    &now,
                )?;
            }
            tx.commit()?;
            Ok::<(), anyhow::Error>(())
        };
        if let Err(err) = create_result {
            if let Some(project) = project_identities::find_owner_project_by_identity(
                &conn,
                user_id,
                &identity_candidates,
            )? {
                let project = update_external_project_binding(
                    &conn,
                    user_id,
                    &project.id,
                    None,
                    None,
                    workspace_path,
                    node_id,
                    repo_url,
                    branch,
                    &now,
                    "project_reused_external_identity",
                )?;
                return Ok(CreateProjectResult {
                    project,
                    reused_existing: true,
                });
            }
            if let Some(project) = find_owner_project_by_name(&conn, user_id, name)? {
                let project = update_external_project_binding(
                    &conn,
                    user_id,
                    &project.id,
                    None,
                    None,
                    workspace_path,
                    node_id,
                    repo_url,
                    branch,
                    &now,
                    "project_reused_external_name",
                )?;
                return Ok(CreateProjectResult {
                    project,
                    reused_existing: true,
                });
            }
            return Err(err);
        }

        let mut project = ProjectSummary {
            id,
            name: name.to_string(),
            display_name: None,
            description: description.map(ToOwned::to_owned),
            workspace_key,
            template: template.to_string(),
            source_type: source_type.into(),
            repo_url: repo_url.map(ToOwned::to_owned),
            branch: branch.map(ToOwned::to_owned),
            workspace_path: Some(workspace_path.to_string()),
            node_id: node_id.map(ToOwned::to_owned),
            storage_node_id: None,
            storage_repo_path: None,
            storage_repo_url: None,
            storage_worktree_path: None,
            storage_status: "none".into(),
            status: "active".into(),
            role: "owner".into(),
            member_count: 1,
            is_public: false,
            join_mode: "open".into(),
            runtime_permission: default_project_runtime_permission(),
            last_task_status: None,
            last_apk_url: None,
            icon_data_url: None,
            updated_at: now,
        };
        project_branding::apply_project_summary_branding(&mut project);

        Ok(CreateProjectResult {
            project,
            reused_existing: false,
        })
    }

}
