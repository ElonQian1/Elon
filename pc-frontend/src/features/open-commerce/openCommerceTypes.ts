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

export interface OpenCommerceDirectoryPublication {
  merchant_id: string
  project_id: string
  status: 'published' | 'unpublished'
  revision: number
  published_by_user_id?: string
  published_at?: string
  unpublished_at?: string
  updated_at: string
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
  max_invocations?: number
  max_amount_micros?: number
  budget_currency: string
  used_invocations: number
  used_amount_micros: number
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

export interface MerchantBusinessReceipt {
  schema: 'open_commerce.merchant_business_receipt.v1'
  entity_type: string
  reference_id: string
  state: string
  occurred_at: string
  amount_minor?: number
  currency?: string
}

export interface MerchantEvidenceErpBinding {
  instance_id: string
  instance_key: string
  configuration_revision: number
}

export interface MerchantBusinessEvidenceSummary {
  schema: 'open_commerce.merchant_business_evidence.v1'
  sequence: number
  invocation_id: string
  merchant_id: string
  erp_binding?: MerchantEvidenceErpBinding
  capability_key: string
  capability_kind: OpenCommerceCapabilityKind
  requester_app_id: string
  status: 'succeeded' | 'failed'
  source_authority: 'merchant_runtime_asserted' | 'platform_handler_result'
  receipt_state: 'valid' | 'digest_only' | 'invalid_legacy' | 'not_available' | 'not_applicable'
  business_receipt?: MerchantBusinessReceipt
  result_available: boolean
  result_sha256?: string
  error_code?: string
  amount_micros: number
  currency: string
  settlement_status: string
  funds_moved: false
  created_at: string
  completed_at: string
}

export interface MerchantBusinessEvidenceList {
  schema: 'open_commerce.merchant_business_evidence_list.v1'
  project_id: string
  merchant_id: string
  erp_binding?: MerchantEvidenceErpBinding
  evidence: MerchantBusinessEvidenceSummary[]
  boundary: string[]
}

export interface MerchantBusinessEvidenceDetail {
  schema: 'open_commerce.merchant_business_evidence_detail.v1'
  evidence: MerchantBusinessEvidenceSummary
  result?: unknown
  boundary: string[]
}

export type OpenCommerceBusinessHandoffStatus = 'applied' | 'ignored' | 'rejected'
export type OpenCommerceBusinessHandoffTarget = 'erp' | 'crm'
export type OpenCommerceBusinessHandoffQueueState = 'pending' | 'retry_required'

export interface OpenCommerceBusinessHandoffReceipt {
  schema: 'open_commerce.business_handoff_receipt.v1'
  id: string
  project_id: string
  merchant_id: string
  invocation_id: string
  integration_id: string
  receipt_key: string
  status: OpenCommerceBusinessHandoffStatus
  target_domain: OpenCommerceBusinessHandoffTarget
  evidence_result_sha256: string
  target_reference_sha256?: string
  error_code?: string
  confirmed_by_user: boolean
  assertion_authority: 'project_editor_asserted' | 'adapter_token_authenticated'
  adapter_credential_id?: string
  adapter_credential_version?: number
  adapter_claim_id?: string
  recorded_by_user_id: string
  recorded_by_app_id: string
  completed_at: string
  created_at: string
  funds_moved: false
}

export interface OpenCommerceAdapterCredential {
  schema: 'open_commerce.adapter_credential.v1'
  id: string
  project_id: string
  merchant_id: string
  integration_id: string
  status: 'active' | 'revoked'
  scopes: string[]
  token_hint: string
  credential_version: number
  created_by_user_id: string
  last_used_at?: string
  expires_at: string
  is_expired: boolean
  created_at: string
  updated_at: string
}

export interface OpenCommerceAdapterCredentialIssue {
  schema: 'open_commerce.adapter_credential_issue.v1'
  credential: OpenCommerceAdapterCredential
  adapter_token: string
  token_visible_once: true
}

export interface OpenCommerceAdapterCredentialList {
  schema: 'open_commerce.adapter_credential_list.v1'
  project_id: string
  credentials: OpenCommerceAdapterCredential[]
  boundary: string[]
}

export interface OpenCommerceAdapterHandoffClaim {
  schema: 'open_commerce.adapter_business_handoff_claim.v1'
  id: string
  project_id: string
  merchant_id: string
  invocation_id: string
  integration_id: string
  adapter_credential_id: string
  adapter_credential_version: number
  attempt_no: number
  status: 'active' | 'completed' | 'expired' | 'released'
  lease_token_hint: string
  lease_expires_at: string
  lease_deadline_at: string
  release_reason_code?: string
  released_at?: string
  completion_status?: 'applied' | 'ignored' | 'rejected'
  retry_not_before?: string
  retry_suspended_at?: string
  retry_suspension_reason?: string
  retry_resumed_at?: string
  completed_receipt_id?: string
  created_at: string
  updated_at: string
}

export interface OpenCommerceAdapterHandoffClaimList {
  schema: 'open_commerce.adapter_business_handoff_claim_list.v1'
  project_id: string
  claims: OpenCommerceAdapterHandoffClaim[]
  boundary: string[]
}

export interface OpenCommerceAdapterHandoffClaimResume {
  schema: 'open_commerce.adapter_business_handoff_claim_resume.v1'
  claim: OpenCommerceAdapterHandoffClaim
  resumed: true
  funds_moved: false
  boundary: string[]
}

export interface OpenCommerceBusinessHandoffReceiptList {
  schema: 'open_commerce.business_handoff_receipt_list.v1'
  project_id: string
  merchant_id: string
  receipts: OpenCommerceBusinessHandoffReceipt[]
  boundary: string[]
}

export interface OpenCommerceBusinessHandoffQueueItem {
  schema: 'open_commerce.business_handoff_queue_item.v1'
  queue_state: OpenCommerceBusinessHandoffQueueState
  can_apply: boolean
  evidence: MerchantBusinessEvidenceSummary
  latest_receipt: OpenCommerceBusinessHandoffReceipt | null
}

export interface OpenCommerceBusinessHandoffQueue {
  schema: 'open_commerce.business_handoff_queue.v1'
  project_id: string
  merchant_id: string
  state_filter: OpenCommerceBusinessHandoffQueueState | null
  items: OpenCommerceBusinessHandoffQueueItem[]
  returned_pending_count: number
  returned_retry_required_count: number
  has_more: boolean
  boundary: string[]
}

export interface RecordOpenCommerceBusinessHandoffReceipt {
  merchant_id: string
  invocation_id: string
  integration_id: string
  receipt_key: string
  status: OpenCommerceBusinessHandoffStatus
  target_domain: OpenCommerceBusinessHandoffTarget
  evidence_result_sha256: string
  target_reference?: string
  error_code?: string
  confirmed_by_user: boolean
  completed_at: string
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

export interface OpenCommerceAppActivityHealth {
  merchant_id: string
  requester_app_id: string
  status: 'normal' | 'attention'
  total_invocations_24h: number
  succeeded_invocations_24h: number
  failed_invocations_24h: number
  rate_limited_invocations_24h: number
  grant_budget_rejections_24h: number
  recovered_invocations_24h: number
  last_invoked_at: string
  attention_codes: Array<
    | 'recovered_invocation'
    | 'repeated_failures'
    | 'rate_limit_pressure'
    | 'grant_budget_pressure'
  >
}

export interface OpenCommerceRateLimitPolicy {
  id: string
  project_id: string
  merchant_id: string
  capability_id: string
  capability_key: string
  requester_app_id?: string
  window_seconds: number
  max_requests: number
  status: 'active' | 'disabled'
  created_by_user_id: string
  created_at: string
  updated_at: string
}

export interface OpenCommerceRateLimitUsage {
  policy_id: string
  window_started_at_unix: number
  accepted_requests: number
  active_subjects: number
}

export type OpenCommerceAppBlockReason =
  | 'abusive_traffic'
  | 'policy_violation'
  | 'security_incident'
  | 'merchant_request'
  | 'other'

export interface OpenCommerceAppBlock {
  id: string
  project_id: string
  merchant_id: string
  requester_app_id: string
  reason_code: OpenCommerceAppBlockReason
  reason_note: string
  status: 'active' | 'unblocked'
  blocked_by_user_id: string
  unblocked_by_user_id?: string
  blocked_at: string
  unblocked_at?: string
  created_at: string
  updated_at: string
}

export interface OpenCommerceAppBlockOutcome {
  block: OpenCommerceAppBlock
  revoked_grants: number
  canceled_authorization_requests: number
  grants_restored: number
}

export interface BlockOpenCommerceApp {
  merchant_id: string
  requester_app_id: string
  reason_code: OpenCommerceAppBlockReason
  reason_note: string
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

export interface OpenCommerceCapabilitySourceLink {
  id: string
  project_id: string
  merchant_id: string
  capability_id: string
  capability_key: string
  capability_version: number
  current_capability_version: number
  integration_id: string
  sync_receipt_id: string
  data_domain: string
  provider_key: string
  connection_mode: OpenCommerceIntegration['connection_mode']
  integration_status: OpenCommerceIntegration['status']
  sync_kind: OpenCommerceSyncReceipt['sync_kind']
  receipt_status: OpenCommerceSyncReceipt['status']
  receipt_sha256: string
  receipt_completed_at: string
  revision: number
  linked_by_user_id: string
  created_at: string
  updated_at: string
  publishable: boolean
  blocking_reason?: string
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
  directory_publications: OpenCommerceDirectoryPublication[]
  grants: OpenCommerceGrant[]
  recent_invocations: OpenCommerceInvocation[]
  integrations: OpenCommerceIntegration[]
  runtime_bindings: OpenCommerceRuntimeBinding[]
  recent_sync_receipts: OpenCommerceSyncReceipt[]
  capability_source_links: OpenCommerceCapabilitySourceLink[]
  recent_audit_events: OpenCommerceAuditEvent[]
  rate_limit_policies: OpenCommerceRateLimitPolicy[]
  rate_limit_usage: OpenCommerceRateLimitUsage[]
  app_activity_health: OpenCommerceAppActivityHealth[]
  totals: {
    merchants: number
    active_merchants: number
    published_merchants: number
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
    rate_limit_policies: number
    active_rate_limit_policies: number
    recent_rate_limited_invocations: number
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
  max_invocations?: number
  max_amount_micros?: number
  budget_currency?: string
}

export interface InvokeOpenCommerceCapability {
  merchant_id: string
  capability_key: string
  requester_app_id: string
  grant_id?: string
  idempotency_key: string
  action_confirmation_id?: string
  input: Record<string, unknown>
}

export interface OpenCommerceActionConfirmation {
  id: string
  merchant_id: string
  capability_key: string
  requester_app_id: string
  idempotency_key: string
  status: 'pending' | 'confirmed' | 'consumed' | 'expired'
  expires_at: string
  canceled_at?: string
}

export interface UpsertOpenCommerceRateLimit {
  merchant_id: string
  capability_key: string
  requester_app_id?: string
  window_seconds: number
  max_requests: number
  enabled: boolean
}
