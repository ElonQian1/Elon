import { api } from '../../api/client'
import type {
  DeveloperAppManifestReviewQueue,
  OpenCommerceDeveloperApp,
} from './openCommerceClientTypes'

function projectBase(projectId: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/open-commerce`
}

export const developerAppManifestClientApi = {
  updateDeveloperAppManifest: (
    projectId: string,
    appRecordId: string,
    request: {
      expected_manifest_revision: number
      homepage_url: string | null
      privacy_policy_url: string | null
      terms_url: string | null
      support_email: string | null
      requested_scopes: string[]
    },
  ) => api.post<OpenCommerceDeveloperApp>(
    `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/manifest`,
    request,
  ),

  submitDeveloperAppManifest: (
    projectId: string,
    appRecordId: string,
    expectedManifestRevision: number,
  ) => api.post<OpenCommerceDeveloperApp>(
    `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/manifest/submit`,
    { expected_manifest_revision: expectedManifestRevision },
  ),

  listSubmittedDeveloperAppManifests: () =>
    api.get<DeveloperAppManifestReviewQueue>(
      '/api/admin/open-commerce/developer-app-manifests',
    ),

  reviewDeveloperAppManifest: (
    appRecordId: string,
    request: {
      expected_manifest_revision: number
      decision: 'approved' | 'changes_requested'
      note: string
    },
  ) => api.post<OpenCommerceDeveloperApp>(
    `/api/admin/open-commerce/developer-app-manifests/${encodeURIComponent(appRecordId)}/review`,
    request,
  ),
}
