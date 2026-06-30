import { api } from '../../api/client'

export interface UserProgressionSummary {
  user_id: string
  level: number
  tier_name: string
  total_xp_tokens: number
  consumed_tokens: number
  provided_tokens: number
  level_floor_tokens: number
  next_level_tokens: number
  tokens_into_level: number
  tokens_to_next_level: number
  level_progress_ratio: number
  consumed_progress_ratio: number
  provided_progress_ratio: number
  consumed_call_count: number
  provided_run_count: number
  provider_earned_fen: number
}

export function fetchMyProgression() {
  return api.get<UserProgressionSummary>('/api/me/progression')
}
