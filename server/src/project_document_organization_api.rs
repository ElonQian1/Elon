//! Web API adapter for the same document-governance domain used by local MCP.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    project_docs_snapshot::load_project_documents_catalog_snapshot,
    project_document_gateway::{read_optional_project_file, write_project_file},
    project_document_governance::{
        apply_suggestions, parse_manifest, parse_suggestions, to_pretty_json, SECTION_CONFIG_PATH,
        SUGGESTIONS_CONFIG_PATH,
    },
    types::AppState,
};

#[derive(Debug, Deserialize)]
pub(crate) struct ApplyOrganizationRequest {
    reviewed: bool,
    expected_catalog_revision: String,
    #[serde(default)]
    expected_manifest_revision: Option<String>,
    #[serde(default)]
    expected_suggestions_revision: Option<String>,
}

pub(crate) async fn apply_organization_suggestions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<ApplyOrganizationRequest>,
) -> Response {
    if !request.reviewed {
        return json_error(
            StatusCode::BAD_REQUEST,
            "应用 AI 文档整理建议前必须显式确认 reviewed=true",
        );
    }
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    let access = match project_access(&state, &user.id, &project_id) {
        Ok(access) => access,
        Err(error) => return json_error(StatusCode::FORBIDDEN, error.to_string()),
    };
    if !can_edit(&access.role) {
        return json_error(StatusCode::FORBIDDEN, "当前项目角色不能应用文档分区建议");
    }
    let snapshot = load_project_documents_catalog_snapshot(&state, &access).await;
    if request.expected_catalog_revision.trim().is_empty()
        || request.expected_catalog_revision != snapshot.revision
    {
        return json_error(StatusCode::CONFLICT, "文档目录已变化，请刷新后重新审核建议");
    }
    let manifest_file = match read_optional_project_file(&state, &access, SECTION_CONFIG_PATH).await
    {
        Ok(value) => value,
        Err((status, message)) => return json_error(status, message),
    };
    let suggestions_file =
        match read_optional_project_file(&state, &access, SUGGESTIONS_CONFIG_PATH).await {
            Ok(value) => value,
            Err((status, message)) => return json_error(status, message),
        };
    let manifest = match parse_manifest(manifest_file.as_ref().map(|file| file.content.as_str())) {
        Ok(value) => value,
        Err(error) => {
            return json_error(StatusCode::BAD_REQUEST, format!("分区配置无效：{error:#}"))
        }
    };
    let suggestions =
        match parse_suggestions(suggestions_file.as_ref().map(|file| file.content.as_str())) {
            Ok(Some(value)) => value,
            Ok(None) => return json_error(StatusCode::BAD_REQUEST, "项目尚未生成 AI 文档整理建议"),
            Err(error) => {
                return json_error(StatusCode::BAD_REQUEST, format!("AI 建议无效：{error:#}"))
            }
        };
    let current_manifest = manifest;
    let result = match apply_suggestions(current_manifest.clone(), suggestions, &snapshot.documents)
    {
        Ok(value) => value,
        Err(error) => {
            return json_error(StatusCode::BAD_REQUEST, format!("无法应用建议：{error:#}"))
        }
    };
    if result.already_applied {
        return Json(json!({
            "ok": true,
            "status": "applied",
            "already_applied": true,
            "manifest": result.manifest,
            "suggestions": result.suggestions,
            "manifest_revision": manifest_file.map(|file| file.revision),
            "suggestions_revision": suggestions_file.map(|file| file.revision),
            "markdown_changed": false,
        }))
        .into_response();
    }
    if let Err(message) = verify_revision(
        "AI 整理建议",
        suggestions_file.as_ref().map(|file| file.revision.as_str()),
        request.expected_suggestions_revision.as_deref(),
    ) {
        return json_error(StatusCode::CONFLICT, message);
    }
    let manifest_already_applied = result.manifest == current_manifest;
    if !manifest_already_applied {
        if let Err(message) = verify_revision(
            "项目文档分区",
            manifest_file.as_ref().map(|file| file.revision.as_str()),
            request.expected_manifest_revision.as_deref(),
        ) {
            return json_error(StatusCode::CONFLICT, message);
        }
    }
    let manifest_content = match to_pretty_json(&result.manifest) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    };
    let manifest_revision = if manifest_already_applied {
        manifest_file.as_ref().map(|file| file.revision.clone())
    } else {
        match write_project_file(
            &state,
            &access,
            SECTION_CONFIG_PATH,
            &manifest_content,
            request.expected_manifest_revision.as_deref(),
        )
        .await
        {
            Ok(value) => Some(value.revision),
            Err((status, message)) => return json_error(status, message),
        }
    };
    let suggestions_content = match to_pretty_json(&result.suggestions) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    };
    let suggestions_saved = match write_project_file(
        &state,
        &access,
        SUGGESTIONS_CONFIG_PATH,
        &suggestions_content,
        request.expected_suggestions_revision.as_deref(),
    )
    .await
    {
        Ok(value) => value,
        Err((status, message)) => {
            return json_error(
                status,
                format!("分区已安全写入，但建议状态更新失败；重新应用即可幂等恢复：{message}"),
            )
        }
    };
    Json(json!({
        "ok": true,
        "status": "applied",
        "already_applied": false,
        "manifest": result.manifest,
        "suggestions": result.suggestions,
        "manifest_revision": manifest_revision,
        "suggestions_revision": suggestions_saved.revision,
        "manifest_already_applied": manifest_already_applied,
        "applied_assignments": result.applied_assignments,
        "skipped_assignments": result.skipped_assignments,
        "markdown_changed": false,
    }))
    .into_response()
}

fn verify_revision(
    label: &str,
    current: Option<&str>,
    expected: Option<&str>,
) -> Result<(), String> {
    match (current, expected.filter(|value| !value.trim().is_empty())) {
        (Some(current), Some(expected)) if current == expected => Ok(()),
        (None, None) => Ok(()),
        (Some(_), None) => Err(format!("{label}已存在，请刷新后重新审核")),
        _ => Err(format!("{label}已被其他会话修改，请刷新后重新审核")),
    }
}

#[cfg(test)]
mod tests {
    use super::verify_revision;

    #[test]
    fn revision_gate_distinguishes_absent_current_and_stale_files() {
        assert!(verify_revision("配置", None, None).is_ok());
        assert!(verify_revision("配置", Some("a"), Some("a")).is_ok());
        assert!(verify_revision("配置", Some("a"), None).is_err());
        assert!(verify_revision("配置", Some("a"), Some("b")).is_err());
    }
}
