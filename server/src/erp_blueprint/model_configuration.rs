use serde::{Deserialize, Serialize};

use super::model::{ErpCapabilityDefinition, ErpExtensionRef, ErpModuleDefinition};

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct EvolveBlueprintRequest {
    pub expected_revision: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub proposal_threshold: Option<i64>,
    #[serde(default)]
    pub add_modules: Vec<ErpModuleDefinition>,
    #[serde(default)]
    pub add_capabilities: Vec<ErpCapabilityDefinition>,
    #[serde(default)]
    pub add_themes: Vec<String>,
    #[serde(default)]
    pub add_extension_points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ErpInstanceConfiguration {
    pub theme_key: String,
    pub enabled_modules: Vec<String>,
    #[serde(default)]
    pub plugins: Vec<ErpExtensionRef>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct UpdateErpInstanceRequest {
    pub expected_revision: i64,
    pub merchant_confirmed: bool,
    pub theme_key: String,
    pub enabled_modules: Vec<String>,
    #[serde(default)]
    pub plugins: Vec<ErpExtensionRef>,
    #[serde(default)]
    pub private_extensions: Vec<ErpExtensionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ErpUpgradeAdoptionEvidence {
    pub execution_attested: bool,
    pub verification_summary: String,
    #[serde(default)]
    pub deployed_commit: Option<String>,
}
