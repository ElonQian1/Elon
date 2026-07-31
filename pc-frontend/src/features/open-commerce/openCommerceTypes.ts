export type OpenCommerceAccessLevel = 'public' | 'authorized' | 'owner_only'
export type OpenCommerceHandlerType = 'merchant_profile' | 'static_json' | 'merchant_runtime'
export type OpenCommerceCapabilityKind = 'query' | 'action'

export interface OpenCommerceMerchant {
  id: string
  project_id: string
  owner_user_id: string
  slug: string
  display_name: string
  description: string
  status: string
  node_mode: string
  public_profile: Record<string, unknown>
  created_at: string
  updated_at: string
}

export interface OpenCommerceCapability {
  id: string
  merchant_id: string
  capability_key: string
  display_name: string
  description: string
  kind: OpenCommerceCapabilityKind
  access_level: OpenCommerceAccessLevel
  input_schema: Record<string, unknown>
  output_schema: Record<string, unknown>
  handler_type: OpenCommerceHandlerType
  handler_config?: Record<string, unknown>
  unit_price_micros: number
  currency: string
  freshness_seconds: number
  status: string
  version: number
  created_at: string
  updated_at: string
}

export interface OpenCommerceMerchantDetail {
  schema: string
  merchant: OpenCommerceMerchant
  capabilities: OpenCommerceCapability[]
}

export interface OpenCommerceGrant {
  id: string
  project_id: string
  merchant_id: string
  grantor_user_id: string
  grantee_app_id: string
  scopes: string[]
  purpose: string
  expires_at?: string
  revoked_at?: string
  created_at: string
  updated_at: string
}

export interface OpenCommerceInvocation {
  id: string
  merchant_id: string
  capability_key: string
  requester_app_id: string
  status: string
  amount_micros: number
  currency: string
  settlement_status: string
  created_at: string
}

export interface OpenCommerceAuditEvent {
  id: string
  actor_app_id?: string
  action: string
  subject_type: string
  subject_id: string
  metadata: Record<string, unknown>
  created_at: string
}

export interface OpenCommerceIntegration {
  id: string
  project_id: string
  merchant_id: string
  integration_key: string
  provider_key: string
  display_name: string
  connection_mode: 'official_api' | 'merchant_export' | 'local_adapter' | 'manual_import'
  status: 'configured' | 'connected' | 'degraded' | 'disabled'
  scopes: string[]
  data_domains: string[]
  last_verified_at?: string
  last_sync_at?: string
  created_at: string
  updated_at: string
}

export interface OpenCommerceSyncReceipt {
  id: string
  integration_id: string
  receipt_key: string
  sync_kind: 'full' | 'incremental' | 'health_check'
  status: 'succeeded' | 'partial' | 'failed'
  records_seen: number
  records_changed: number
  error_code?: string
  started_at: string
  completed_at: string
  created_at: string
}

export interface OpenCommerceRuntimeBinding {
  id: string
  project_id: string
  merchant_id: string
  endpoint_base_url: string
  credential_ref: string
  manifest_sha256?: string
  timeout_ms: number
  status: 'configured' | 'active' | 'degraded' | 'disabled'
  last_verified_at?: string
  last_error_code?: string
  created_by_user_id: string
  created_at: string
  updated_at: string
}

export interface OpenCommerceOverview {
  schema: string
  project_id: string
  merchants: OpenCommerceMerchantDetail[]
  grants: OpenCommerceGrant[]
  recent_invocations: OpenCommerceInvocation[]
  integrations: OpenCommerceIntegration[]
  runtime_bindings: OpenCommerceRuntimeBinding[]
  recent_sync_receipts: OpenCommerceSyncReceipt[]
  recent_audit_events: OpenCommerceAuditEvent[]
  totals: {
    merchants: number
    active_merchants: number
    capabilities: number
    active_capabilities: number
    active_grants: number
    invocations: number
    integrations: number
    connected_integrations: number
    degraded_integrations: number
    sync_receipts: number
    active_runtime_bindings: number
    metered_amount_micros: number
  }
}

export interface UpsertOpenCommerceRuntimeBinding {
  endpoint_base_url: string
  credential_ref: string
  manifest_sha256?: string
  timeout_ms: number
}

export interface CreateOpenCommerceIntegration {
  merchant_id: string
  integration_key: string
  provider_key: string
  display_name: string
  connection_mode: OpenCommerceIntegration['connection_mode']
  scopes: string[]
  data_domains: string[]
}

export interface RecordOpenCommerceSyncReceipt {
  integration_id: string
  receipt_key: string
  sync_kind: OpenCommerceSyncReceipt['sync_kind']
  status: OpenCommerceSyncReceipt['status']
  records_seen: number
  records_changed: number
  error_code?: string
  started_at: string
  completed_at: string
}

export interface CreateOpenCommerceMerchant {
  display_name: string
  slug?: string
  description: string
  node_mode: 'platform_hosted' | 'self_hosted' | 'third_party_hosted'
  public_profile: Record<string, unknown>
}

export interface CreateOpenCommerceCapability {
  capability_key: string
  display_name: string
  description: string
  kind: OpenCommerceCapabilityKind
  access_level: OpenCommerceAccessLevel
  input_schema: Record<string, unknown>
  output_schema: Record<string, unknown>
  handler_type: OpenCommerceHandlerType
  handler_config?: Record<string, unknown>
  unit_price_micros: number
  currency: string
  freshness_seconds: number
}

export interface CreateOpenCommerceGrant {
  merchant_id: string
  grantee_app_id: string
  scopes: string[]
  purpose: string
  expires_at?: string
}

export interface InvokeOpenCommerceCapability {
  merchant_id: string
  capability_key: string
  requester_app_id: string
  grant_id?: string
  idempotency_key: string
  input: Record<string, unknown>
}
