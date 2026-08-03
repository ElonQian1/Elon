import { api } from '../../api/client'
import type {
  CreateOpenCommerceCapability,
  BlockOpenCommerceApp,
  CreateOpenCommerceGrant,
  CreateOpenCommerceIntegration,
  CreateOpenCommerceMerchant,
  InvokeOpenCommerceCapability,
  OpenCommerceActionConfirmation,
  OpenCommerceCapability,
  OpenCommerceCapabilitySourceLink,
  OpenCommerceAppBlock,
  OpenCommerceAppBlockOutcome,
  OpenCommerceGrant,
  OpenCommerceDirectoryPublication,
  OpenCommerceMerchant,
  OpenCommerceOverview,
  OpenCommerceIntegration,
  OpenCommerceAdapterCredential,
  OpenCommerceAdapterCredentialIssue,
  OpenCommerceAdapterCredentialList,
  OpenCommerceAdapterHandoffClaimList,
  OpenCommerceAdapterHandoffClaimResume,
  MerchantBusinessEvidenceDetail,
  MerchantBusinessEvidenceList,
  OpenCommerceBusinessHandoffReceipt,
  OpenCommerceBusinessHandoffReceiptList,
  OpenCommerceBusinessHandoffQueue,
  OpenCommerceBusinessHandoffQueueState,
  OpenCommerceRuntimeBinding,
  OpenCommerceRateLimitPolicy,
  UpsertOpenCommerceRateLimit,
  UpsertOpenCommerceRuntimeBinding,
  RecordOpenCommerceBusinessHandoffReceipt,
} from './openCommerceTypes'

function projectBase(projectId: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/open-commerce`
}

export const openCommerceApi = {
  overview: (projectId: string) =>
    api.get<OpenCommerceOverview>(`${projectBase(projectId)}/overview`),

  createMerchant: (projectId: string, request: CreateOpenCommerceMerchant) =>
    api.post<OpenCommerceMerchant>(`${projectBase(projectId)}/merchants`, request),

  createCapability: (
    projectId: string,
    merchantId: string,
    request: CreateOpenCommerceCapability,
  ) =>
    api.post<OpenCommerceCapability>(
      `${projectBase(projectId)}/merchants/${encodeURIComponent(merchantId)}/capabilities`,
      request,
    ),

  linkCapabilitySource: (
    projectId: string,
    capabilityId: string,
    request: { integration_id: string; sync_receipt_id: string; data_domain: string },
  ) =>
    api.put<OpenCommerceCapabilitySourceLink>(
      `${projectBase(projectId)}/capabilities/${encodeURIComponent(capabilityId)}/source-link`,
      request,
    ),

  removeCapabilitySource: (projectId: string, capabilityId: string) =>
    api.delete<{ schema: string; capability_id: string; removed: boolean }>(
      `${projectBase(projectId)}/capabilities/${encodeURIComponent(capabilityId)}/source-link`,
    ),

  setDirectoryPublication: (projectId: string, merchantId: string, published: boolean) =>
    api.put<OpenCommerceDirectoryPublication>(
      `${projectBase(projectId)}/merchants/${encodeURIComponent(merchantId)}/directory-publication`,
      { published },
    ),

  createGrant: (projectId: string, request: CreateOpenCommerceGrant) =>
    api.post<OpenCommerceGrant>(`${projectBase(projectId)}/grants`, request),

  revokeGrant: (projectId: string, grantId: string) =>
    api.post<OpenCommerceGrant>(
      `${projectBase(projectId)}/grants/${encodeURIComponent(grantId)}/revoke`,
      {},
    ),

  invoke: (request: InvokeOpenCommerceCapability) =>
    api.post<Record<string, unknown>>('/api/open-commerce/invoke', request),

  prepareActionConfirmation: (request: InvokeOpenCommerceCapability) =>
    api.post<OpenCommerceActionConfirmation>('/api/open-commerce/action-confirmations', request),

  confirmActionConfirmation: (confirmationId: string) =>
    api.post<OpenCommerceActionConfirmation>(
      `/api/open-commerce/action-confirmations/${encodeURIComponent(confirmationId)}/confirm`,
      { confirmation_phrase: 'CONFIRM_ACTION' },
    ),

  createIntegration: (projectId: string, request: CreateOpenCommerceIntegration) =>
    api.post<OpenCommerceIntegration>(`${projectBase(projectId)}/integrations`, request),

  setIntegrationEnabled: (projectId: string, integrationId: string, enabled: boolean) =>
    api.patch<OpenCommerceIntegration>(
      `${projectBase(projectId)}/integrations/${encodeURIComponent(integrationId)}/enabled`,
      { enabled },
    ),

  listAdapterCredentials: (projectId: string) =>
    api.get<OpenCommerceAdapterCredentialList>(
      `${projectBase(projectId)}/adapter-credentials`,
    ),

  listAdapterHandoffClaims: (projectId: string, limit = 50) =>
    api.get<OpenCommerceAdapterHandoffClaimList>(
      `${projectBase(projectId)}/adapter-handoff-claims?limit=${limit}`,
    ),

  resumeAdapterHandoffClaim: (projectId: string, claimId: string) =>
    api.post<OpenCommerceAdapterHandoffClaimResume>(
      `${projectBase(projectId)}/adapter-handoff-claims/${encodeURIComponent(claimId)}/resume`,
      { confirmed_by_user: true },
    ),

  rotateAdapterCredential: (
    projectId: string,
    integrationId: string,
    expiresInDays: number,
    allowTaskClaims: boolean,
  ) =>
    api.post<OpenCommerceAdapterCredentialIssue>(
      `${projectBase(projectId)}/integrations/${encodeURIComponent(integrationId)}/adapter-credential/rotate`,
      {
        confirmed_by_user: true,
        expires_in_days: expiresInDays,
        allow_task_claims: allowTaskClaims,
      },
    ),

  revokeAdapterCredential: (projectId: string, credentialId: string) =>
    api.post<OpenCommerceAdapterCredential>(
      `${projectBase(projectId)}/adapter-credentials/${encodeURIComponent(credentialId)}/revoke`,
      { confirmed_by_user: true },
    ),

  upsertRuntime: (
    projectId: string,
    merchantId: string,
    request: UpsertOpenCommerceRuntimeBinding,
  ) =>
    api.put<OpenCommerceRuntimeBinding>(
      `${projectBase(projectId)}/merchants/${encodeURIComponent(merchantId)}/runtime`,
      request,
    ),

  verifyRuntime: (projectId: string, merchantId: string) =>
    api.post<OpenCommerceRuntimeBinding>(
      `${projectBase(projectId)}/merchants/${encodeURIComponent(merchantId)}/runtime/verify`,
      {},
    ),

  listMerchantBusinessEvidence: (projectId: string, merchantId: string) =>
    api.get<MerchantBusinessEvidenceList>(
      `${projectBase(projectId)}/merchants/${encodeURIComponent(merchantId)}/business-evidence`,
    ),

  getMerchantBusinessEvidence: (
    projectId: string,
    merchantId: string,
    invocationId: string,
  ) =>
    api.get<MerchantBusinessEvidenceDetail>(
      `${projectBase(projectId)}/merchants/${encodeURIComponent(merchantId)}/business-evidence/${encodeURIComponent(invocationId)}`,
    ),

  listBusinessHandoffReceipts: (projectId: string, merchantId: string) =>
    api.get<OpenCommerceBusinessHandoffReceiptList>(
      `${projectBase(projectId)}/merchants/${encodeURIComponent(merchantId)}/business-handoff-receipts`,
    ),

  listBusinessHandoffQueue: (
    projectId: string,
    merchantId: string,
    state?: OpenCommerceBusinessHandoffQueueState,
  ) => {
    const query = new URLSearchParams({ limit: '100' })
    if (state) query.set('state', state)
    return api.get<OpenCommerceBusinessHandoffQueue>(
      `${projectBase(projectId)}/merchants/${encodeURIComponent(merchantId)}/business-handoff-queue?${query}`,
    )
  },

  recordBusinessHandoffReceipt: (
    projectId: string,
    request: RecordOpenCommerceBusinessHandoffReceipt,
  ) =>
    api.post<OpenCommerceBusinessHandoffReceipt>(
      `${projectBase(projectId)}/business-handoff-receipts`,
      request,
    ),

  upsertRateLimit: (projectId: string, request: UpsertOpenCommerceRateLimit) =>
    api.put<OpenCommerceRateLimitPolicy>(`${projectBase(projectId)}/rate-limits`, request),

  setRateLimitEnabled: (projectId: string, policyId: string, enabled: boolean) =>
    api.patch<OpenCommerceRateLimitPolicy>(
      `${projectBase(projectId)}/rate-limits/${encodeURIComponent(policyId)}/enabled`,
      { enabled },
    ),

  listAppBlocks: (projectId: string) =>
    api.get<OpenCommerceAppBlock[]>(`${projectBase(projectId)}/app-blocks`),

  blockApp: (projectId: string, request: BlockOpenCommerceApp) =>
    api.put<OpenCommerceAppBlockOutcome>(`${projectBase(projectId)}/app-blocks`, request),

  unblockApp: (projectId: string, blockId: string) =>
    api.post<OpenCommerceAppBlockOutcome>(
      `${projectBase(projectId)}/app-blocks/${encodeURIComponent(blockId)}/unblock`,
      {},
    ),

}

