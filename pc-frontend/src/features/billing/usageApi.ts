import { api } from '../../api/client'

export interface UserUsageQuota {
  limit_tokens: number | null
  used_tokens: number
  remaining_tokens: number | null
  is_blocked: boolean
  block_reason?: string | null
  reset_at?: string
}

export interface UserUsageStats {
  user_id: string
  period_days: number
  total: {
    total_tokens: number
  }
  quota: UserUsageQuota
}

export function fetchMyUsageStats(userId: string, days = 7) {
  return api.get<UserUsageStats>(`/api/user/${encodeURIComponent(userId)}/usage/stats?days=${days}`)
}
