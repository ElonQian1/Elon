import { api } from '../../api/client'
import type {
  CreateOpenCommerceCapability,
  CreateOpenCommerceGrant,
  CreateOpenCommerceIntegration,
  CreateOpenCommerceMerchant,
  InvokeOpenCommerceCapability,
  OpenCommerceCapability,
  OpenCommerceGrant,
  OpenCommerceDirectoryPublication,
  OpenCommerceMerchant,
  OpenCommerceOverview,
  OpenCommerceIntegration,
  OpenCommerceRuntimeBinding,
  OpenCommerceRateLimitPolicy,
  UpsertOpenCommerceRateLimit,
  UpsertOpenCommerceRuntimeBinding,
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

  createIntegration: (projectId: string, request: CreateOpenCommerceIntegration) =>
    api.post<OpenCommerceIntegration>(`${projectBase(projectId)}/integrations`, request),

  setIntegrationEnabled: (projectId: string, integrationId: string, enabled: boolean) =>
    api.patch<OpenCommerceIntegration>(
      `${projectBase(projectId)}/integrations/${encodeURIComponent(integrationId)}/enabled`,
      { enabled },
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

  upsertRateLimit: (projectId: string, request: UpsertOpenCommerceRateLimit) =>
    api.put<OpenCommerceRateLimitPolicy>(`${projectBase(projectId)}/rate-limits`, request),

  setRateLimitEnabled: (projectId: string, policyId: string, enabled: boolean) =>
    api.patch<OpenCommerceRateLimitPolicy>(
      `${projectBase(projectId)}/rate-limits/${encodeURIComponent(policyId)}/enabled`,
      { enabled },
    ),

}

