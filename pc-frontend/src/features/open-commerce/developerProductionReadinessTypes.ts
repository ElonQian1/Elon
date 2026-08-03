export type DeveloperProductionReadinessStepCode =
  | 'app'
  | 'manifest'
  | 'domain'
  | 'admission'
  | 'credential_gateway'
  | 'credential'
  | 'webhook_gateway'
  | 'webhook'

export interface DeveloperProductionReadinessStep {
  code: DeveloperProductionReadinessStepCode
  ready: boolean
  blocker_code: string | null
}

export interface DeveloperProductionReadinessSummary {
  schema: 'open_commerce.developer_production_readiness.v1'
  app_record_id: string
  app_id: string
  manifest_revision: number
  admission_status: string | null
  admission_revision: number | null
  production_credentials_enabled: boolean
  current_production_credential_present: boolean
  production_webhooks_enabled: boolean
  active_production_webhook_count: number
  production_invocation_ready: boolean
  production_webhook_ready: boolean
  next_action_code: string | null
  blocker_codes: string[]
  steps: DeveloperProductionReadinessStep[]
  generated_at: string
}
