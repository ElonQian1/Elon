use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;

use super::{new_id, project_summary_from_row, ProjectSummary};

pub(super) const IDENTITY_WORKSPACE_PATH: &str = "workspace_path";
pub(super) const IDENTITY_GIT_REMOTE: &str = "git_remote";
pub(super) const IDENTITY_GIT_REMOTE_BRANCH: &str = "git_remote_branch";

const WORKSPACE_SCOPE_UNKNOWN_NODE: &str = "node:unknown";
const GIT_SCOPE: &str = "git";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectIdentityCandidate {
    pub scope_key: String,
    pub node_id: Option<String>,
    pub identity_type: &'static str,
    pub identity_value: String,
    pub confidence: i64,
}

pub(super) fn identity_candidates(
    node_id: Option<&str>,
    workspace_path: &str,
    repo_url: Option<&str>,
    branch: Option<&str>,
) -> Vec<ProjectIdentityCandidate> {
    let mut candidates = Vec::new();
    let normalized_workspace = normalize_workspace_path(workspace_path);
    if !normalized_workspace.is_empty() {
        candidates.push(ProjectIdentityCandidate {
            scope_key: workspace_scope_key(node_id),
            node_id: node_id.map(|value| value.trim().to_string()),
            identity_type: IDENTITY_WORKSPACE_PATH,
            identity_value: normalized_workspace,
            confidence: 100,
        });
    }

    if let Some(normalized_remote) = repo_url.and_then(normalize_git_remote_url) {
        candidates.push(ProjectIdentityCandidate {
            scope_key: GIT_SCOPE.to_string(),
            node_id: None,
            identity_type: IDENTITY_GIT_REMOTE,
            identity_value: normalized_remote.clone(),
            confidence: 95,
        });
        if let Some(branch) = normalize_git_branch(branch) {
            candidates.push(ProjectIdentityCandidate {
                scope_key: GIT_SCOPE.to_string(),
                node_id: None,
                identity_type: IDENTITY_GIT_REMOTE_BRANCH,
                identity_value: format!("{}@{}", normalized_remote, branch),
                confidence: 98,
            });
        }
    }

    dedupe_candidates(candidates)
}

pub(super) fn replace_project_identities(
    conn: &Connection,
    project_id: &str,
    owner_user_id: &str,
    node_id: Option<&str>,
    workspace_path: &str,
    repo_url: Option<&str>,
    branch: Option<&str>,
    now: &str,
) -> Result<()> {
    let candidates = identity_candidates(node_id, workspace_path, repo_url, branch);
    conn.execute(
        "DELETE FROM project_identities
         WHERE project_id = ?1
           AND owner_user_id = ?2
           AND identity_type IN ('workspace_path', 'git_remote', 'git_remote_branch')",
        params![project_id, owner_user_id],
    )?;

    for candidate in candidates {
        let changed = conn.execute(
            "INSERT INTO project_identities (
                id, project_id, owner_user_id, scope_key, node_id, identity_type,
                identity_value, confidence, source, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'register_external_project', ?9, ?9)
             ON CONFLICT(owner_user_id, scope_key, identity_type, identity_value)
             DO UPDATE SET
                project_id = excluded.project_id,
                node_id = excluded.node_id,
                confidence = excluded.confidence,
                source = excluded.source,
                updated_at = excluded.updated_at
             WHERE project_identities.project_id = excluded.project_id",
            params![
                new_id("pident"),
                project_id,
                owner_user_id,
                candidate.scope_key,
                candidate.node_id.as_deref(),
                candidate.identity_type,
                candidate.identity_value,
                candidate.confidence,
                now
            ],
        )?;
        if changed == 0 {
            if let Some(project) =
                find_owner_project_by_identity(conn, owner_user_id, &[candidate.clone()])?
            {
                return Err(identity_conflict_error(&project));
            }
            return Err(anyhow!("项目身份写入冲突，请重试"));
        }
    }

    Ok(())
}

pub(super) fn find_owner_project_by_identity(
    conn: &Connection,
    owner_user_id: &str,
    candidates: &[ProjectIdentityCandidate],
) -> Result<Option<ProjectSummary>> {
    for candidate in candidates {
        let project = conn
            .query_row(
                "SELECT p.id, p.name, p.description, p.workspace_key, p.template,
                        p.source_type, p.repo_url, p.branch, p.workspace_path, p.node_id,
                        p.storage_node_id, p.storage_repo_path, p.storage_repo_url,
                        p.storage_worktree_path, COALESCE(p.storage_status, 'none'), p.status,
                        COALESCE(pm.role, 'owner') AS role,
                        (SELECT COUNT(*) FROM project_members pm2 WHERE pm2.project_id = p.id) AS member_count,
                        p.is_public,
                        p.join_mode,
                        (
                            SELECT t.status FROM tasks t
                            WHERE t.project_id = p.id
                            ORDER BY t.created_at DESC
                            LIMIT 1
                        ) AS last_task_status,
                        (
                            SELECT t.apk_url FROM tasks t
                            WHERE t.project_id = p.id AND t.apk_url IS NOT NULL
                            ORDER BY t.created_at DESC
                            LIMIT 1
                        ) AS last_apk_url,
                        p.icon_data_url,
                        p.updated_at,
                        p.display_name
                 FROM project_identities pi
                 JOIN projects p ON p.id = pi.project_id
                 LEFT JOIN project_members pm ON pm.project_id = p.id AND pm.user_id = ?1
                 WHERE pi.owner_user_id = ?1
                   AND pi.scope_key = ?2
                   AND pi.identity_type = ?3
                   AND pi.identity_value = ?4
                   AND p.created_by = ?1
                   AND p.status != 'deleted'
                 ORDER BY pi.confidence DESC, p.updated_at DESC
                 LIMIT 1",
                params![
                    owner_user_id,
                    candidate.scope_key,
                    candidate.identity_type,
                    candidate.identity_value
                ],
                project_summary_from_row,
            )
            .optional()?;
        if project.is_some() {
            return Ok(project);
        }
    }
    Ok(None)
}

pub(super) fn find_owner_project_by_git_remote(
    conn: &Connection,
    owner_user_id: &str,
    repo_url: Option<&str>,
) -> Result<Option<ProjectSummary>> {
    let Some(expected) = repo_url.and_then(normalize_git_remote_url) else {
        return Ok(None);
    };
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.description, p.workspace_key, p.template,
                p.source_type, p.repo_url, p.branch, p.workspace_path, p.node_id,
                p.storage_node_id, p.storage_repo_path, p.storage_repo_url,
                p.storage_worktree_path, COALESCE(p.storage_status, 'none'), p.status,
                COALESCE(pm.role, 'owner') AS role,
                (SELECT COUNT(*) FROM project_members pm2 WHERE pm2.project_id = p.id) AS member_count,
                p.is_public,
                p.join_mode,
                (
                    SELECT t.status FROM tasks t
                    WHERE t.project_id = p.id
                    ORDER BY t.created_at DESC
                    LIMIT 1
                ) AS last_task_status,
                (
                    SELECT t.apk_url FROM tasks t
                    WHERE t.project_id = p.id AND t.apk_url IS NOT NULL
                    ORDER BY t.created_at DESC
                    LIMIT 1
                ) AS last_apk_url,
                p.icon_data_url,
                p.updated_at,
                p.display_name
         FROM projects p
         LEFT JOIN project_members pm ON pm.project_id = p.id AND pm.user_id = ?1
         WHERE p.created_by = ?1
           AND p.status != 'deleted'
           AND p.repo_url IS NOT NULL
           AND TRIM(p.repo_url) != ''
         ORDER BY p.updated_at DESC",
    )?;
    let mut rows = stmt.query_map(params![owner_user_id], project_summary_from_row)?;
    while let Some(project) = rows.next() {
        let project = project?;
        if project
            .repo_url
            .as_deref()
            .and_then(normalize_git_remote_url)
            .as_deref()
            == Some(expected.as_str())
        {
            return Ok(Some(project));
        }
    }
    Ok(None)
}

pub(super) fn identity_conflict_error(project: &ProjectSummary) -> anyhow::Error {
    let display = project
        .display_name
        .as_deref()
        .unwrap_or(project.name.as_str());
    anyhow!("该代码项目已绑定到项目「{}」，请直接打开该项目", display)
}

pub(super) fn normalize_workspace_path(path: &str) -> String {
    path.trim()
        .trim_matches(|ch| ch == '"' || ch == '\'')
        .trim_end_matches(|ch| ch == '/' || ch == '\\')
        .replace('\\', "/")
        .to_ascii_lowercase()
}

pub(super) fn normalize_git_remote_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_matches(|ch| ch == '"' || ch == '\'').trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut value = trimmed.replace('\\', "/");
    if let Some(index) = value.find(|ch| ch == '?' || ch == '#') {
        value.truncate(index);
    }
    value = value.trim_end_matches('/').to_string();

    let (host, path) = if let Some((scheme, rest)) = value.split_once("://") {
        let slash = rest.find('/').unwrap_or(rest.len());
        let authority = &rest[..slash];
        let path = rest.get(slash + 1..).unwrap_or_default();
        let host = authority.rsplit('@').next().unwrap_or(authority);
        (normalize_host(host, scheme), path.to_string())
    } else if looks_like_scp_url(&value) {
        let (_, rest) = value.split_once('@')?;
        let (host, path) = rest.split_once(':')?;
        (normalize_host(host, "ssh"), path.to_string())
    } else if value.contains('/') {
        let without_leading_scheme = value.trim_start_matches('/');
        let slash = without_leading_scheme.find('/')?;
        let host = &without_leading_scheme[..slash];
        let path = &without_leading_scheme[slash + 1..];
        (normalize_host(host, ""), path.to_string())
    } else {
        return None;
    };

    let host = host.trim_matches('/').trim().to_ascii_lowercase();
    let path = normalize_repo_path(&path);
    if host.is_empty() || path.is_empty() {
        return None;
    }
    Some(format!("{}/{}", host, path))
}

fn normalize_git_branch(branch: Option<&str>) -> Option<String> {
    branch
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.trim_start_matches("refs/heads/").to_ascii_lowercase())
}

fn workspace_scope_key(node_id: Option<&str>) -> String {
    node_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("node:{}", value.to_ascii_lowercase()))
        .unwrap_or_else(|| WORKSPACE_SCOPE_UNKNOWN_NODE.to_string())
}

fn normalize_host(host: &str, scheme: &str) -> String {
    let mut host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    match scheme.to_ascii_lowercase().as_str() {
        "https" => {
            if let Some(stripped) = host.strip_suffix(":443") {
                host = stripped.to_string();
            }
        }
        "http" => {
            if let Some(stripped) = host.strip_suffix(":80") {
                host = stripped.to_string();
            }
        }
        "ssh" => {
            if let Some(stripped) = host.strip_suffix(":22") {
                host = stripped.to_string();
            }
        }
        _ => {}
    }
    host
}

fn normalize_repo_path(path: &str) -> String {
    let mut path = path
        .trim()
        .trim_start_matches('/')
        .trim_end_matches('/')
        .to_ascii_lowercase();
    while path.ends_with(".git") {
        let new_len = path.len() - 4;
        path.truncate(new_len);
    }
    path
}

fn looks_like_scp_url(value: &str) -> bool {
    value.contains('@') && value.contains(':') && !value.contains("://")
}

fn dedupe_candidates(candidates: Vec<ProjectIdentityCandidate>) -> Vec<ProjectIdentityCandidate> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for candidate in candidates {
        let key = (
            candidate.scope_key.clone(),
            candidate.identity_type,
            candidate.identity_value.clone(),
        );
        if seen.insert(key) {
            deduped.push(candidate);
        }
    }
    deduped.sort_by(|left, right| right.confidence.cmp(&left.confidence));
    deduped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_git_remote_protocol_variants() {
        assert_eq!(
            normalize_git_remote_url("git@github.com:Owner/Repo.git").as_deref(),
            Some("github.com/owner/repo")
        );
        assert_eq!(
            normalize_git_remote_url("https://github.com/owner/repo").as_deref(),
            Some("github.com/owner/repo")
        );
        assert_eq!(
            normalize_git_remote_url("ssh://git@github.com:22/OWNER/Repo.git").as_deref(),
            Some("github.com/owner/repo")
        );
    }
}
