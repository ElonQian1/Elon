import type { OpenCommerceDeveloperApp } from './openCommerceClientTypes'

export type DeveloperAppAdmissionStatus =
  | 'submitted'
  | 'changes_requested'
  | 'approved'
  | 'suspended'

export type DeveloperAppAdmissionRiskTier = 'low' | 'standard' | 'enhanced'

export interface DeveloperAppAdmission {
  schema: 'open_commerce.developer_app_admission.v1'
  id: string
  app_record_id: string
  project_id: string
  manifest_revision: number
  organization_name: string
  jurisdiction: string
  registration_id: string
  attested_at: string
  status: DeveloperAppAdmissionStatus
  requested_at: string
  reviewed_at: string | null
  reviewed_by_user_id: string | null
  review_note: string | null
  risk_tier: DeveloperAppAdmissionRiskTier | null
  suspended_at: string | null
  production_credential_issued: boolean
  network_access_enabled: boolean
  created_at: string
  updated_at: string
}

export interface DeveloperAppAdmissionState {
  schema: 'open_commerce.developer_app_admission_state.v1'
  admission: DeveloperAppAdmission | null
  production_credential_issued: boolean
  network_access_enabled: boolean
}

export interface DeveloperAppAdmissionReviewItem {
  app: OpenCommerceDeveloperApp
  admission: DeveloperAppAdmission
}

export interface DeveloperAppAdmissionReviewQueue {
  schema: 'open_commerce.developer_app_admission_review_queue.v1'
  items: DeveloperAppAdmissionReviewItem[]
  production_credentials_enabled: boolean
}
