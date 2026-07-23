use serde::{Deserialize, Serialize};

pub const CAP_PROJECT_DOCUMENT_FEDERATION_V1: &str = "project_document_federation_v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDocumentFederationPageRequest {
    pub workspace_path: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "default_page_limit")]
    pub limit: usize,
    #[serde(default)]
    pub cursor: Option<String>,
}

fn default_page_limit() -> usize {
    8
}
