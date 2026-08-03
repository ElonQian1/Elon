import { api } from '../../api/client'
import type {
  DeveloperAppAdmission,
  DeveloperAppAdmissionReviewQueue,
  DeveloperAppAdmissionRiskTier,
  DeveloperAppAdmissionState,
} from './developerAppAdmissionTypes'

function projectBase(projectId: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/open-commerce`
}

export const developerAppAdmissionClientApi = {
  getDeveloperAppAdmission: (projectId: string, appRecordId: string) =>
    api.get<DeveloperAppAdmissionState>(
      `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/admission`,
    ),

  submitDeveloperAppAdmission: (
    projectId: string,
    appRecordId: string,
    request: {
      expected_manifest_revision: number
      organization_name: string
      jurisdiction: string
      registration_id: string
      information_attested: boolean
    },
  ) => api.post<DeveloperAppAdmission>(
    `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/admission/submit`,
    request,
  ),

  listReviewableDeveloperAppAdmissions: () =>
    api.get<DeveloperAppAdmissionReviewQueue>(
      '/api/admin/open-commerce/developer-app-admissions',
    ),

  reviewDeveloperAppAdmission: (
    appRecordId: string,
    request: {
      expected_manifest_revision: number
      decision: 'approved' | 'changes_requested' | 'suspended'
      risk_tier: DeveloperAppAdmissionRiskTier
      note: string
    },
  ) => api.post<DeveloperAppAdmission>(
    `/api/admin/open-commerce/developer-app-admissions/${encodeURIComponent(appRecordId)}/review`,
    request,
  ),
}
