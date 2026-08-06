import { api } from '../../api/client'

export interface AccountSession {
  id: string
  device_name?: string | null
  apk_version?: string | null
  trusted_device: boolean
  current: boolean
  created_at: string
  last_seen_at?: string | null
  expires_at: string
}

export interface AccountSecuritySnapshot {
  schema_version: number
  password: {
    enabled: boolean
    changed_at?: string | null
    can_set: boolean
    can_change: boolean
  }
  recovery: {
    mode: string
    available_code_count: number
    external_delivery_configured: boolean
    external_delivery_state: string
  }
  sessions: AccountSession[]
}

interface RecoveryCodeRotation {
  ok: boolean
  one_time_display: boolean
  result: {
    batch_id: string
    codes: string[]
    replayed: boolean
  }
}

export const accountSecurityApi = {
  status: () => api.get<AccountSecuritySnapshot>('/api/auth/security'),
  changePassword: (currentPassword: string, newPassword: string) =>
    api.put('/api/auth/password', {
      current_password: currentPassword || null,
      new_password: newPassword,
      request_id: requestId('password'),
      confirm: true,
    }),
  rotateRecoveryCodes: (currentPassword: string) =>
    api.post<RecoveryCodeRotation>('/api/auth/recovery-codes/rotate', {
      current_password: currentPassword || null,
      request_id: requestId('recovery-codes'),
      confirm: true,
    }),
  revokeSession: (sessionId: string) =>
    api.delete<{ result: { current_session: boolean } }>(
      `/api/auth/sessions/${encodeURIComponent(sessionId)}`,
    ),
  revokeOtherSessions: () =>
    api.post<{ revoked_session_count: number }>('/api/auth/sessions/revoke-others', {
      confirm: true,
    }),
  recoverPassword: (account: string, recoveryCode: string, newPassword: string) =>
    api.post('/api/auth/password/recover', {
      account,
      recovery_code: recoveryCode,
      new_password: newPassword,
      request_id: requestId('password-recover'),
      client_instance_id: clientInstanceId(),
      confirm: true,
    }),
  startExternalRecovery: (account: string) =>
    api.post<{ delivery_configured: boolean; message: string }>(
      '/api/auth/password/recovery/start',
      { account, client_instance_id: clientInstanceId() },
    ),
}

function requestId(action: string) {
  return `pc:${action}:${crypto.randomUUID()}`
}

function clientInstanceId() {
  const key = 'elon_auth_client_instance_id'
  const existing = localStorage.getItem(key)
  if (existing) return existing
  const created = `pc:${crypto.randomUUID()}`
  localStorage.setItem(key, created)
  return created
}
