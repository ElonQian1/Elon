import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { api } from '../api/client'
import { nodeApi } from '../features/node/localNodeApi'
import { safeNodeAdminUrl } from '../lib/utils'

const PWA_FORGET_PENDING_KEY = 'elon_pwa_credential_forget_pending'

async function forgetRememberedPwaCredential(): Promise<void> {
  try {
    localStorage.setItem(PWA_FORGET_PENDING_KEY, '1')
  } catch {
    // 浏览器禁用存储时仍尝试本次撤销。
  }
  await nodeApi(
    safeNodeAdminUrl(),
    '/api/source-preview/pwa-auth-profile/remembered',
    { method: 'DELETE' },
    3000,
  )
  try {
    localStorage.removeItem(PWA_FORGET_PENDING_KEY)
  } catch {
    // 删除已成功；本地 marker 清理失败不影响节点凭据状态。
  }
}

function retryPendingPwaCredentialForget(): void {
  try {
    if (localStorage.getItem(PWA_FORGET_PENDING_KEY) === '1') {
      void forgetRememberedPwaCredential().catch(() => {})
    }
  } catch {
    // 非浏览器环境或存储不可用。
  }
}

export interface User {
  id: string
  account: string
  nickname?: string
  role?: string
  status?: string
  avatar_data_url?: string | null
}

interface AuthState {
  token: string | null
  expiresAt: string | null
  user: User | null
  login: (username: string, password: string) => Promise<void>
  register: (username: string, password: string, nickname?: string) => Promise<void>
  logout: () => void
  fetchMe: () => Promise<void>
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, _get) => ({
      token: null,
      expiresAt: null,
      user: null,

      login: async (username, password) => {
        const res = await api.post<{ token: string; expires_at: string; user: User }>(
          '/api/auth/login',
          { account: username, password, remember_device: true, device_name: 'PC Web' },
        )
        set({ token: res.token, expiresAt: res.expires_at, user: res.user })
      },

      register: async (username, password, nickname) => {
        const res = await api.post<{ token: string; expires_at: string; user: User }>(
          '/api/auth/register',
          {
            account: username,
            password,
            nickname: nickname || undefined,
            remember_device: true,
            device_name: 'PC Web',
          },
        )
        set({ token: res.token, expiresAt: res.expires_at, user: res.user })
      },

      logout: () => {
        set({ token: null, expiresAt: null, user: null })
        void forgetRememberedPwaCredential().catch(() => {
          // 本机节点离线不阻止退出；marker 会让下次打开 PC 网页时继续撤销。
        })
      },

      fetchMe: async () => {
        // /api/me 返回 { "user": {...} } 格式，需要取出内层 user
        const res = await api.get<{ user?: User }>('/api/me')
        const user = res?.user ?? (res as unknown as User)
        if (user?.id) set({ user })
        try {
          const trusted = await api.post<{ expires_at?: string | null }>(
            '/api/auth/trust-current-device',
            {},
          )
          if (trusted.expires_at) set({ expiresAt: trusted.expires_at })
        } catch {
          // 兼容尚未发布该端点的旧服务端；用户仍保持当前有效登录。
        }
      },
    }),
    {
      name: 'elon_auth',
      // 只持久化 token，user 由 fetchMe 刷新
      partialize: (s) => ({ token: s.token, expiresAt: s.expiresAt, user: s.user }),
    },
  ),
)

retryPendingPwaCredentialForget()
