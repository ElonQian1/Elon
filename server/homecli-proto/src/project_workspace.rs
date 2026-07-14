use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliProjectContext {
    pub project_id: String,
    pub conversation_id: String,
    /// "project_write" | "full_access". Old nodes ignore this field.
    #[serde(default)]
    pub runtime_permission: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliWorkspaceStatus {
    #[serde(default)]
    pub base_workspace_path: Option<String>,
    pub active_workspace_path: String,
    pub isolated: bool,
    #[serde(default)]
    pub branch: Option<String>,
    pub prepare_status: String,
    #[serde(default)]
    pub merge_status: Option<String>,
    #[serde(default)]
    pub merge_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDocumentEntry {
    pub path: String,
    pub title: String,
    pub content: String,
    pub truncated: bool,
    pub byte_len: u64,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub metadata: ProjectDocumentMetadata,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectDocumentMetadata {
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub lifecycle: String,
    #[serde(default)]
    pub authority: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub default_retrieval: bool,
    #[serde(default)]
    pub ambiguous: bool,
    #[serde(default)]
    pub confidence: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub token_estimate: u64,
    #[serde(default)]
    pub content_hash: String,
    #[serde(default)]
    pub headings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDocumentsSnapshot {
    pub workspace_path: String,
    #[serde(default)]
    pub revision: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub generated_at_ms: u64,
    #[serde(default)]
    pub documents: Vec<ProjectDocumentEntry>,
    #[serde(default)]
    pub warnings: Vec<String>,
}
