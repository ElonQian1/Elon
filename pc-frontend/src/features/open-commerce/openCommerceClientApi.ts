import { api } from '../../api/client'
import { developerWebhookClientApi } from './developerWebhookClientApi'
import { developerAppManifestClientApi } from './developerAppManifestClientApi'
import { developerAppAdmissionClientApi } from './developerAppAdmissionClientApi'
import { developerProductionCredentialClientApi } from './developerProductionCredentialClientApi'
import { developerProductionReadinessClientApi } from './developerProductionReadinessClientApi'
import type {
  AuthorizationRequest,
  AuthorizationRequestList,
  ConsumerDiscoveryRequest,
  ConsumerDiscoveryResponse,
  ConsumerDataRequest,
  ConsumerDataRequestList,
  ConsumerInvocationReceipt,
  ConsumerInvocationReceiptList,
  ConsumerPreferenceDisclosure,
  ConsumerPreferenceDisclosureList,
  ConsumerPreferenceField,
  ConsumerPreferenceProfile,
  ConsumerPreferenceProfileEnvelope,
  ConsumerPortabilityExport,
  ConsumerPortabilityExportList,
  ConsumerPortabilityImport,
  ConsumerPortabilityImportList,
  ConsumerPortabilityImportSummary,
  ConsumerPortabilityPackageSignature,
  ConsumerPortabilityTrustKey,
  ConsumerPortabilityTrustKeyList,
  ConsumerPortabilityAdoption,
  ConsumerPortabilityAdoptionList,
  ConsumerPortabilityAdoptionPlan,
  PortabilityReauthorizationResult,
  PortabilityRelationshipMapping,
  PortabilityRelationshipMappingList,
  ConsumerRelationship,
  ConsumerRelationshipList,
  DeveloperAppCredential,
  DeveloperAppList,
  DeveloperInvokeRequest,
  DeveloperTerminalEventDetail,
  DeveloperTerminalEventPage,
  DeleteConsumerPreferenceDisclosureResult,
  DeleteConsumerPreferenceProfileResult,
  DirectoryMerchantList,
  MerchantIdentityKey,
  MerchantIdentityKeyList,
  OpenCommerceDeveloperApp,
} from './openCommerceClientTypes'
import type {
  InvokeOpenCommerceCapability,
  OpenCommerceActionConfirmation,
} from './openCommerceTypes'

function projectBase(projectId: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/open-commerce`
}

export const openCommerceClientApi = {
  ...developerAppManifestClientApi,
  ...developerAppAdmissionClientApi,
  ...developerProductionCredentialClientApi,
  ...developerProductionReadinessClientApi,
  ...developerWebhookClientApi,
  searchDirectoryMerchants: (query: string, limit = 20) => {
    const params = new URLSearchParams({ limit: String(limit) })
    if (query.trim()) params.set('query', query.trim())
    return api.get<DirectoryMerchantList>(`/api/open-commerce/merchants?${params.toString()}`)
  },

  listMerchantIdentityKeys: (projectId: string, merchantId: string) =>
    api.get<MerchantIdentityKeyList>(
      `${projectBase(projectId)}/merchants/${encodeURIComponent(merchantId)}/identity-keys`,
    ),

  createMerchantIdentityKey: (
    projectId: string,
    merchantId: string,
    request: { public_key_pem: string; proof_signature_base64: string },
  ) => api.post<MerchantIdentityKey>(
    `${projectBase(projectId)}/merchants/${encodeURIComponent(merchantId)}/identity-keys`,
    request,
  ),

  revokeMerchantIdentityKey: (projectId: string, merchantId: string, recordId: string) =>
    api.post<MerchantIdentityKey>(
      `${projectBase(projectId)}/merchants/${encodeURIComponent(merchantId)}/identity-keys/${encodeURIComponent(recordId)}/revoke`,
      {},
    ),

  listApps: (projectId: string) =>
    api.get<DeveloperAppList>(`${projectBase(projectId)}/developer-apps`),

  createApp: (projectId: string, request: { app_id: string; display_name: string }) =>
    api.post<DeveloperAppCredential>(`${projectBase(projectId)}/developer-apps`, request),

  rotateToken: (projectId: string, appRecordId: string) =>
    api.post<DeveloperAppCredential>(
      `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/rotate-token`,
      {},
    ),

  disableApp: (projectId: string, appRecordId: string) =>
    api.post<OpenCommerceDeveloperApp>(
      `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/disable`,
      {},
    ),

  reactivateApp: (projectId: string, appRecordId: string) =>
    api.post<DeveloperAppCredential>(
      `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/reactivate`,
      {},
    ),

  listAuthorizationRequests: (projectId: string) =>
    api.get<AuthorizationRequestList>(`${projectBase(projectId)}/authorization-requests`),

  listOutboundAuthorizationRequests: (projectId: string) =>
    api.get<AuthorizationRequestList>(`${projectBase(projectId)}/outbound-authorization-requests`),

  cancelOutboundAuthorization: (projectId: string, requestId: string) =>
    api.post<AuthorizationRequest>(
      `${projectBase(projectId)}/outbound-authorization-requests/${encodeURIComponent(requestId)}/cancel`,
      {},
    ),

  listConsumerRelationships: (projectId: string) =>
    api.get<ConsumerRelationshipList>(`${projectBase(projectId)}/consumer-relationships`),

  getConsumerPreferenceProfile: (projectId: string) =>
    api.get<ConsumerPreferenceProfileEnvelope>(
      `${projectBase(projectId)}/consumer-preference-profile`,
    ),

  upsertConsumerPreferenceProfile: (
    projectId: string,
    preferences: ConsumerPreferenceProfile['preferences'],
  ) => api.put<ConsumerPreferenceProfile>(
    `${projectBase(projectId)}/consumer-preference-profile`,
    { preferences },
  ),

  deleteConsumerPreferenceProfile: (projectId: string) =>
    api.delete<DeleteConsumerPreferenceProfileResult>(
      `${projectBase(projectId)}/consumer-preference-profile`,
    ),

  listConsumerPreferenceDisclosures: (projectId: string) =>
    api.get<ConsumerPreferenceDisclosureList>(
      `${projectBase(projectId)}/consumer-preference-disclosures`,
    ),

  upsertConsumerPreferenceDisclosure: (
    projectId: string,
    relationshipId: string,
    sharedFields: ConsumerPreferenceField[],
  ) => api.put<ConsumerPreferenceDisclosure>(
    `${projectBase(projectId)}/consumer-relationships/${encodeURIComponent(relationshipId)}/preference-disclosure`,
    { shared_fields: sharedFields },
  ),

  deleteConsumerPreferenceDisclosure: (projectId: string, relationshipId: string) =>
    api.delete<DeleteConsumerPreferenceDisclosureResult>(
      `${projectBase(projectId)}/consumer-relationships/${encodeURIComponent(relationshipId)}/preference-disclosure`,
    ),

  listMerchantPreferenceDisclosures: (projectId: string, merchantId: string) =>
    api.get<ConsumerPreferenceDisclosureList>(
      `${projectBase(projectId)}/merchants/${encodeURIComponent(merchantId)}/preference-disclosures`,
    ),

  createConsumerRelationship: (projectId: string, request: {
    merchant_id: string
    source_app_id: string
    scopes: string[]
    purpose: string
    expires_at: string
  }) => api.post<ConsumerRelationship>(`${projectBase(projectId)}/consumer-relationships`, request),

  revokeConsumerRelationship: (projectId: string, relationshipId: string) =>
    api.post<ConsumerRelationship>(
      `${projectBase(projectId)}/consumer-relationships/${encodeURIComponent(relationshipId)}/revoke`,
      {},
    ),

  renewConsumerRelationship: (
    projectId: string,
    relationshipId: string,
    request: { source_app_id: string; expires_at: string },
  ) => api.post<ConsumerRelationship>(
    `${projectBase(projectId)}/consumer-relationships/${encodeURIComponent(relationshipId)}/renew`,
    request,
  ),

  listMerchantRelationships: (projectId: string, merchantId: string) =>
    api.get<ConsumerRelationshipList>(
      `${projectBase(projectId)}/merchants/${encodeURIComponent(merchantId)}/consumer-relationships`,
    ),

  listConsumerDataRequests: (projectId: string) =>
    api.get<ConsumerDataRequestList>(`${projectBase(projectId)}/consumer-data-requests`),

  listConsumerPortabilityExports: (projectId: string) =>
    api.get<ConsumerPortabilityExportList>(`${projectBase(projectId)}/consumer-portability-exports`),

  createConsumerPortabilityExport: (projectId: string, idempotencyKey: string) =>
    api.post<ConsumerPortabilityExport>(`${projectBase(projectId)}/consumer-portability-exports`, {
      idempotency_key: idempotencyKey,
    }),

  getConsumerPortabilityExport: (projectId: string, exportId: string) =>
    api.get<ConsumerPortabilityExport>(
      `${projectBase(projectId)}/consumer-portability-exports/${encodeURIComponent(exportId)}`,
    ),

  listConsumerPortabilityImports: (projectId: string) =>
    api.get<ConsumerPortabilityImportList>(`${projectBase(projectId)}/consumer-portability-imports`),

  createConsumerPortabilityImport: (
    projectId: string,
    sourceOperator: string,
    portabilityPackage: ConsumerPortabilityExport,
    signature?: ConsumerPortabilityPackageSignature,
  ) => api.post<ConsumerPortabilityImport>(
    `${projectBase(projectId)}/consumer-portability-imports`,
    { source_operator: sourceOperator, package: portabilityPackage, signature },
  ),

  getConsumerPortabilityImport: (projectId: string, importId: string) =>
    api.get<ConsumerPortabilityImport>(
      `${projectBase(projectId)}/consumer-portability-imports/${encodeURIComponent(importId)}`,
    ),

  deleteConsumerPortabilityImport: (projectId: string, importId: string) =>
    api.delete<ConsumerPortabilityImportSummary>(
      `${projectBase(projectId)}/consumer-portability-imports/${encodeURIComponent(importId)}`,
    ),

  listConsumerPortabilityTrustKeys: (projectId: string) =>
    api.get<ConsumerPortabilityTrustKeyList>(
      `${projectBase(projectId)}/consumer-portability-trust-keys`,
    ),

  createConsumerPortabilityTrustKey: (
    projectId: string,
    sourceOperator: string,
    publicKeyPem: string,
  ) => api.post<ConsumerPortabilityTrustKey>(
    `${projectBase(projectId)}/consumer-portability-trust-keys`,
    { source_operator: sourceOperator, public_key_pem: publicKeyPem },
  ),

  revokeConsumerPortabilityTrustKey: (projectId: string, recordId: string) =>
    api.post<ConsumerPortabilityTrustKey>(
      `${projectBase(projectId)}/consumer-portability-trust-keys/${encodeURIComponent(recordId)}/revoke`,
      {},
    ),

  getConsumerPortabilityAdoptionPlan: (projectId: string, importId: string) =>
    api.get<ConsumerPortabilityAdoptionPlan>(
      `${projectBase(projectId)}/consumer-portability-imports/${encodeURIComponent(importId)}/adoption-plan`,
    ),

  applyConsumerPortabilityPreferences: (
    projectId: string,
    importId: string,
    expectedCurrentRevision?: number,
    selectedFields: string[] = [],
  ) => api.post<ConsumerPortabilityAdoption>(
    `${projectBase(projectId)}/consumer-portability-imports/${encodeURIComponent(importId)}/adopt-preferences`,
    {
      expected_current_revision: expectedCurrentRevision,
      selected_fields: selectedFields,
      confirmed_by_user: true,
    },
  ),

  listConsumerPortabilityAdoptions: (projectId: string) =>
    api.get<ConsumerPortabilityAdoptionList>(
      `${projectBase(projectId)}/consumer-portability-adoptions`,
    ),

  rollbackConsumerPortabilityAdoption: (
    projectId: string,
    adoptionId: string,
    expectedCurrentRevision: number,
  ) => api.post<ConsumerPortabilityAdoption>(
    `${projectBase(projectId)}/consumer-portability-adoptions/${encodeURIComponent(adoptionId)}/rollback`,
    { expected_current_revision: expectedCurrentRevision, confirmed_by_user: true },
  ),

  listPortabilityRelationshipMappings: (projectId: string) =>
    api.get<PortabilityRelationshipMappingList>(
      `${projectBase(projectId)}/portability-relationship-mappings`,
    ),

  createPortabilityRelationshipMapping: (
    projectId: string,
    request: { import_id: string; source_relationship_id: string; target_merchant_id: string },
  ) => api.post<PortabilityRelationshipMapping>(
    `${projectBase(projectId)}/portability-relationship-mappings`,
    { ...request, confirmed_by_user: true },
  ),

  revokePortabilityRelationshipMapping: (projectId: string, mappingId: string) =>
    api.post<PortabilityRelationshipMapping>(
      `${projectBase(projectId)}/portability-relationship-mappings/${encodeURIComponent(mappingId)}/revoke`,
      {},
    ),

  createPortabilityReauthorization: (
    projectId: string,
    mappingId: string,
    request: { requester_app_id: string; scopes: string[]; purpose: string },
  ) => api.post<PortabilityReauthorizationResult>(
    `${projectBase(projectId)}/portability-relationship-mappings/${encodeURIComponent(mappingId)}/reauthorize`,
    { ...request, confirmed_by_user: true },
  ),

  listConsumerInvocationReceipts: () =>
    api.get<ConsumerInvocationReceiptList>(
      '/api/open-commerce/consumer-invocation-receipts?limit=100',
    ),

  getConsumerInvocationReceipt: (invocationId: string) =>
    api.get<ConsumerInvocationReceipt>(
      `/api/open-commerce/consumer-invocation-receipts/${encodeURIComponent(invocationId)}`,
    ),

  createConsumerDataErasureRequest: (projectId: string, relationshipId: string) =>
    api.post<ConsumerDataRequest>(`${projectBase(projectId)}/consumer-data-requests`, {
      relationship_id: relationshipId,
    }),

  withdrawConsumerDataRequest: (projectId: string, requestId: string) =>
    api.post<ConsumerDataRequest>(
      `${projectBase(projectId)}/consumer-data-requests/${encodeURIComponent(requestId)}/withdraw`,
      {},
    ),

  listMerchantDataRequests: (projectId: string, merchantId: string) =>
    api.get<ConsumerDataRequestList>(
      `${projectBase(projectId)}/merchants/${encodeURIComponent(merchantId)}/consumer-data-requests`,
    ),

  decideConsumerDataRequest: (
    projectId: string,
    merchantId: string,
    requestId: string,
    request: { action: 'accept' | 'complete' | 'reject'; note: string },
  ) => api.post<ConsumerDataRequest>(
    `${projectBase(projectId)}/merchants/${encodeURIComponent(merchantId)}/consumer-data-requests/${encodeURIComponent(requestId)}/decision`,
    request,
  ),

  decideAuthorization: (
    projectId: string,
    requestId: string,
    decision: 'approve' | 'reject',
    request: {
      reason: string
      expires_at?: string
      max_invocations?: number
      max_amount_micros?: number
      budget_currency?: string
    },
  ) =>
    api.post<AuthorizationRequest>(
      `${projectBase(projectId)}/authorization-requests/${encodeURIComponent(requestId)}/${decision}`,
      request,
    ),

  discover: (request: ConsumerDiscoveryRequest) =>
    api.post<ConsumerDiscoveryResponse>('/api/open-commerce/sandbox/discover', request),

  requestAuthorization: (request: {
    merchant_id: string
    requester_app_id: string
    scopes: string[]
    purpose: string
  }) => api.post<AuthorizationRequest>('/api/open-commerce/authorization-requests', request),

  invokeAsApp: (appId: string, request: InvokeOpenCommerceCapability) =>
    api.postWithHeaders<Record<string, unknown>>('/api/open-commerce/invoke', request, {
      'x-elon-app-id': appId,
    }),

  prepareActionConfirmation: (appId: string, request: InvokeOpenCommerceCapability) =>
    api.postWithHeaders<OpenCommerceActionConfirmation>(
      '/api/open-commerce/action-confirmations',
      request,
      { 'x-elon-app-id': appId },
    ),

  confirmActionConfirmation: (appId: string, confirmationId: string) =>
    api.postWithHeaders<OpenCommerceActionConfirmation>(
      `/api/open-commerce/action-confirmations/${encodeURIComponent(confirmationId)}/confirm`,
      { confirmation_phrase: 'CONFIRM_ACTION' },
      { 'x-elon-app-id': appId },
    ),

  developerPrepareActionConfirmation: (testToken: string, request: DeveloperInvokeRequest) =>
    api.postWithHeaders<OpenCommerceActionConfirmation>(
      '/api/open-commerce/developer/action-confirmations',
      request,
      { Authorization: `Bearer ${testToken}` },
    ),

  developerConfirmActionConfirmation: (testToken: string, confirmationId: string) =>
    api.postWithHeaders<OpenCommerceActionConfirmation>(
      `/api/open-commerce/developer/action-confirmations/${encodeURIComponent(confirmationId)}/confirm`,
      { confirmation_phrase: 'CONFIRM_ACTION' },
      { Authorization: `Bearer ${testToken}` },
    ),

  developerInvoke: (testToken: string, request: DeveloperInvokeRequest) =>
    api.postWithHeaders<Record<string, unknown>>('/api/open-commerce/developer/invoke', request, {
      Authorization: `Bearer ${testToken}`,
    }),

  listDeveloperTerminalEvents: (testToken: string, cursor?: string) => {
    const query = new URLSearchParams({ limit: '20' })
    if (cursor) query.set('cursor', cursor)
    return api.getWithHeaders<DeveloperTerminalEventPage>(
      `/api/open-commerce/developer/events?${query.toString()}`,
      { Authorization: `Bearer ${testToken}` },
    )
  },

  getDeveloperTerminalEvent: (testToken: string, invocationId: string) =>
    api.getWithHeaders<DeveloperTerminalEventDetail>(
      `/api/open-commerce/developer/events/${encodeURIComponent(invocationId)}`,
      { Authorization: `Bearer ${testToken}` },
    ),
}
