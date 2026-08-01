use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) use super::model_configuration::*;

pub(crate) const BLUEPRINT_SCHEMA: &str = "yilong.erp.blueprint.v1";
pub(crate) const INSTANCE_SCHEMA: &str = "yilong.erp.instance.v1";
pub(crate) const RELEASE_SCHEMA: &str = "yilong.erp.release.v1";
pub(crate) const SIGNAL_SCHEMA: &str = "yilong.erp.feature_signal.v1";
pub(crate) const MATERIALIZATION_CONTRACT_SCHEMA: &str = "yilong.erp.materialization_contract.v1";
pub(crate) const MATERIALIZATION_EVIDENCE_SCHEMA: &str = "yilong.erp.materialization_evidence.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ErpModuleDefinition {
    pub module_key: String,
    pub version: String,
    pub kind: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ErpCapabilityDefinition {
    pub capability_key: String,
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    pub category: String,
    pub module_key: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub composable_with: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ErpBlueprintDefinition {
    pub schema: String,
    pub blueprint_key: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub source_project_id: String,
    pub modules: Vec<ErpModuleDefinition>,
    #[serde(default)]
    pub capabilities: Vec<ErpCapabilityDefinition>,
    pub themes: Vec<String>,
    #[serde(default)]
    pub extension_points: Vec<String>,
    #[serde(default = "default_proposal_threshold")]
    pub proposal_threshold: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateBlueprintRequest {
    pub blueprint_key: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub modules: Vec<ErpModuleDefinition>,
    #[serde(default)]
    pub capabilities: Vec<ErpCapabilityDefinition>,
    pub themes: Vec<String>,
    #[serde(default)]
    pub extension_points: Vec<String>,
    #[serde(default = "default_proposal_threshold")]
    pub proposal_threshold: i64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ErpBlueprint {
    pub id: String,
    pub definition: ErpBlueprintDefinition,
    pub definition_revision: i64,
    pub status: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct VersionedErpModule {
    pub module_key: String,
    pub version: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ErpMigrationStep {
    pub migration_key: String,
    pub reversible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ErpReleaseCompatibility {
    pub minimum_instance_version: String,
    #[serde(default)]
    pub required_plugins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ErpRollbackPlan {
    pub supported: bool,
    pub instructions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ErpReleaseManifest {
    pub schema: String,
    pub blueprint_key: String,
    pub version: String,
    #[serde(default)]
    pub previous_version: Option<String>,
    pub source_git_commit: String,
    pub modules: Vec<VersionedErpModule>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub extension_points: Vec<String>,
    #[serde(default)]
    pub migrations: Vec<ErpMigrationStep>,
    pub compatibility: ErpReleaseCompatibility,
    pub rollback: ErpRollbackPlan,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateBlueprintVersionRequest {
    pub manifest: ErpReleaseManifest,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ErpBlueprintVersion {
    pub id: String,
    pub blueprint_id: String,
    pub manifest: ErpReleaseManifest,
    pub manifest_sha256: String,
    pub status: String,
    pub created_by: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ErpExtensionRef {
    pub extension_key: String,
    pub version: String,
    pub extension_point: String,
    #[serde(default)]
    pub requires_modules: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateErpInstanceRequest {
    pub instance_key: String,
    pub project_name: String,
    pub version: String,
    pub industry: String,
    pub theme_key: String,
    #[serde(default)]
    pub enabled_modules: Vec<String>,
    #[serde(default)]
    pub plugins: Vec<ErpExtensionRef>,
    #[serde(default)]
    pub private_extensions: Vec<ErpExtensionRef>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ErpInstance {
    pub id: String,
    pub instance_key: String,
    pub project_id: String,
    pub blueprint_id: String,
    pub pinned_version_id: String,
    pub pinned_version: String,
    pub industry: String,
    pub theme_key: String,
    pub enabled_modules: Vec<String>,
    pub plugins: Vec<ErpExtensionRef>,
    pub private_extensions: Vec<ErpExtensionRef>,
    pub configuration_revision: i64,
    pub bootstrap_matter_id: Option<String>,
    pub status: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ErpMaterializationSource {
    pub project_id: String,
    pub git_commit: String,
    pub blueprint_key: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ErpMaterializationConfiguration {
    pub revision: i64,
    pub industry: String,
    pub theme_key: String,
    pub enabled_modules: Vec<String>,
    pub plugins: Vec<ErpExtensionRef>,
    pub private_extensions: Vec<ErpExtensionRef>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ErpMaterializationArtifactContract {
    pub artifact_kind: &'static str,
    pub instance_manifest_path: &'static str,
    pub evidence_schema: &'static str,
    pub required_metadata_fields: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ErpMaterializationContract {
    pub schema: &'static str,
    pub instance_id: String,
    pub instance_key: String,
    pub target_project_id: String,
    pub source: ErpMaterializationSource,
    pub configuration: ErpMaterializationConfiguration,
    pub required_artifact: ErpMaterializationArtifactContract,
    pub verification_requirements: Vec<&'static str>,
    pub boundaries: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ErpMaterializationAssignmentSummary {
    pub total: usize,
    pub planned: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub failed_assignment_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ErpMaterializationMatterSummary {
    pub id: String,
    pub status: String,
    pub decision: Option<String>,
    pub plan_contract_matches: bool,
    pub assignments: ErpMaterializationAssignmentSummary,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ErpMaterializationEvidenceSummary {
    pub artifact_id: String,
    pub assignment_id: String,
    pub valid: bool,
    pub issues: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ErpMaterializationStatus {
    pub schema: &'static str,
    pub state: String,
    pub recoverable: bool,
    pub contract: ErpMaterializationContract,
    pub matter: Option<ErpMaterializationMatterSummary>,
    pub evidence: Vec<ErpMaterializationEvidenceSummary>,
    pub blockers: Vec<String>,
    pub next_action: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ResolveRequirementRequest {
    #[serde(default)]
    pub instance_id: Option<String>,
    pub requirement: String,
    #[serde(default)]
    pub expected_scope: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RequirementResolution {
    pub schema: &'static str,
    pub classification: String,
    pub requirement: String,
    pub matched_capabilities: Vec<ErpCapabilityDefinition>,
    pub need_key: Option<String>,
    pub recommendation: String,
    pub may_submit_signal: bool,
    pub catalog_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct FeatureSignalEvidence {
    #[serde(default)]
    pub occurrence_count: Option<i64>,
    #[serde(default)]
    pub affected_workflow: Option<String>,
    #[serde(default)]
    pub estimated_time_saved_minutes: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SubmitFeatureSignalRequest {
    pub schema: String,
    pub requirement_summary: String,
    #[serde(default)]
    pub need_key: Option<String>,
    pub industry: String,
    #[serde(default)]
    pub requested_outcome: String,
    pub merchant_authorized: bool,
    pub classification: String,
    #[serde(default)]
    pub evidence: FeatureSignalEvidence,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ErpFeatureSignal {
    pub id: String,
    pub blueprint_id: String,
    pub instance_id: String,
    pub need_key: String,
    pub requirement_summary: String,
    pub industry: String,
    pub requested_outcome: String,
    pub evidence: FeatureSignalEvidence,
    pub classification: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ErpFeatureProposal {
    pub id: String,
    pub blueprint_id: String,
    pub need_key: String,
    pub title: String,
    pub summary: String,
    pub status: String,
    pub support_count: i64,
    pub industries: Vec<String>,
    pub matter_id: Option<String>,
    pub decision_by: Option<String>,
    pub decision_note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct DecideProposalRequest {
    pub decision: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub create_matter: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CompatibilityIssue {
    pub code: String,
    pub severity: String,
    pub subject: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ErpCompatibilityReport {
    pub compatible: bool,
    pub from_version: String,
    pub target_version: String,
    pub preserved_private_extensions: Vec<ErpExtensionRef>,
    pub issues: Vec<CompatibilityIssue>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PrepareUpgradeRequest {
    pub target_version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct DecideUpgradeRequest {
    pub action: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub merchant_confirmed: bool,
    #[serde(default)]
    pub execution_attested: bool,
    #[serde(default)]
    pub verification_summary: String,
    #[serde(default)]
    pub deployed_commit: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ErpUpgradeCampaign {
    pub id: String,
    pub instance_id: String,
    pub from_version_id: String,
    pub target_version_id: String,
    pub status: String,
    pub compatibility: ErpCompatibilityReport,
    pub private_extensions_snapshot: Vec<ErpExtensionRef>,
    pub instance_revision: i64,
    pub adopted_instance_revision: Option<i64>,
    pub from_configuration: ErpInstanceConfiguration,
    pub target_configuration: ErpInstanceConfiguration,
    pub adoption_evidence: Option<ErpUpgradeAdoptionEvidence>,
    pub created_by: String,
    pub decided_by: Option<String>,
    pub rollback_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ErpProjectOverview {
    pub schema: &'static str,
    pub blueprint: Option<ErpBlueprint>,
    pub versions: Vec<ErpBlueprintVersion>,
    pub instance: Option<ErpInstance>,
    pub instances: Vec<ErpInstance>,
    pub proposals: Vec<ErpFeatureProposal>,
    pub upgrades: Vec<ErpUpgradeCampaign>,
    pub materialization: Option<ErpMaterializationStatus>,
    pub capability_catalog: Vec<ErpCapabilityDefinition>,
    pub catalog_version: Option<String>,
    pub unreleased_capability_keys: Vec<String>,
    pub boundaries: Value,
}

fn default_proposal_threshold() -> i64 {
    3
}
