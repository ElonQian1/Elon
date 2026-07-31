import type { OpenCommerceCapability, OpenCommerceMerchant } from './openCommerceTypes'

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
  created_at: string
  updated_at: string
}

export interface AuthorizationRequestList {
  schema: string
  requests: AuthorizationRequest[]
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
  status: 'not_required' | 'owner_only' | 'granted' | 'pending' | 'request_required'
  grant_id?: string
  request_id?: string
}

export interface ConsumerDiscoveryMatch {
  merchant: OpenCommerceMerchant
  capability: OpenCommerceCapability
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
