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
            Ok(snapshot) => AgentToServer::ProjectDocumentsRead { req_id, snapshot },
            Err(error) => AgentToServer::ProjectDocumentsReadError {
                req_id,
                message: error.to_string(),
            },
        };
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
