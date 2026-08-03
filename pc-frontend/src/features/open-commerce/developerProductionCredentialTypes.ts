export type DeveloperProductionCredentialStatus = 'active' | 'revoked'

export interface DeveloperProductionCredential {
  schema: 'open_commerce.developer_production_credential.v1'
  id: string
  app_record_id: string
  project_id: string
  admission_id: string
  manifest_revision: number
  environment: 'production'
  scopes: string[]
  status: DeveloperProductionCredentialStatus
  token_hint: string
  issued_by_user_id: string
  issued_at: string
  expires_at: string
  last_used_at: string | null
  revoked_at: string | null
  revocation_reason: string | null
  created_at: string
  updated_at: string
}

export interface DeveloperProductionCredentialSecret {
  schema: 'open_commerce.developer_production_credential_secret.v1'
  credential: DeveloperProductionCredential
  live_token: string
  token_visible_once: true
  funds_moved: false
}

export interface DeveloperProductionCredentialList {
  schema: 'open_commerce.developer_production_credentials.v1'
  credentials: DeveloperProductionCredential[]
  issuance_enabled: boolean
  funds_moved: false
}
