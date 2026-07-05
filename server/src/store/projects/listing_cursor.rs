use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rusqlite::{params, Row};
use serde::{Deserialize, Serialize};

use super::super::project_roles::project_member_effective_role_locked;
use super::super::{project_branding, PublicProjectItem, Store};

const STORE_CURSOR_VERSION: u8 = 1;

#[derive(Debug)]
pub(crate) struct PublicProjectListPage {
    pub(crate) projects: Vec<PublicProjectItem>,
    pub(crate) next_cursor: Option<String>,
    pub(crate) has_more: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PublicProjectSort {
    Updated,
    Created,
    Members,
}

impl PublicProjectSort {
    fn from_query(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some("created") => Self::Created,
            Some("members") => Self::Members,
            _ => Self::Updated,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Updated => "updated",
            Self::Created => "created",
            Self::Members => "members",
        }
    }

    fn order_by(self) -> &'static str {
        match self {
            Self::Updated => "updated_at DESC, id DESC",
            Self::Created => "created_at DESC, id DESC",
            Self::Members => "member_count DESC, updated_at DESC, id DESC",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoreProjectCursor {
    v: u8,
    sort: String,
    id: String,
    updated_at: Option<String>,
    created_at: Option<String>,
    member_count: Option<i64>,
}

impl Store {
    pub(crate) fn list_public_projects_cursor_page_for_viewer(
        &self,
        search: Option<&str>,
        join_mode: Option<&str>,
        has_apk: Option<bool>,
        sort: Option<&str>,
        limit: i64,
        cursor: Option<&str>,
        viewer_user_id: Option<&str>,
    ) -> Result<PublicProjectListPage> {
        let conn = self.conn()?;
        let sort = PublicProjectSort::from_query(sort);
        let cursor = decode_cursor(cursor, sort)?;
        let fetch_limit = limit.clamp(1, 50) + 1;
        let pattern = search
            .filter(|s| !s.trim().is_empty())
            .map(|s| format!("%{}%", s.to_ascii_lowercase()));
        let join_mode_filter = join_mode.and_then(|mode| match mode.trim() {
            "open" | "approval" | "readonly" => Some(mode.trim().to_string()),
            _ => None,
        });
        let has_apk_filter = has_apk.map(|value| if value { 1_i64 } else { 0_i64 });
        let cursor_filter = match sort {
            PublicProjectSort::Updated => {
                "(?6 IS NULL OR updated_at < ?6 OR (updated_at = ?6 AND id < ?7))"
            }
            PublicProjectSort::Created => {
                "(?6 IS NULL OR created_at < ?6 OR (created_at = ?6 AND id < ?7))"
            }
            PublicProjectSort::Members => {
                "(?6 IS NULL OR member_count < ?6 OR (member_count = ?6 AND (updated_at < ?7 OR (updated_at = ?7 AND id < ?8))))"
            }
        };
        let sql = format!(
            "
            WITH store_rows AS (
              SELECT
                p.id,
                p.name,
                p.description,
                p.template,
                COALESCE(u.nickname, u.phone, u.email, p.created_by) AS owner_account,
                (SELECT COUNT(*) FROM project_members pm2
                 WHERE pm2.project_id = p.id) AS member_count,
                p.is_public,
                p.join_mode,
                (SELECT t.status FROM tasks t
                 WHERE t.project_id = p.id
                 ORDER BY t.created_at DESC LIMIT 1) AS last_task_status,
                (SELECT t.apk_url FROM tasks t
                 WHERE t.project_id = p.id AND t.apk_url IS NOT NULL AND t.apk_url != ''
                 ORDER BY t.created_at DESC LIMIT 1) AS latest_apk_url,
                p.icon_data_url,
                p.created_at,
                p.updated_at,
                p.created_by AS owner_id,
                p.source_type,
                p.workspace_path,
                p.display_name,
                (SELECT pm.role FROM project_members pm
                 WHERE pm.project_id = p.id AND pm.user_id = ?5
                 LIMIT 1) AS viewer_role
              FROM projects p
              LEFT JOIN users u ON u.id = p.created_by
              WHERE p.is_public = 1
                AND p.join_mode != 'invite'
                AND p.status != 'deleted'
                AND p.source_type NOT IN ('agent_balloon', 'chat_memory')
                AND (
                  ?1 IS NULL
                  OR LOWER(p.name) LIKE ?1
                  OR LOWER(COALESCE(p.display_name,'')) LIKE ?1
                  OR LOWER(COALESCE(p.description,'')) LIKE ?1
                  OR LOWER(COALESCE(u.nickname, '')) LIKE ?1
                  OR LOWER(COALESCE(u.phone, u.email, p.created_by)) LIKE ?1
                )
                AND (?2 IS NULL OR p.join_mode = ?2)
                AND (
                  ?3 IS NULL
                  OR (?3 = 1 AND EXISTS (
                    SELECT 1 FROM tasks t_apk
                    WHERE t_apk.project_id = p.id
                      AND t_apk.apk_url IS NOT NULL
                      AND t_apk.apk_url != ''
                  ))
                  OR (?3 = 0 AND NOT EXISTS (
                    SELECT 1 FROM tasks t_apk
                    WHERE t_apk.project_id = p.id
                      AND t_apk.apk_url IS NOT NULL
                      AND t_apk.apk_url != ''
                  ))
                )
            )
            SELECT *
            FROM store_rows
            WHERE {cursor_filter}
            ORDER BY {}
            LIMIT ?4",
            sort.order_by()
        );

        let mut stmt = conn.prepare(&sql)?;
        let mapped = match sort {
            PublicProjectSort::Updated => stmt.query_map(
                params![
                    pattern,
                    join_mode_filter,
                    has_apk_filter,
                    fetch_limit,
                    viewer_user_id,
                    cursor.as_ref().and_then(|value| value.updated_at.clone()),
                    cursor.as_ref().map(|value| value.id.clone()),
                ],
                project_listing_row,
            )?,
            PublicProjectSort::Created => stmt.query_map(
                params![
                    pattern,
                    join_mode_filter,
                    has_apk_filter,
                    fetch_limit,
                    viewer_user_id,
                    cursor.as_ref().and_then(|value| value.created_at.clone()),
                    cursor.as_ref().map(|value| value.id.clone()),
                ],
                project_listing_row,
            )?,
            PublicProjectSort::Members => stmt.query_map(
                params![
                    pattern,
                    join_mode_filter,
                    has_apk_filter,
                    fetch_limit,
                    viewer_user_id,
                    cursor.as_ref().and_then(|value| value.member_count),
                    cursor.as_ref().and_then(|value| value.updated_at.clone()),
                    cursor.as_ref().map(|value| value.id.clone()),
                ],
                project_listing_row,
            )?,
        };

        let raw_rows = mapped.collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);

        let mut projects = Vec::with_capacity(raw_rows.len().min(limit as usize));
        for (mut project, source_type, workspace_path) in raw_rows {
            project_branding::apply_public_project_branding(
                &mut project,
                &source_type,
                workspace_path.as_deref(),
            );
            projects.push(project);
        }
        if let Some(viewer_user_id) = viewer_user_id {
            for project in &mut projects {
                project.viewer_role =
                    project_member_effective_role_locked(&conn, &project.id, viewer_user_id)?;
            }
        }

        let requested = limit.clamp(1, 50) as usize;
        let has_more = projects.len() > requested;
        if has_more {
            projects.truncate(requested);
        }
        let next_cursor = if has_more {
            projects
                .last()
                .map(|project| encode_cursor(sort, project))
                .transpose()?
        } else {
            None
        };

        Ok(PublicProjectListPage {
            projects,
            next_cursor,
            has_more,
        })
    }
}

fn project_listing_row(
    row: &Row<'_>,
) -> rusqlite::Result<(PublicProjectItem, String, Option<String>)> {
    let project = PublicProjectItem {
        id: row.get(0)?,
        name: row.get(1)?,
        display_name: row.get(16)?,
        description: row.get(2)?,
        template: row.get(3)?,
        owner_account: row.get(4)?,
        member_count: row.get(5)?,
        is_public: row.get::<_, i64>(6)? != 0,
        join_mode: row.get(7)?,
        viewer_role: row.get(17)?,
        last_task_status: row.get(8)?,
        latest_apk_url: row.get(9)?,
        icon_data_url: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
        owner_id: row.get(13).unwrap_or_default(),
    };
    Ok((project, row.get(14)?, row.get(15)?))
}

fn decode_cursor(
    cursor: Option<&str>,
    expected_sort: PublicProjectSort,
) -> Result<Option<StoreProjectCursor>> {
    let Some(raw) = cursor.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let bytes = URL_SAFE_NO_PAD
        .decode(raw.as_bytes())
        .map_err(|_| anyhow!("项目广场分页游标无效"))?;
    let cursor: StoreProjectCursor =
        serde_json::from_slice(&bytes).map_err(|_| anyhow!("项目广场分页游标无效"))?;
    if cursor.v != STORE_CURSOR_VERSION
        || cursor.sort != expected_sort.key()
        || cursor.id.trim().is_empty()
    {
        return Err(anyhow!("项目广场分页游标无效或已过期"));
    }
    match expected_sort {
        PublicProjectSort::Updated if cursor.updated_at.is_none() => {
            return Err(anyhow!("项目广场分页游标缺少更新时间"));
        }
        PublicProjectSort::Created if cursor.created_at.is_none() => {
            return Err(anyhow!("项目广场分页游标缺少创建时间"));
        }
        PublicProjectSort::Members
            if cursor.member_count.is_none() || cursor.updated_at.is_none() =>
        {
            return Err(anyhow!("项目广场分页游标缺少热度排序字段"));
        }
        _ => {}
    }
    Ok(Some(cursor))
}

fn encode_cursor(sort: PublicProjectSort, project: &PublicProjectItem) -> Result<String> {
    let cursor = StoreProjectCursor {
        v: STORE_CURSOR_VERSION,
        sort: sort.key().to_string(),
        id: project.id.clone(),
        updated_at: Some(project.updated_at.clone()),
        created_at: Some(project.created_at.clone()),
        member_count: (sort == PublicProjectSort::Members).then_some(project.member_count),
    };
    let bytes = serde_json::to_vec(&cursor)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
