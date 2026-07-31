import { api } from '../../api/client'
import type {
  AuthorizationRequest,
  AuthorizationRequestList,
  ConsumerDiscoveryRequest,
  ConsumerDiscoveryResponse,
  DeveloperAppCredential,
  DeveloperAppList,
  DeveloperInvokeRequest,
} from './openCommerceClientTypes'
import type { InvokeOpenCommerceCapability } from './openCommerceTypes'

function projectBase(projectId: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/open-commerce`
}

export const openCommerceClientApi = {
  listApps: (projectId: string) =>
    api.get<DeveloperAppList>(`${projectBase(projectId)}/developer-apps`),

  createApp: (projectId: string, request: { app_id: string; display_name: string }) =>
    api.post<DeveloperAppCredential>(`${projectBase(projectId)}/developer-apps`, request),

  rotateToken: (projectId: string, appRecordId: string) =>
    api.post<DeveloperAppCredential>(
      `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/rotate-token`,
      {},
    ),

  listAuthorizationRequests: (projectId: string) =>
    api.get<AuthorizationRequestList>(`${projectBase(projectId)}/authorization-requests`),

  decideAuthorization: (
    projectId: string,
    requestId: string,
    decision: 'approve' | 'reject',
    reason: string,
  ) =>
    api.post<AuthorizationRequest>(
      `${projectBase(projectId)}/authorization-requests/${encodeURIComponent(requestId)}/${decision}`,
      { reason },
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

  developerInvoke: (testToken: string, request: DeveloperInvokeRequest) =>
    api.postWithHeaders<Record<string, unknown>>('/api/open-commerce/developer/invoke', request, {
      Authorization: `Bearer ${testToken}`,
    }),
}
