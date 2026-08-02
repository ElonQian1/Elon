export interface OpenCommerceDeveloperApp {
  id: string
  project_id: string
  owner_user_id: string
  app_id: string
  display_name: string
  environment: 'sandbox'
  status: 'active' | 'disabled'
  token_hint: string
  created_at: string
  updated_at: string
}

export interface DeveloperAppCredential {
  schema: string
  app: OpenCommerceDeveloperApp
  test_token: string
  token_visible_once: boolean
}

export interface DeveloperAppList {
  schema: string
  apps: OpenCommerceDeveloperApp[]
}

export interface AuthorizationRequest {
  id: string
  merchant_project_id: string
  merchant_id: string
  requester_user_id: string
  requester_app_id: string
  scopes: string[]
  purpose: string
  status: 'pending' | 'approved' | 'rejected' | 'canceled'
  decided_by_user_id?: string
  decision_reason?: string
  grant_id?: string
  grant_expires_at?: string
  grant_max_invocations?: number
  grant_max_amount_micros?: number
  grant_budget_currency?: string
  created_at: string
  updated_at: string
}

export interface AuthorizationRequestList {
  schema: string
  requests: AuthorizationRequest[]
}

export interface ConsumerRelationship {
  id: string
  merchant_id: string
  source_app_id: string
  subject_alias: string
  scopes: Array<'preference.remember' | 'membership.link'>
  purpose: string
  status: 'active' | 'expired' | 'revoked'
  expires_at: string
  revoked_at?: string
  created_at: string
  updated_at: string
}

export interface ConsumerRelationshipList {
  schema: string
  relationships: ConsumerRelationship[]
}

export interface ConsumerDataRequest {
  id: string
  relationship_id: string
  merchant_id: string
  subject_alias: string
  request_type: 'erase_linked_data'
  status: 'requested' | 'in_progress' | 'completed' | 'rejected' | 'withdrawn'
  resolution_kind?: 'merchant_processing' | 'merchant_attested_completed' | 'merchant_rejected' | 'consumer_withdrawn'
  resolution_note?: string
  requested_at: string
  accepted_at?: string
  resolved_at?: string
  withdrawn_at?: string
  updated_at: string
}

export interface ConsumerDataRequestList {
  schema: string
  requests: ConsumerDataRequest[]
}

export interface ConsumerPreferences {
  categories: string[]
  tags: string[]
  city?: string
  max_unit_price_micros?: number
  prefer_public: boolean
}

export interface ConsumerDiscoveryRequest {
  query?: string
  capability_key?: string
  requester_app_id: string
  preferences: ConsumerPreferences
  limit: number
}

export interface ConsumerAuthorizationState {
  required: boolean
  status: 'not_required' | 'app_registration_required' | 'granted' | 'pending' | 'request_required'
  grant_id?: string
  request_id?: string
}

export interface DirectoryMerchant {
  id: string
  slug: string
  display_name: string
  description: string
  public_profile: Record<string, unknown>
  directory_revision: number
  published_at: string
  updated_at: string
}

export interface DirectoryCapability {
  capability_key: string
  display_name: string
  description: string
  kind: 'query' | 'action'
  access_level: 'public' | 'authorized'
  input_schema: Record<string, unknown>
  output_schema: Record<string, unknown>
  unit_price_micros: number
  currency: string
  freshness_seconds: number
  version: number
  updated_at: string
}

export interface ConsumerDiscoveryMatch {
  merchant: DirectoryMerchant
  capability: DirectoryCapability
  score: number
  reasons: string[]
  authorization: ConsumerAuthorizationState
}

export interface ConsumerDiscoveryResponse {
  schema: string
  requester_app_id: string
  ranking_policy: string
  ranking_is_paid: boolean
  matches: ConsumerDiscoveryMatch[]
}

export interface DeveloperInvokeRequest {
  merchant_id: string
  capability_key: string
  grant_id?: string
  idempotency_key: string
  input: Record<string, unknown>
}
