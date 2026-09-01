use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;

use super::parse_catalog;

const PUBLIC_PREVIEW_SCHEMA: &str = "yilong.official_project_preview.v1";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OfficialProjectPublicPreview {
    pub(crate) schema: String,
    pub(crate) project_id: String,
    pub(crate) title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tagline: Option<String>,
    pub(crate) summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    pub(crate) highlights: Vec<String>,
    pub(crate) target_users: Vec<String>,
    pub(crate) recent_updates: Vec<String>,
    pub(crate) privacy_notes: Vec<String>,
    pub(crate) system_requirements: Vec<String>,
    pub(crate) downloads: Vec<OfficialProjectPublicDownload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) paper_launch: Option<OfficialProjectPublicPaperLaunch>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OfficialProjectPublicDownload {
    pub(crate) platform: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) label: Option<String>,
    pub(crate) status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OfficialProjectPublicPaperLaunch {
    pub(crate) schema: String,
    pub(crate) mode: String,
    pub(crate) simulated: bool,
    pub(crate) funds_moved: bool,
    pub(crate) target_is_guaranteed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
}

pub(crate) fn has_public_preview(project_id: &str) -> bool {
    public_preview(project_id).ok().flatten().is_some()
}

pub(crate) fn public_preview(project_id: &str) -> Result<Option<OfficialProjectPublicPreview>> {
    let catalog = parse_catalog()?;
    let Some(project) = catalog
        .projects
        .iter()
        .find(|project| project.id == project_id)
    else {
        return Ok(None);
    };
    let landing = crate::project_landing::normalize_landing_snapshot(&project.landing)
        .with_context(|| format!("官方项目 {} 的公开首页为空", project.id))?;
    let object = landing
        .as_object()
        .with_context(|| format!("官方项目 {} 的公开首页不是对象", project.id))?;

    // Download url, manifest_url and resource URLs are intentionally excluded.
    let downloads = object
        .get("downloads")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(public_download)
        .collect();
    let paper_launch = object
        .get("paper_launch")
        .and_then(Value::as_object)
        .and_then(|paper| {
            (paper.get("schema")?.as_str()? == "yilong.quant.paper_launch.v1").then(|| {
                OfficialProjectPublicPaperLaunch {
                    schema: paper
                        .get("schema")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    mode: paper
                        .get("mode")
                        .and_then(Value::as_str)
                        .unwrap_or("paper")
                        .to_string(),
                    simulated: paper
                        .get("simulated")
                        .and_then(Value::as_bool)
                        .unwrap_or(true),
                    funds_moved: paper
                        .get("funds_moved")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    target_is_guaranteed: paper
                        .get("target_is_guaranteed")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    label: text(paper.get("label")),
                    description: text(paper.get("description")),
                }
            })
        });

    Ok(Some(OfficialProjectPublicPreview {
        schema: PUBLIC_PREVIEW_SCHEMA.to_string(),
        project_id: project.id.clone(),
        title: text(object.get("title")).unwrap_or_else(|| project.display_name.clone()),
        tagline: text(object.get("tagline")),
        summary: text(object.get("summary")).unwrap_or_else(|| project.description.clone()),
        description: text(object.get("description")),
        highlights: text_list(object.get("highlights")),
        target_users: text_list(object.get("target_users")),
        recent_updates: text_list(object.get("recent_updates")),
        privacy_notes: text_list(object.get("privacy_notes")),
        system_requirements: text_list(object.get("system_requirements")),
        downloads,
        paper_launch,
    }))
}

fn public_download(value: &Value) -> Option<OfficialProjectPublicDownload> {
    let object = value.as_object()?;
    Some(OfficialProjectPublicDownload {
        platform: object.get("platform")?.as_str()?.to_string(),
        kind: text(object.get("kind")),
        label: text(object.get("label")),
        status: object
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("planned")
            .to_string(),
        note: text(object.get("note")),
    })
}

fn text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn text_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| text(Some(item)))
        .collect()
}
