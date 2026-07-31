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
  status: string
  created_by: string
  created_at: string
  updated_at: string
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
  capability_catalog: ErpCapability[]
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
}
