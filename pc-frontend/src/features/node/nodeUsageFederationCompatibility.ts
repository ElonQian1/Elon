import type { NodeComputeRun, NodeUsageResponse } from './types'

export const LEGACY_LLM_V1_PROJECTION_LIST_SCHEMA = 'compute_federation.legacy_llm_v1_projection_list.v1'
export const LEGACY_LLM_V1_PROJECTION_SCHEMA = 'compute_federation.legacy_llm_v1_projection.v1'

export type LegacyLlmCompatibilitySide = 'consuming' | 'providing'

export interface LegacyLlmV1CompatibilityProjection {
  schema: typeof LEGACY_LLM_V1_PROJECTION_SCHEMA
  compatibility_level: 'partial'
  provider_kind: 'user_node'
  task_kind: 'llm_chat'
  source_run_id: string
  source_compute_call_id: string
  consumer_account_id: string
  provider_account_id: string | null
  node_id: string
  model_id: string | null
  feature: 'node_llm'
  run_status: string
  started_at: string
  finished_at: string | null
  reserved_token_budget: number
  provider_reported_prompt_tokens: number
  provider_reported_completion_tokens: number
  legacy_billed_cost_rmb_fen: number
  legacy_provider_earned_rmb_fen: number
  legacy_settlement_status: string | null
  metering_trust: 'provider_reported_unverified'
  missing_contracts: string[]
}

const REQUIRED_STRINGS = [
  'source_run_id', 'source_compute_call_id', 'consumer_account_id', 'node_id',
  'run_status', 'started_at',
] as const
const NULLABLE_STRINGS = [
  'provider_account_id', 'model_id', 'finished_at', 'legacy_settlement_status',
] as const
const REQUIRED_NUMBERS = [
  'reserved_token_budget', 'provider_reported_prompt_tokens',
  'provider_reported_completion_tokens', 'legacy_billed_cost_rmb_fen',
  'legacy_provider_earned_rmb_fen',
] as const

export function indexLegacyLlmCompatibility(
  response: NodeUsageResponse,
  side: LegacyLlmCompatibilitySide,
): ReadonlyMap<string, LegacyLlmV1CompatibilityProjection> {
  const rawById = new Map<string, NodeComputeRun>()
  for (const run of response[side] ?? []) {
    if (!run.id) continue
    if (rawById.has(run.id)) return new Map()
    rawById.set(run.id, run)
  }
  const envelope = response.federation_compatibility
  if (!isRecord(envelope) || envelope.schema !== LEGACY_LLM_V1_PROJECTION_LIST_SCHEMA) return new Map()
  const selected = envelope[side]
  if (!Array.isArray(selected)) return new Map()
  const result = new Map<string, LegacyLlmV1CompatibilityProjection>()
  for (const item of selected) {
    if (!isProjection(item) || !item.source_run_id || result.has(item.source_run_id)) return new Map()
    const raw = rawById.get(item.source_run_id)
    if (!raw || raw.feature !== 'node_llm' || raw.usage_mode !== 'server_node_llm') return new Map()
    result.set(item.source_run_id, item)
  }
  return result
}

function isProjection(value: unknown): value is LegacyLlmV1CompatibilityProjection {
  if (!isRecord(value)
    || value.schema !== LEGACY_LLM_V1_PROJECTION_SCHEMA
    || value.compatibility_level !== 'partial'
    || value.provider_kind !== 'user_node'
    || value.task_kind !== 'llm_chat'
    || value.feature !== 'node_llm'
    || value.metering_trust !== 'provider_reported_unverified'
    || !Array.isArray(value.missing_contracts)
    || !value.missing_contracts.every((item) => typeof item === 'string')) return false
  if (REQUIRED_STRINGS.some((key) => typeof value[key] !== 'string')) return false
  if (NULLABLE_STRINGS.some((key) => value[key] !== null && typeof value[key] !== 'string')) return false
  return REQUIRED_NUMBERS.every((key) => typeof value[key] === 'number' && Number.isFinite(value[key]))
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
