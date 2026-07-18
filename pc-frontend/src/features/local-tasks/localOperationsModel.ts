import type {
  GlobalPublishLeaseEntry,
  GlobalPublishStatus,
  SelfEvolutionGates,
  SelfEvolutionItem,
  SelfEvolutionQueue,
} from './types'

type JsonObject = Record<string, unknown>

export function normalizeSelfEvolutionQueue(payload: unknown): SelfEvolutionQueue {
  const root = objectValue(payload)
  const gates = objectValue(root.gates)
  return {
    items: arrayValue(root.items).map(normalizeSelfEvolutionItem).filter((item) => item.logical_id),
    gates: normalizeGates(gates),
  }
}

export function normalizeGlobalPublishStatus(payload: unknown): GlobalPublishStatus {
  const root = objectValue(payload)
  const global = objectValue(root.globalPublish)
  return {
    owner: normalizeLease(global.owner),
    waiters: arrayValue(global.waiters).map(normalizeLease).filter(Boolean) as GlobalPublishLeaseEntry[],
    waiterCount: numberValue(global.waiterCount),
    queuePolicy: textValue(global.queuePolicy) || 'fifo',
    coalescingKey: textValue(global.coalescingKey) || 'kind+sha',
    immutableReleaseSha: global.immutableReleaseSha !== false,
  }
}

function normalizeSelfEvolutionItem(payload: unknown): SelfEvolutionItem {
  const item = objectValue(payload)
  return {
    logical_id: textValue(item.logical_id),
    root_task_id: textValue(item.root_task_id),
    parent_task_id: textValue(item.parent_task_id),
    project_id: textValue(item.project_id),
    conversation_id: textValue(item.conversation_id),
    workspace_path: textValue(item.workspace_path),
    prompt: textValue(item.prompt),
    status: textValue(item.status),
    active_task_id: textValue(item.active_task_id) || undefined,
    generation: numberValue(item.generation),
    pause_reason: textValue(item.pause_reason) || undefined,
    review_verdict: textValue(item.review_verdict) || undefined,
    review_note: textValue(item.review_note) || undefined,
    created_at_ms: optionalNumber(item.created_at_ms),
    updated_at_ms: optionalNumber(item.updated_at_ms),
  }
}

function normalizeGates(gates: JsonObject): SelfEvolutionGates {
  return {
    foreground_task_ids: arrayValue(gates.foreground_task_ids).map(textValue).filter(Boolean),
    publish_active: Boolean(gates.publish_active),
    publish_status: textValue(gates.publish_status),
    publish_owner: textValue(gates.publish_owner) || undefined,
    publish_waiter_count: numberValue(gates.publish_waiter_count),
    update_active: Boolean(gates.update_active),
    resource_pressure: Boolean(gates.resource_pressure),
    checked_at_ms: optionalNumber(gates.checked_at_ms),
  }
}

function normalizeLease(payload: unknown): GlobalPublishLeaseEntry | undefined {
  const lease = objectValue(payload)
  const token = textValue(lease.token)
  if (!token) return undefined
  return {
    token,
    kind: textValue(lease.kind),
    sha: textValue(lease.sha),
    builderLabel: textValue(lease.builderLabel),
    requestedAt: optionalNumber(lease.requestedAt),
    leaseExpiresAt: optionalNumber(lease.leaseExpiresAt),
  }
}

function objectValue(value: unknown): JsonObject {
  return value && typeof value === 'object' && !Array.isArray(value) ? value as JsonObject : {}
}

function arrayValue(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function textValue(value: unknown): string {
  return typeof value === 'string' ? value.trim() : value == null ? '' : String(value)
}

function numberValue(value: unknown): number {
  const parsed = typeof value === 'number' ? value : Number(value)
  return Number.isFinite(parsed) ? parsed : 0
}

function optionalNumber(value: unknown): number | undefined {
  const parsed = numberValue(value)
  return parsed > 0 ? parsed : undefined
}
