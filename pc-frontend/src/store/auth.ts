import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { api } from '../api/client'

interface User {
  id: string
  username: string
  display_name?: string
}

interface AuthState {
  token: string | null
  user: User | null
  login: (username: string, password: string) => Promise<void>
  logout: () => void
  fetchMe: () => Promise<void>
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, _get) => ({
      token: null,
      user: null,

      login: async (username, password) => {
        const res = await api.post<{ token: string; user: User }>(
          '/api/auth/login',
          { username, password },
        )
        set({ token: res.token, user: res.user })
      },

      logout: () => {
        set({ token: null, user: null })
      },

      fetchMe: async () => {
        const user = await api.get<User>('/api/me')
        set({ user })
      },
    }),
    {
      name: 'elon_auth',
      // 只持久化 token，user 由 fetchMe 刷新
      partialize: (s) => ({ token: s.token, user: s.user }),
    },
  ),
)
