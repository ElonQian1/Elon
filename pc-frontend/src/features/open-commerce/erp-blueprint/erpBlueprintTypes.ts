export type ErpModule = {
  module_key: string
  version: string
  kind: 'core' | 'industry' | 'integration'
  required: boolean
  dependencies: string[]
}
export type ErpCapability = {
  capability_key: string
  display_name: string
  description: string
  category: string
  module_key: string
  aliases: string[]
  composable_with: string[]
}

export type ErpExtension = {
  extension_key: string
  version: string
  extension_point: string
  requires_modules: string[]
}

export type ErpBlueprint = {
  id: string
  definition: {
    schema: string
    blueprint_key: string
    name: string
    description: string
    source_project_id: string
    modules: ErpModule[]
    capabilities: ErpCapability[]
    themes: string[]
    extension_points: string[]
    proposal_threshold: number
  }
  definition_revision: number
  status: string
  created_by: string
  created_at: string
  updated_at: string
}

export type ErpReleaseManifest = {
  schema: string
  blueprint_key: string
  version: string
  previous_version: string | null
  source_git_commit: string
  modules: Array<{ module_key: string; version: string; required: boolean }>
  capabilities: string[]
  extension_points: string[]
  migrations: Array<{ migration_key: string; reversible: boolean }>
  compatibility: { minimum_instance_version: string; required_plugins: string[] }
  rollback: { supported: boolean; instructions: string }
}

export type ErpBlueprintVersion = {
  id: string
  blueprint_id: string
  manifest: ErpReleaseManifest
  manifest_sha256: string
  status: string
  created_by: string
  created_at: string
}

export type ErpInstance = {
  id: string
  instance_key: string
  project_id: string
  blueprint_id: string
  pinned_version_id: string
  pinned_version: string
  industry: string
  theme_key: string
  enabled_modules: string[]
  plugins: ErpExtension[]
  private_extensions: ErpExtension[]
  configuration_revision: number
  open_commerce_merchant_id?: string | null
  bootstrap_matter_id?: string | null
  onboarding_mode: 'new_project' | 'existing_project'
  status: string
  created_by: string
  created_at: string
  updated_at: string
}

export type ErpMaterializationStatus = {
  schema: string
  state: string
  recoverable: boolean
  contract: {
    schema: string
    instance_id: string
    instance_key: string
    target_project_id: string
    target_onboarding_mode: 'new_project' | 'existing_project'
    source: {
      project_id: string
      git_commit: string
      blueprint_key: string
      version: string
    }
    configuration: {
      revision: number
      open_commerce_merchant_id?: string | null
      industry: string
      theme_key: string
      enabled_modules: string[]
      plugins: ErpExtension[]
      private_extensions: ErpExtension[]
    }
    required_artifact: {
      artifact_kind: string
      instance_manifest_path: string
      evidence_schema: string
      required_metadata_fields: string[]
    }
    verification_requirements: string[]
    boundaries: string[]
  }
  matter?: {
    id: string
    status: string
    decision?: string | null
    plan_contract_matches: boolean
    assignments: {
      total: number
      planned: number
      running: number
      completed: number
      failed: number
      failed_assignment_ids: string[]
    }
  } | null
  evidence: Array<{
    artifact_id: string
    assignment_id: string
    valid: boolean
    issues: string[]
    created_at: string
  }>
  blockers: string[]
  next_action: string
}

export type ErpOpenCommerceReadiness = {
  schema: 'yilong.erp.open_commerce_readiness.v1'
  project_id: string
  instance_id: string
  overall_state: 'ready' | 'consumer_ready_erp_pending' | 'erp_ready_commerce_pending' | 'blocked'
  erp_onboarding_ready: boolean
  consumer_invocation_ready: boolean
  consumer_discovery_ready: boolean
  materialization: {
    state: string
    recoverable: boolean
    blockers: string[]
    next_action: string
  }
  merchant_selection: {
    status: 'merchant_missing' | 'bound_merchant_missing' | 'selection_required' | 'selected_implicit' | 'selected_explicit' | 'selected_binding'
    selected?: ErpOpenCommerceMerchantSummary | null
    candidates: ErpOpenCommerceMerchantSummary[]
  }
  runtime?: {
    status: string
    manifest_sha256?: string | null
    last_verified_at?: string | null
    last_error_code?: string | null
  } | null
  active_runtime_capability_keys: string[]
  directory?: {
    status: string
    revision: number
    published_at?: string | null
  } | null
  blockers: Array<{
    code: string
    scope: 'erp_onboarding' | 'consumer_invocation' | 'consumer_discovery'
    message: string
    next_action: string
  }>
}

export type ErpOpenCommerceMerchantSummary = {
  id: string
  display_name: string
  status: string
  node_mode: string
}

export type ErpTargetProject = {
  id: string
  name: string
  display_name?: string | null
  role?: string
  my_role?: string
  viewer_role?: string
}

export type ErpTargetProjectList = {
  projects?: ErpTargetProject[]
}

export type ErpProposal = {
  id: string
  blueprint_id: string
  need_key: string
  title: string
  summary: string
  status: 'candidate' | 'accepted' | 'rejected' | 'matter_created'
  support_count: number
  industries: string[]
  matter_id?: string | null
  decision_note?: string | null
  updated_at: string
}

export type CompatibilityIssue = {
  code: string
  severity: 'blocking' | 'warning'
  subject: string
  message: string
}

export type ErpUpgrade = {
  id: string
  instance_id: string
  from_version_id: string
  target_version_id: string
  status: 'checking' | 'ready' | 'blocked' | 'adopted' | 'rolled_back'
  compatibility: {
    compatible: boolean
    from_version: string
    target_version: string
    preserved_private_extensions: ErpExtension[]
    issues: CompatibilityIssue[]
  }
  private_extensions_snapshot: ErpExtension[]
  instance_revision: number
  adopted_instance_revision?: number | null
  from_configuration: ErpInstanceConfiguration
  target_configuration: ErpInstanceConfiguration
  adoption_evidence?: ErpUpgradeAdoptionEvidence | null
  rollback_reason?: string | null
  updated_at: string
}

export type ErpOverview = {
  schema: string
  blueprint: ErpBlueprint | null
  versions: ErpBlueprintVersion[]
  instance: ErpInstance | null
  instances: ErpInstance[]
  proposals: ErpProposal[]
  upgrades: ErpUpgrade[]
  materialization?: ErpMaterializationStatus | null
  capability_catalog: ErpCapability[]
  catalog_version?: string | null
  unreleased_capability_keys: string[]
  boundaries: Record<string, boolean | string>
}

export type RequirementResolution = {
  schema: string
  classification: 'existing' | 'composition' | 'private_extension' | 'candidate_common'
  requirement: string
  matched_capabilities: ErpCapability[]
  need_key?: string | null
  recommendation: string
  may_submit_signal: boolean
  catalog_version?: string | null
}

export type ErpInstanceConfiguration = {
  theme_key: string
  enabled_modules: string[]
  plugins: ErpExtension[]
}

export type ErpUpgradeAdoptionEvidence = {
  execution_attested: boolean
  verification_summary: string
  deployed_commit?: string | null
}

export type UpdateErpInstanceRequest = ErpInstanceConfiguration & {
  expected_revision: number
  merchant_confirmed: boolean
  private_extensions: ErpExtension[]
}

export type DecideErpUpgradeRequest = {
  action: 'adopt' | 'rollback'
  reason: string
  merchant_confirmed: boolean
  execution_attested: boolean
  verification_summary: string
  deployed_commit?: string | null
}
