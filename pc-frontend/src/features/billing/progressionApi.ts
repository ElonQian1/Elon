import { api } from '../../api/client'

export interface UserProgressionSummary {
  user_id: string
  level: number
  tier_name: string
  total_xp_tokens: number
  consumed_tokens: number
  own_codex_tokens?: number
  shared_codex_tokens?: number
  platform_tokens?: number
  provided_tokens: number
  level_floor_tokens: number
  next_level_tokens: number
  tokens_into_level: number
  tokens_to_next_level: number
  level_progress_ratio: number
  consumed_progress_ratio: number
  own_codex_progress_ratio?: number
  shared_codex_progress_ratio?: number
  platform_progress_ratio?: number
  provided_progress_ratio: number
  consumed_call_count: number
  own_codex_call_count?: number
  shared_codex_call_count?: number
  platform_call_count?: number
  provided_run_count: number
  provider_earned_fen: number
  provider_week_start_at?: string
  provider_week_end_at?: string
  provider_week_tokens?: number
  provider_week_run_count?: number
  provider_week_billed_fen?: number
  provider_week_earned_fen?: number
}

export function fetchMyProgression() {
  return api.get<UserProgressionSummary>('/api/me/progression')
}
