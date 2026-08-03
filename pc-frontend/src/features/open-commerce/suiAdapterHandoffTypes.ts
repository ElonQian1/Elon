import type { SuiTargetNetwork } from './taskEconomyTypes'

export interface SuiAdapterHandoffConstraints {
  allowed_adapter_action: 'offline_preflight_only'
  signature_present: false
  transaction_broadcast: false
  finality_verified: false
  funds_moved: false
}

export interface SuiAdapterHandoffBundle {
  schema: 'task_economy.sui_adapter_handoff.v1'
  package_kind: 'standard' | 'correction'
  project_id: string
  projection_package_id: string
  source_id: string
  target_network: SuiTargetNetwork
  package_schema: string
  projection_digest: string
  source_digest: string
  envelope: Record<string, unknown>
  shadow_only: true
  atomic_bundle: boolean
  network_submission: 'not_submitted'
  submission_attempts: 0
  package_created_at: string
  constraints: SuiAdapterHandoffConstraints
  handoff_digest: string
}
