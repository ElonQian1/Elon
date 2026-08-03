export type SuiPreflightPackageKind = 'standard' | 'correction'
export type SuiPreflightOutcome = 'passed' | 'rejected'

export interface SuiPreflightAdapter {
  schema: 'task_economy.sui_preflight_adapter.v1'
  id: string
  project_id: string
  display_name: string
  status: 'active' | 'disabled'
  allowed_networks: Array<'devnet' | 'testnet' | 'mainnet'>
  allowed_package_kinds: SuiPreflightPackageKind[]
  token_hint: string
  credential_version: number
  created_by_user_id: string
  last_used_at: string | null
  expires_at: string
  is_expired: boolean
  disabled_at: string | null
  created_at: string
  updated_at: string
}

export interface SuiPreflightAdapterIssue {
  schema: 'task_economy.sui_preflight_adapter_issue.v1'
  adapter: SuiPreflightAdapter
  adapter_token: string
  token_visible_once: true
}

export interface SuiPreflightAdapterList {
  schema: 'task_economy.sui_preflight_adapter_list.v1'
  project_id: string
  runtime_enabled: boolean
  adapters: SuiPreflightAdapter[]
  boundary: string[]
}

export interface SuiPreflightReport {
  schema: 'task_economy.sui_preflight_report.v1'
  id: string
  project_id: string
  adapter_id: string
  credential_version: number
  package_kind: SuiPreflightPackageKind
  projection_package_id: string
  target_network: 'devnet' | 'testnet' | 'mainnet'
  handoff_digest: string
  projection_digest: string
  outcome: SuiPreflightOutcome
  summary: string
  tool_version: string
  idempotency_key: string
  report_digest: string
  created_at: string
}

export interface SuiPreflightReportList {
  schema: 'task_economy.sui_preflight_report_list.v1'
  project_id: string
  runtime_enabled: boolean
  reports: SuiPreflightReport[]
  boundary: string[]
}
