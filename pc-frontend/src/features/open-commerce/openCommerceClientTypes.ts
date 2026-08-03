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

export interface DeveloperWebhookSubscription {
  schema: string
  id: string
  project_id: string
  app_record_id: string
  app_id: string
  callback_url: string
  signing_key_id: string
  signing_secret_version: number
  status: 'active' | 'disabled'
  verification_status: 'pending' | 'failed' | 'verified'
  verification_attempted_at?: string
  verification_error_code?: string
  verified_at?: string
  consecutive_failures: number
  last_delivery_at?: string
  last_error_code?: string
  created_at: string
  updated_at: string
  disabled_at?: string
}

export interface DeveloperWebhookCredential {
  schema: string
  subscription: DeveloperWebhookSubscription
  signing_secret: string
  signing_secret_visible_once: boolean
}

export interface DeveloperWebhookSubscriptionList {
  schema: string
  webhooks: DeveloperWebhookSubscription[]
}

export interface DeveloperWebhookDelivery {
  schema: string
  id: string
  subscription_id: string
  invocation_id: string
  event_sequence: number
  event_type: 'invocation.succeeded' | 'invocation.failed'
  status: 'pending' | 'delivering' | 'retry' | 'delivered' | 'dead'
  attempt_count: number
  manual_retry_count: number
  next_attempt_at: string
  response_status?: number
  error_code?: string
  created_at: string
  last_attempt_at?: string
  last_manual_retry_at?: string
  delivered_at?: string
}

export interface DeveloperWebhookDeliveryList {
  schema: string
  deliveries: DeveloperWebhookDelivery[]
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

export interface ConsumerRelationshipRenewalLink {
  source_relationship_id: string
  renewed_relationship_id: string
}

export interface ConsumerPortableMerchantIdentityClaim {
  source_merchant_id: string
  key_ids: string[]
  authority: 'merchant_private_key_possession'
}

export interface ConsumerPortabilityPayload {
  schema: string
  source_project_id: string
  generated_at: string
  relationships: ConsumerRelationship[]
  relationship_renewals: ConsumerRelationshipRenewalLink[]
  data_requests: ConsumerDataRequest[]
  preference_profile?: ConsumerPreferenceProfile
  preference_disclosures?: ConsumerPreferenceDisclosure[]
  invocation_receipt_scope?: 'authenticated_user_account'
  invocation_receipts?: ConsumerPortableInvocationReceipt[]
  merchant_identity_claims?: ConsumerPortableMerchantIdentityClaim[]
}

export interface ConsumerPortabilityExport {
  schema: string
  id: string
  source_project_id: string
  idempotency_key: string
  payload_sha256: string
  payload_json: string
  payload: ConsumerPortabilityPayload
  created_at: string
}

export interface ConsumerPortabilityExportSummary {
  id: string
  source_project_id: string
  idempotency_key: string
  payload_sha256: string
  relationship_count: number
  renewal_count: number
  data_request_count: number
  preference_profile_included: boolean
  preference_disclosure_count: number
  invocation_receipt_count: number
  merchant_identity_claim_count: number
  created_at: string
}

export interface ConsumerPortabilityExportList {
  schema: string
  exports: ConsumerPortabilityExportSummary[]
}

export interface ConsumerPortabilityImport {
  schema: 'open_commerce.consumer_portability_import.v1'
  id: string
  destination_project_id: string
  source_operator: string
  source_project_id: string
  source_package_id: string
  source_package_schema: string
  envelope_sha256: string
  payload_sha256: string
  package_json: string
  package: ConsumerPortabilityExport
  trust_status: ConsumerPortabilityImportTrustStatus
  merge_status: 'isolated_snapshot'
  signature?: ConsumerPortabilityPackageSignature
  signature_verified_at?: string
  imported_at: string
}

export interface ConsumerPortabilityImportSummary {
  id: string
  source_operator: string
  source_project_id: string
  source_package_id: string
  source_package_schema: string
  envelope_sha256: string
  payload_sha256: string
  relationship_count: number
  data_request_count: number
  preference_profile_included: boolean
  invocation_receipt_count: number
  merchant_identity_claim_count: number
  trust_status: ConsumerPortabilityImportTrustStatus
  merge_status: 'isolated_snapshot'
  signer_key_id?: string
  signature_verified_at?: string
  imported_at: string
}

export interface ConsumerPortabilityImportList {
  schema: 'open_commerce.consumer_portability_imports.v1'
  imports: ConsumerPortabilityImportSummary[]
}

export type ConsumerPortabilityImportTrustStatus =
  | 'integrity_verified_source_untrusted'
  | 'trusted_operator_signature_verified'

export interface ConsumerPortabilityPackageSignature {
  algorithm: 'rsa-pkcs1v15-sha256'
  key_id: string
  signature_base64: string
}

export interface SignedConsumerPortabilityPackage {
  schema: 'open_commerce.consumer_portability_signed_package.v1'
  source_operator: string
  package: ConsumerPortabilityExport
  signature: ConsumerPortabilityPackageSignature
}

export interface ConsumerPortabilityTrustKey {
  schema: 'open_commerce.consumer_portability_trust_key.v1'
  id: string
  source_operator: string
  key_id: string
  algorithm: 'rsa-pkcs1v15-sha256'
  public_key_pem: string
  status: 'active' | 'revoked'
  created_at: string
  revoked_at?: string
}

export interface ConsumerPortabilityTrustKeyList {
  schema: 'open_commerce.consumer_portability_trust_keys.v1'
  keys: ConsumerPortabilityTrustKey[]
}

export interface ConsumerPortabilityPreferenceChange {
  field: string
  current_value: unknown
  imported_value: unknown
  changed: boolean
}

export interface ConsumerPortabilityRelationshipCandidate {
  source_relationship_id: string
  source_merchant_id: string
  source_status: string
  requested_scopes: string[]
  purpose: string
  requires_reauthorization: true
  source_identity_key_ids: string[]
  verified_target_merchant_ids: string[]
  identity_match_authority?: 'trusted_operator_package_plus_matching_possession_key'
}

export interface ConsumerPortabilityAdoptionPlan {
  schema: 'open_commerce.consumer_portability_adoption_plan.v1'
  import_id: string
  import_trust_status: ConsumerPortabilityImportTrustStatus
  source_package_schema: string
  imported_profile_available: boolean
  current_profile_revision?: number
  preference_changes: ConsumerPortabilityPreferenceChange[]
  relationship_candidates: ConsumerPortabilityRelationshipCandidate[]
  automatic_relationship_restore: false
  automatic_business_write: false
}

export interface ConsumerPortabilityAdoption {
  schema: 'open_commerce.consumer_portability_adoption.v1'
  id: string
  import_id: string
  kind: 'preferences'
  before_preferences?: ConsumerPreferences
  before_revision?: number
  applied_preferences: ConsumerPreferences
  resulting_revision: number
  status: 'applied' | 'rolled_back'
  applied_at: string
  rolled_back_at?: string
  rollback_revision?: number
}

export interface ConsumerPortabilityAdoptionList {
  schema: 'open_commerce.consumer_portability_adoptions.v1'
  adoptions: ConsumerPortabilityAdoption[]
}

export interface PortabilityRelationshipMapping {
  schema: 'open_commerce.portability_relationship_mapping.v1'
  id: string
  import_id: string
  source_relationship_id: string
  source_merchant_id: string
  target_merchant_id: string
  target_merchant_project_id: string
  identity_match_status: 'not_verified' | 'trusted_operator_key_match'
  identity_match_key_id?: string
  status: 'active' | 'revoked'
  created_at: string
  revoked_at?: string
}

export interface PortabilityRelationshipMappingList {
  schema: 'open_commerce.portability_relationship_mappings.v1'
  mappings: PortabilityRelationshipMapping[]
}

export interface PortabilityReauthorizationResult {
  schema: 'open_commerce.portability_reauthorization_result.v1'
  mapping: PortabilityRelationshipMapping
  authorization_request: AuthorizationRequest
  old_grant_restored: false
}

export interface ConsumerPreferences {
  categories: string[]
  tags: string[]
  city?: string
  max_unit_price_micros?: number
  prefer_public: boolean
}

export interface ConsumerPreferenceProfile {
  preferences: ConsumerPreferences
  revision: number
  created_at: string
  updated_at: string
}

export interface ConsumerPreferenceProfileEnvelope {
  schema: string
  profile: ConsumerPreferenceProfile | null
}

export type ConsumerPreferenceField =
  | 'categories'
  | 'tags'
  | 'city'
  | 'max_unit_price_micros'

export interface DisclosedConsumerPreferences {
  categories?: string[]
  tags?: string[]
  city?: string
  max_unit_price_micros?: number
}

export interface ConsumerPreferenceDisclosure {
  relationship_id: string
  merchant_id: string
  subject_alias: string
  relationship_status: 'active' | 'expired' | 'revoked'
  shared_fields: ConsumerPreferenceField[]
  preferences: DisclosedConsumerPreferences
  profile_revision: number
  created_at: string
  updated_at: string
}

export interface ConsumerPreferenceDisclosureList {
  schema: string
  disclosures: ConsumerPreferenceDisclosure[]
}

export interface DeleteConsumerPreferenceProfileResult {
  deleted_profile: boolean
  removed_disclosures: number
}

export interface DeleteConsumerPreferenceDisclosureResult {
  relationship_id: string
  deleted: boolean
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

export interface DirectoryMerchantList {
  schema: string
  merchants: DirectoryMerchantDetail[]
}

export interface PublicMerchantIdentityKey {
  key_id: string
  algorithm: 'rsa-pkcs1v15-sha256'
  proof_verified_at: string
  created_at: string
}

export interface DirectoryMerchantDetail {
  schema: string
  merchant: DirectoryMerchant
  capabilities: DirectoryCapability[]
  portable_identity_keys: PublicMerchantIdentityKey[]
}

export interface MerchantIdentityKey extends PublicMerchantIdentityKey {
  schema: 'open_commerce.merchant_identity_key.v1'
  id: string
  project_id: string
  merchant_id: string
  public_key_pem: string
  status: 'active' | 'revoked'
  revoked_at?: string
}

export interface MerchantIdentityKeyList {
  schema: string
  keys: MerchantIdentityKey[]
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
  capability_contract_profile: string
  requester_app_id: string
  ranking_policy: string
  ranking_is_paid: boolean
  matches: ConsumerDiscoveryMatch[]
}

export interface ConsumerInvocationReceiptSummary {
  invocation_id: string
  merchant_id: string
  capability_key: string
  requester_app_id: string
  status: 'succeeded' | 'failed'
  result_available: boolean
  error_code: string | null
  amount_micros: number
  currency: string
  settlement_status: string
  funds_moved: false
  created_at: string
  completed_at: string
}

export interface ConsumerInvocationReceiptList {
  schema: string
  scope: 'authenticated_user_account'
  receipts: ConsumerInvocationReceiptSummary[]
}

export interface ConsumerInvocationReceiptPayload {
  schema: string
  invocation_id: string
  merchant_id: string
  capability_key: string
  requester_app_id: string
  request_shape: {
    input_fields: string[]
    input_bytes: number
    contains_raw_values: false
  }
  result: unknown | null
  error_code: string | null
  status: 'succeeded' | 'failed'
  units: number
  unit_price_micros: number
  amount_micros: number
  currency: string
  settlement_status: string
  funds_moved: false
  created_at: string
  completed_at: string
}

export interface ConsumerPortableInvocationReceipt {
  schema: string
  payload_sha256: string
  payload_json: string
}

export interface ConsumerInvocationReceipt {
  schema: string
  payload_sha256: string
  payload_json: string
  payload: ConsumerInvocationReceiptPayload
}

export interface DeveloperInvokeRequest {
  merchant_id: string
  capability_key: string
  grant_id?: string
  idempotency_key: string
  action_confirmation_id?: string
  input: Record<string, unknown>
}

export interface DeveloperTerminalEventSummary {
  schema: string
  event_id: string
  event_type: 'invocation.succeeded' | 'invocation.failed'
  invocation_id: string
  merchant_id: string
  capability_key: string
  idempotency_key: string
  status: 'succeeded' | 'failed'
  result_available: boolean
  error_code: string | null
  units: number
  amount_micros: number
  currency: string
  settlement_status: string
  funds_moved: false
  created_at: string
  completed_at: string
}

export interface DeveloperTerminalEventPage {
  schema: string
  app_id: string
  events: DeveloperTerminalEventSummary[]
  next_cursor: string | null
  has_more: boolean
}

export interface DeveloperTerminalEventDetail {
  schema: string
  event: DeveloperTerminalEventSummary
  result: unknown | null
}
