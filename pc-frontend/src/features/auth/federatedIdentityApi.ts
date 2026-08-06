import { api } from '../../api/client'
import type { User } from '../../store/auth'

export interface FederatedProvider {
  id: string
  configured: boolean
  login: boolean
  bind: boolean
  client_id?: string | null
}
export interface IdentityChallenge {
  id: string
  provider: string
  mode: 'login' | 'bind'
  nonce: string
  expires_at: string
}

export interface LinkedIdentity {
  id: string
  provider: string
  email?: string | null
  display_name?: string | null
  avatar_url?: string | null
  created_at: string
  last_login_at?: string | null
}

export interface FederatedCompletion {
  mode: 'login' | 'bind'
  user: User
  identity: LinkedIdentity
  created_user: boolean
  session?: { token: string; expires_at: string } | null
}

export const federatedIdentityApi = {
  providers: () => api.get<{ providers: FederatedProvider[] }>('/api/auth/federation/providers'),
  challenge: (mode: 'login' | 'bind') =>
    api.post<IdentityChallenge>('/api/auth/federation/google/challenges', {
      mode,
      platform: 'windows',
      request_id: authRequestId('challenge'),
      client_instance_id: authClientInstanceId(),
    }),
  complete: (challengeId: string, idToken: string) =>
    api.post<FederatedCompletion>('/api/auth/federation/google/complete', {
      challenge_id: challengeId,
      id_token: idToken,
      remember_device: true,
      device_name: 'PC Web',
      request_id: authRequestId('complete'),
      client_instance_id: authClientInstanceId(),
    }),
  identities: () => api.get<{ identities: LinkedIdentity[] }>('/api/auth/identities'),
  unlink: (identityId: string) =>
    api.delete<void>(`/api/auth/identities/${encodeURIComponent(identityId)}`),
}

function authRequestId(action: string) {
  return `pc:${action}:${crypto.randomUUID()}`
}

function authClientInstanceId() {
  const key = 'elon_auth_client_instance_id'
  const existing = localStorage.getItem(key)
  if (existing) return existing
  const created = `pc:${crypto.randomUUID()}`
  localStorage.setItem(key, created)
  return created
}
