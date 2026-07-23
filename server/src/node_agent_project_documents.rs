use homecli_proto::AgentToServer;
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::Message;

use crate::{project_docs_scan, project_document_files, ws_text};

pub(crate) fn spawn_catalog_response(
    req_id: String,
    workspace_path: String,
    seed_defaults: bool,
    catalog_only: bool,
    output: UnboundedSender<Message>,
) {
    tokio::spawn(async move {
        let response = match project_docs_scan::collect_project_documents_with_options(
            std::path::Path::new(&workspace_path),
            project_docs_scan::ProjectDocumentScanOptions {
                seed_missing_defaults: seed_defaults,
                catalog_only,
                include_analysis: true,
            },
        ) {
            Ok(mut snapshot) => {
                if catalog_only {
                    crate::project_document_federation_service::strip_catalog_nodes(
                        &mut snapshot.analysis,
                    );
                }
                AgentToServer::ProjectDocumentsRead { req_id, snapshot }
            }
            Err(error) => AgentToServer::ProjectDocumentsReadError {
                req_id,
                message: error.to_string(),
            },
        };
        let _ = output.send(ws_text(&response));
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_federation_response(
    req_id: String,
    workspace_path: String,
    parent_id: Option<String>,
    query: Option<String>,
    offset: usize,
    limit: usize,
    cursor: Option<String>,
    output: UnboundedSender<Message>,
) {
    tokio::spawn(async move {
        let arguments = serde_json::json!({
            "projection": "page", "offset": offset, "limit": limit, "cursor": cursor,
        });
        let response =
            crate::project_document_response::ProjectionRequest::from_arguments(&arguments)
                .and_then(|request| {
                    crate::project_document_federation_service::get_federation_index(
                        std::path::Path::new(&workspace_path),
                        parent_id.as_deref(),
                        query.as_deref(),
                        &request,
                    )
                })
                .map(|page| AgentToServer::ProjectDocumentFederationRead {
                    req_id: req_id.clone(),
                    page,
                })
                .unwrap_or_else(|error| AgentToServer::ProjectDocumentFederationReadError {
                    req_id,
                    message: error.to_string(),
                });
        let _ = output.send(ws_text(&response));
    });
}

pub(crate) fn spawn_file_read_response(
    req_id: String,
    workspace_path: String,
    document_path: String,
    output: UnboundedSender<Message>,
) {
    tokio::spawn(async move {
        let response = match project_document_files::read_project_document_file(
            std::path::Path::new(&workspace_path),
            &document_path,
        ) {
            Ok(document) => AgentToServer::ProjectDocumentFileRead {
                req_id,
                path: document.path,
                content: document.content,
                revision: document.revision,
                byte_len: document.byte_len,
            },
            Err(error) => AgentToServer::ProjectDocumentFileReadError {
                req_id,
                message: error.to_string(),
            },
        };
        let _ = output.send(ws_text(&response));
    });
}

pub(crate) fn spawn_file_write_response(
    req_id: String,
    workspace_path: String,
    document_path: String,
    content: String,
    expected_revision: Option<String>,
    output: UnboundedSender<Message>,
) {
    tokio::spawn(async move {
        let response = match project_document_files::write_project_document_file(
            std::path::Path::new(&workspace_path),
            &document_path,
            &content,
            expected_revision.as_deref(),
        ) {
            Ok(document) => AgentToServer::ProjectDocumentFileWritten {
                req_id,
                path: document.path,
                revision: document.revision,
                byte_len: document.byte_len,
            },
            Err(error) => AgentToServer::ProjectDocumentFileWriteError {
                req_id,
                message: error.message,
                conflict: error.conflict,
            },
        };
        let _ = output.send(ws_text(&response));
    });
}
