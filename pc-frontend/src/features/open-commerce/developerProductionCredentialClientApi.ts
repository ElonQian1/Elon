import { api } from '../../api/client'
import type {
  DeveloperProductionCredential,
  DeveloperProductionCredentialList,
  DeveloperProductionCredentialSecret,
} from './developerProductionCredentialTypes'

function projectBase(projectId: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/open-commerce`
}

export const developerProductionCredentialClientApi = {
  listDeveloperProductionCredentials: (projectId: string, appRecordId: string) =>
    api.get<DeveloperProductionCredentialList>(
      `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/production-credentials`,
    ),

  issueDeveloperProductionCredential: (
    appRecordId: string,
    request: {
      expected_manifest_revision: number
      scopes: string[]
      expires_in_days: number
    },
  ) => api.post<DeveloperProductionCredentialSecret>(
    `/api/admin/open-commerce/developer-apps/${encodeURIComponent(appRecordId)}/production-credentials/issue`,
    request,
  ),

  revokeDeveloperProductionCredential: (
    projectId: string,
    appRecordId: string,
    credentialId: string,
    reason: string,
  ) => api.post<DeveloperProductionCredential>(
    `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/production-credentials/${encodeURIComponent(credentialId)}/revoke`,
    { reason },
  ),
}
