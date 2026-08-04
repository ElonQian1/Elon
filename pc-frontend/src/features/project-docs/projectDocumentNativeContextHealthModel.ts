import { nodeApi } from '../node/localNodeApi'

export interface NativeContextMemoryHealth {
  checked_count: number
  current_count: number
  drifted_count: number
  relocation_suggested_count: number
  expired_count: number
  review_overdue_count: number
  governance_incomplete_count: number
  potential_conflict_count: number
  truncated: boolean
  failure_policy: 'advisory' | 'fail_on_drift' | 'strict'
  policy_outcome: {
    status: 'pass' | 'warn' | 'fail'
    recommended_exit_code: number
    reasons: string[]
  }
  items: NativeContextMemoryHealthItem[]
  receipt_automation: {
    node_policy_enabled: boolean
    trust_mode: string
    trust_bypass_enabled: boolean
  }
  capabilities: {
    runtime_observation_status: string
    private_memories_relationship: string
    private_memories_read: boolean
  }
}

export interface NativeContextMemoryHealthItem {
  candidate_id: string
  status: 'current' | 'drifted' | 'relocation_suggested' | 'expired' | 'review_overdue' | 'governance_incomplete' | 'potential_conflict'
  owner: string
  repair_plan: Array<{ code: string; action: string; automatic: boolean }>
}

interface NativeContextEnvelope<T> {
  ok: boolean
  result?: T
  error?: string
}

export async function loadNativeContextMemoryHealth(input: {
  adminUrl: string
  projectRoot: string
}): Promise<NativeContextMemoryHealth> {
  const envelope = await nodeApi<NativeContextEnvelope<Record<string, unknown>>>(
    input.adminUrl,
    '/api/project-docs/native-context/health',
    {
      method: 'POST',
      body: JSON.stringify({
        project_root: input.projectRoot,
        offset: 0,
        limit: 200,
        failure_policy: 'advisory',
        include_capabilities: true,
      }),
    },
  )
  if (!envelope.ok || !envelope.result) throw new Error(envelope.error || '读取共享项目记忆健康状态失败')
  const result = envelope.result
  return {
    checked_count: safeNumber(result.checked_count),
    current_count: safeNumber(result.current_count),
    drifted_count: safeNumber(result.drifted_count),
    relocation_suggested_count: safeNumber(result.relocation_suggested_count),
    expired_count: safeNumber(result.expired_count),
    review_overdue_count: safeNumber(result.review_overdue_count),
    governance_incomplete_count: safeNumber(result.governance_incomplete_count),
    potential_conflict_count: safeNumber(result.potential_conflict_count),
    truncated: result.truncated === true,
    failure_policy: sanitizeFailurePolicy(result.failure_policy),
    policy_outcome: sanitizePolicyOutcome(result.policy_outcome),
    items: sanitizeHealthItems(result.items),
    receipt_automation: sanitizeReceiptAutomation(result.receipt_automation),
    capabilities: sanitizeCapabilities(result.capabilities),
  }
}

function sanitizeFailurePolicy(value: unknown): NativeContextMemoryHealth['failure_policy'] {
  return ['fail_on_drift', 'strict'].includes(String(value))
    ? value as NativeContextMemoryHealth['failure_policy']
    : 'advisory'
}

function sanitizePolicyOutcome(value: unknown): NativeContextMemoryHealth['policy_outcome'] {
  const outcome = objectValue(value)
  const status = ['pass', 'warn', 'fail'].includes(String(outcome.status))
    ? outcome.status as NativeContextMemoryHealth['policy_outcome']['status']
    : 'warn'
  return {
    status,
    recommended_exit_code: safeNumber(outcome.recommended_exit_code),
    reasons: uniqueStrings(outcome.reasons, 8, 64),
  }
}

function sanitizeHealthItems(value: unknown): NativeContextMemoryHealthItem[] {
  if (!Array.isArray(value)) return []
  const statuses = ['current', 'drifted', 'relocation_suggested', 'expired', 'review_overdue', 'governance_incomplete', 'potential_conflict']
  return value.slice(0, 200).flatMap((entry) => {
    const item = objectValue(entry)
    const status = boundedText(item.status, 32)
    if (!statuses.includes(status)) return []
    const repairPlan = Array.isArray(item.repair_plan) ? item.repair_plan.slice(0, 8).map((raw) => {
      const repair = objectValue(raw)
      return {
        code: boundedText(repair.code, 64),
        action: boundedText(repair.action, 500),
        automatic: repair.automatic === true,
      }
    }) : []
    return [{
      candidate_id: boundedText(item.candidate_id, 80),
      status: status as NativeContextMemoryHealthItem['status'],
      owner: boundedText(item.owner, 80),
      repair_plan: repairPlan,
    }]
  })
}

function sanitizeReceiptAutomation(value: unknown): NativeContextMemoryHealth['receipt_automation'] {
  const automation = objectValue(value)
  return {
    node_policy_enabled: automation.node_policy_enabled === true,
    trust_mode: boundedText(automation.trust_mode, 64),
    trust_bypass_enabled: automation.trust_bypass_enabled === true,
  }
}

function sanitizeCapabilities(value: unknown): NativeContextMemoryHealth['capabilities'] {
  const capabilities = objectValue(value)
  const observation = objectValue(capabilities.runtime_observation)
  const memories = objectValue(capabilities.official_codex_memories)
  return {
    runtime_observation_status: boundedText(observation.status, 64),
    private_memories_relationship: boundedText(memories.relationship, 64),
    private_memories_read: memories.read_by_project_docs === true,
  }
}

function objectValue(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' ? value as Record<string, unknown> : {}
}

function uniqueStrings(value: unknown, limit: number, charLimit: number): string[] {
  if (!Array.isArray(value)) return []
  return [...new Set(value.map((entry) => boundedText(entry, charLimit)).filter(Boolean))].slice(0, limit)
}

function boundedText(value: unknown, limit: number): string {
  return String(value ?? '').trim().replace(/\s+/g, ' ').slice(0, limit)
}

function safeNumber(value: unknown): number {
  const number = Number(value)
  return Number.isFinite(number) && number >= 0 ? Math.floor(number) : 0
}
