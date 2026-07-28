export type OpenCommerceAccessLevel = 'public' | 'authorized' | 'owner_only'
export type OpenCommerceHandlerType = 'merchant_profile' | 'static_json'
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

export interface OpenCommerceOverview {
  schema: string
  project_id: string
  merchants: OpenCommerceMerchantDetail[]
  grants: OpenCommerceGrant[]
  recent_invocations: OpenCommerceInvocation[]
  recent_audit_events: OpenCommerceAuditEvent[]
  totals: {
    merchants: number
    active_merchants: number
    capabilities: number
    active_capabilities: number
    active_grants: number
    invocations: number
    metered_amount_micros: number
  }
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
  requester_app_id: 'pc-web'
  grant_id?: string
  idempotency_key: string
  input: Record<string, unknown>
}
