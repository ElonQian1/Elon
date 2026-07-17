//! Continuous knowledge maintenance and the server-side health source of truth.

use anyhow::Result;
use homecli_proto::ProjectDocumentEntry;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::Duration,
};

use crate::{
    project_docs_scan::{collect_project_documents_with_options, ProjectDocumentScanOptions},
    project_document_architecture::analyze_knowledge_architecture,
    project_document_federation::analyze_federation,
    project_document_files::content_revision,
    project_document_governance::{parse_manifest, SECTION_CONFIG_PATH},
    project_document_index::ProjectDocumentIndex,
    project_document_knowledge_graph::build_knowledge_maps,
    project_document_quality::{analyze_document_quality, compact_report},
};

static REGISTERED_WORKSPACES: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();

pub(crate) fn enrich_catalog(
    workspace: &Path,
    documents: &[ProjectDocumentEntry],
    index: &ProjectDocumentIndex,
) -> Result<Value> {
    register_workspace(workspace);
    let (manifest, manifest_revision) = load_manifest(workspace)?;
    let quality = analyze_document_quality(workspace, documents, &manifest, index)?;
    index.replace_issues(
        &quality
            .issues
            .iter()
            .map(serde_json::to_value)
            .collect::<serde_json::Result<Vec<_>>>()?,
    )?;
    let architecture = analyze_knowledge_architecture(documents, &manifest);
    let knowledge_maps = build_knowledge_maps(workspace, documents, &manifest);
    let knowledge_map_revision = content_revision(&serde_json::to_string(&knowledge_maps)?);
    let canonical_workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf())
        .to_string_lossy()
        .to_string();
    let federation = analyze_federation(workspace, documents, &manifest)?;
    let maintenance = index.complete_analysis()?;
    let overall_score = weighted_score(
        architecture.score,
        quality.summary.score,
        federation.aggregated_score,
    );
    Ok(json!({
        "version": 1,
        "source": "server",
        "overall": {
            "score": overall_score,
            "status": health_status(overall_score),
        },
        "architecture": architecture,
        "knowledge_maps": knowledge_maps,
        "identity": {
            "workspace": workspace.to_string_lossy(),
            "canonical_workspace": canonical_workspace,
            "manifest_revision": manifest_revision,
            "knowledge_map_revision": knowledge_map_revision,
        },
        "quality": compact_report(&quality, 40),
        "maintenance": maintenance,
        "federation": federation,
    }))
}

pub(crate) fn list_issues(
    workspace: &Path,
    issue_types: &[String],
    offset: usize,
    limit: usize,
) -> Result<Vec<Value>> {
    ProjectDocumentIndex::open(workspace)?.list_issues(
        issue_types,
        offset.min(100_000),
        limit.clamp(1, 200),
    )
}

pub(crate) fn spawn_maintenance_worker() {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let workspaces = registered_workspaces();
            for workspace in workspaces {
                match tokio::task::spawn_blocking(move || refresh_workspace(&workspace)).await {
                    Ok(Err(error)) => tracing::warn!("文档后台维护失败：{error:#}"),
                    Err(error) => tracing::warn!("文档后台维护任务异常：{error}"),
                    Ok(Ok(())) => {}
                }
            }
        }
    });
}

fn load_manifest(
    workspace: &Path,
) -> Result<(
    crate::project_document_governance::DocumentSectionManifest,
    Option<String>,
)> {
    let path = workspace.join(SECTION_CONFIG_PATH);
    let content = fs::read_to_string(path).ok();
    Ok((
        parse_manifest(content.as_deref())?,
        content.as_deref().map(content_revision),
    ))
}

fn register_workspace(workspace: &Path) {
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    if let Ok(mut registered) = REGISTERED_WORKSPACES
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
    {
        registered.insert(workspace);
    }
}

fn registered_workspaces() -> Vec<PathBuf> {
    REGISTERED_WORKSPACES
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .map(|registered| registered.iter().cloned().collect())
        .unwrap_or_default()
}

fn refresh_workspace(workspace: &Path) -> Result<()> {
    collect_project_documents_with_options(
        workspace,
        ProjectDocumentScanOptions {
            seed_missing_defaults: false,
            catalog_only: true,
            include_analysis: true,
        },
    )?;
    let index = ProjectDocumentIndex::open(workspace)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent("Elon-Document-Health/1.0")
        .build()?;
    for url in index.external_links_due(20)? {
        if !url.starts_with("https://") && !url.starts_with("http://") {
            continue;
        }
        match client
            .head(&url)
            .send()
            .or_else(|_| client.get(&url).send())
        {
            Ok(response) => {
                index.store_external_link_result(&url, Some(response.status().as_u16()), None)?
            }
            Err(error) => index.store_external_link_result(&url, None, Some(&error.to_string()))?,
        }
    }
    Ok(())
}

fn weighted_score(architecture: u8, quality: u8, federation: u8) -> u8 {
    ((u16::from(architecture) * 35 + u16::from(quality) * 50 + u16::from(federation) * 15) / 100)
        as u8
}

fn health_status(score: u8) -> &'static str {
    if score >= 85 {
        "healthy"
    } else if score >= 60 {
        "review"
    } else {
        "needs_attention"
    }
}
