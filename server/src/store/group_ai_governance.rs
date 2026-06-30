use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Row};
use serde_json::Value;

use crate::group_ai::types::{
    CreateMergeRequestInput, ProjectAiAssignmentArtifact, ProjectAiMergeRequest, ProjectAiReview,
    RecordAssignmentArtifactInput, RecordReviewInput, UpdateMergeRequestRequest,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn record_project_ai_assignment_artifact(
        &self,
        input: RecordAssignmentArtifactInput,
    ) -> Result<ProjectAiAssignmentArtifact> {
        let id = new_id("paif");
        let ts = now();
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO project_ai_assignment_artifacts
               (id, project_id, matter_id, assignment_id, uploader_user_id, artifact_kind,
                summary, worktree_path, branch_name, files_json, diff_stat_json,
                test_results_json, metadata_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
            params![
                id,
                clean_required(&input.project_id, "project_id")?,
                clean_required(&input.matter_id, "matter_id")?,
                clean_required(&input.assignment_id, "assignment_id")?,
                clean_optional(input.uploader_user_id.as_deref()),
                clean_required(&input.artifact_kind, "artifact_kind")?,
                clean_optional(input.summary.as_deref()),
                clean_optional(input.worktree_path.as_deref()),
                clean_optional(input.branch_name.as_deref()),
                serde_json::to_string(&clean_strings(&input.files))?,
                serde_json::to_string(&clean_strings(&input.diff_stat))?,
                serde_json::to_string(&clean_strings(&input.test_results))?,
                serde_json::to_string(&input.metadata)?,
                ts,
            ],
        )?;
        drop(conn);
        self.get_project_ai_assignment_artifact(&id)?
            .ok_or_else(|| anyhow!("Assignment artifact 保存失败"))
    }

    pub(crate) fn get_project_ai_assignment_artifact(
        &self,
        artifact_id: &str,
    ) -> Result<Option<ProjectAiAssignmentArtifact>> {
        let artifact_id = clean_required(artifact_id, "artifact_id")?;
        let conn = self.conn()?;
        conn.query_row(
            &format!("{} WHERE id = ?1", artifact_select_sql()),
            params![artifact_id],
            project_ai_assignment_artifact_from_row,
        )
        .optional()
        .map_err(Into::into)
    }

    pub(crate) fn list_project_ai_assignment_artifacts(
        &self,
        project_id: &str,
        matter_id: &str,
        assignment_id: &str,
    ) -> Result<Vec<ProjectAiAssignmentArtifact>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE project_id = ?1 AND matter_id = ?2 AND assignment_id = ?3 ORDER BY created_at DESC",
            artifact_select_sql()
        ))?;
        let rows = stmt.query_map(
            params![
                clean_required(project_id, "project_id")?,
                clean_required(matter_id, "matter_id")?,
                clean_required(assignment_id, "assignment_id")?,
            ],
            project_ai_assignment_artifact_from_row,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn record_project_ai_review(
        &self,
        input: RecordReviewInput,
    ) -> Result<ProjectAiReview> {
        let id = new_id("pair");
        let ts = now();
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO project_ai_reviews
               (id, matter_id, reviewer_bot_id, reviewer_user_id, target_assignment_id,
                severity, finding_json, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![
                id,
                clean_required(&input.matter_id, "matter_id")?,
                clean_optional(input.reviewer_bot_id.as_deref()),
                clean_optional(input.reviewer_user_id.as_deref()),
                clean_optional(input.target_assignment_id.as_deref()),
                clean_required(&input.severity, "severity")?,
                serde_json::to_string(&input.finding)?,
                clean_required(&input.status, "status")?,
                ts,
            ],
        )?;
        drop(conn);
        self.get_project_ai_review(&id)?
            .ok_or_else(|| anyhow!("Review 保存失败"))
    }

    pub(crate) fn get_project_ai_review(&self, review_id: &str) -> Result<Option<ProjectAiReview>> {
        let review_id = clean_required(review_id, "review_id")?;
        let conn = self.conn()?;
        conn.query_row(
            &format!("{} WHERE id = ?1", review_select_sql()),
            params![review_id],
            project_ai_review_from_row,
        )
        .optional()
        .map_err(Into::into)
    }

    pub(crate) fn list_project_ai_reviews(&self, matter_id: &str) -> Result<Vec<ProjectAiReview>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE matter_id = ?1 ORDER BY updated_at DESC",
            review_select_sql()
        ))?;
        let rows = stmt.query_map(
            params![clean_required(matter_id, "matter_id")?],
            project_ai_review_from_row,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn create_project_ai_merge_request(
        &self,
        input: CreateMergeRequestInput,
    ) -> Result<ProjectAiMergeRequest> {
        let id = new_id("paimr");
        let ts = now();
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO project_ai_merge_requests
               (id, project_id, matter_id, assignment_id, requested_by_user_id,
                worktree_path, branch_name, status, merge_strategy, review_status,
                risk_level, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'open', ?8, ?9, ?10, ?11, ?12, ?12)",
            params![
                id,
                clean_required(&input.project_id, "project_id")?,
                clean_required(&input.matter_id, "matter_id")?,
                clean_required(&input.assignment_id, "assignment_id")?,
                clean_optional(input.requested_by_user_id.as_deref()),
                clean_optional(input.worktree_path.as_deref()),
                clean_optional(input.branch_name.as_deref()),
                clean_required(&input.merge_strategy, "merge_strategy")?,
                clean_required(&input.review_status, "review_status")?,
                clean_required(&input.risk_level, "risk_level")?,
                clean_optional(input.notes.as_deref()),
                ts,
            ],
        )?;
        drop(conn);
        self.get_project_ai_merge_request(&id)?
            .ok_or_else(|| anyhow!("Merge request 保存失败"))
    }

    pub(crate) fn get_project_ai_merge_request(
        &self,
        merge_request_id: &str,
    ) -> Result<Option<ProjectAiMergeRequest>> {
        let merge_request_id = clean_required(merge_request_id, "merge_request_id")?;
        let conn = self.conn()?;
        conn.query_row(
            &format!("{} WHERE id = ?1", merge_request_select_sql()),
            params![merge_request_id],
            project_ai_merge_request_from_row,
        )
        .optional()
        .map_err(Into::into)
    }

    pub(crate) fn list_project_ai_merge_requests(
        &self,
        project_id: &str,
        matter_id: &str,
    ) -> Result<Vec<ProjectAiMergeRequest>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE project_id = ?1 AND matter_id = ?2 ORDER BY updated_at DESC",
            merge_request_select_sql()
        ))?;
        let rows = stmt.query_map(
            params![
                clean_required(project_id, "project_id")?,
                clean_required(matter_id, "matter_id")?,
            ],
            project_ai_merge_request_from_row,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn update_project_ai_merge_request(
        &self,
        project_id: &str,
        matter_id: &str,
        merge_request_id: &str,
        req: UpdateMergeRequestRequest,
    ) -> Result<ProjectAiMergeRequest> {
        let ts = now();
        let conn = self.conn()?;
        let updated = conn.execute(
            "UPDATE project_ai_merge_requests
                SET status = COALESCE(?4, status),
                    review_status = COALESCE(?5, review_status),
                    risk_level = COALESCE(?6, risk_level),
                    notes = COALESCE(?7, notes),
                    updated_at = ?8
              WHERE project_id = ?1 AND matter_id = ?2 AND id = ?3",
            params![
                clean_required(project_id, "project_id")?,
                clean_required(matter_id, "matter_id")?,
                clean_required(merge_request_id, "merge_request_id")?,
                clean_status(req.status.as_deref()),
                clean_optional(req.review_status.as_deref()),
                clean_optional(req.risk_level.as_deref()),
                clean_optional(req.notes.as_deref()),
                ts,
            ],
        )?;
        if updated == 0 {
            anyhow::bail!("Merge request 不存在");
        }
        drop(conn);
        self.get_project_ai_merge_request(merge_request_id)?
            .ok_or_else(|| anyhow!("Merge request 更新后读取失败"))
    }
}

fn artifact_select_sql() -> &'static str {
    "SELECT id, project_id, matter_id, assignment_id, uploader_user_id, artifact_kind,
            summary, worktree_path, branch_name, files_json, diff_stat_json,
            test_results_json, metadata_json, created_at, updated_at
       FROM project_ai_assignment_artifacts"
}

fn review_select_sql() -> &'static str {
    "SELECT id, matter_id, reviewer_bot_id, reviewer_user_id, target_assignment_id,
            severity, finding_json, status, created_at, updated_at
       FROM project_ai_reviews"
}

fn merge_request_select_sql() -> &'static str {
    "SELECT id, project_id, matter_id, assignment_id, requested_by_user_id,
            worktree_path, branch_name, status, merge_strategy, review_status,
            risk_level, notes, created_at, updated_at
       FROM project_ai_merge_requests"
}

fn project_ai_assignment_artifact_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<ProjectAiAssignmentArtifact> {
    Ok(ProjectAiAssignmentArtifact {
        id: row.get(0)?,
        project_id: row.get(1)?,
        matter_id: row.get(2)?,
        assignment_id: row.get(3)?,
        uploader_user_id: row.get(4)?,
        artifact_kind: row.get(5)?,
        summary: row.get(6)?,
        worktree_path: row.get(7)?,
        branch_name: row.get(8)?,
        files: parse_string_vec(row.get::<_, String>(9)?.as_str()),
        diff_stat: parse_string_vec(row.get::<_, String>(10)?.as_str()),
        test_results: parse_string_vec(row.get::<_, String>(11)?.as_str()),
        metadata: parse_json(row.get::<_, String>(12)?.as_str()),
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn project_ai_review_from_row(row: &Row<'_>) -> rusqlite::Result<ProjectAiReview> {
    Ok(ProjectAiReview {
        id: row.get(0)?,
        matter_id: row.get(1)?,
        reviewer_bot_id: row.get(2)?,
        reviewer_user_id: row.get(3)?,
        target_assignment_id: row.get(4)?,
        severity: row.get(5)?,
        finding: parse_json(row.get::<_, String>(6)?.as_str()),
        status: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn project_ai_merge_request_from_row(row: &Row<'_>) -> rusqlite::Result<ProjectAiMergeRequest> {
    Ok(ProjectAiMergeRequest {
        id: row.get(0)?,
        project_id: row.get(1)?,
        matter_id: row.get(2)?,
        assignment_id: row.get(3)?,
        requested_by_user_id: row.get(4)?,
        worktree_path: row.get(5)?,
        branch_name: row.get(6)?,
        status: row.get(7)?,
        merge_strategy: row.get(8)?,
        review_status: row.get(9)?,
        risk_level: row.get(10)?,
        notes: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn clean_required(value: &str, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("{field} 不能为空");
    }
    Ok(value.to_string())
}

fn clean_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn clean_strings(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(500).collect())
        .collect()
}

fn clean_status(value: Option<&str>) -> Option<&str> {
    match value.map(str::trim) {
        Some("open" | "approved" | "merged" | "rejected" | "canceled") => value.map(str::trim),
        _ => None,
    }
}

fn parse_string_vec(value: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(value).unwrap_or_default()
}

fn parse_json(value: &str) -> Value {
    serde_json::from_str::<Value>(value).unwrap_or(Value::Null)
}
