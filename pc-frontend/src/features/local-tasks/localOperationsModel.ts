import type {
  GlobalPublishLeaseEntry,
  GlobalPublishStatus,
  ReleaseBatchLedger,
  ReleaseBatchStage,
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
    batchIdentity: textValue(global.batchIdentity) || 'batchId+sha',
    stateHealth: textValue(root.stateHealth) || 'unavailable',
    batches: arrayValue(root.releaseBatches).map(normalizeBatch).filter((batch) => batch.batchId),
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
    execution_worktree: textValue(item.execution_worktree) || undefined,
    execution_branch: textValue(item.execution_branch) || undefined,
    execution_isolated: Boolean(item.execution_isolated),
    prompt: textValue(item.prompt),
    status: textValue(item.status),
    active_task_id: textValue(item.active_task_id) || undefined,
    generation: numberValue(item.generation),
    pause_reason: textValue(item.pause_reason) || undefined,
    yield_reason: textValue(item.yield_reason) || undefined,
    interruption_source: interruptionSource(item.interruption_source),
    review_verdict: textValue(item.review_verdict) || undefined,
    review_note: textValue(item.review_note) || undefined,
    reviewed_by: textValue(item.reviewed_by) || undefined,
    review_source: textValue(item.review_source) || undefined,
    reviewed_at_ms: optionalNumber(item.reviewed_at_ms),
    retry_count: numberValue(item.retry_count),
    max_retries: numberValue(item.max_retries),
    next_retry_at_ms: optionalNumber(item.next_retry_at_ms),
    last_error: textValue(item.last_error) || undefined,
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
  const kind = textValue(lease.kind)
  const sha = textValue(lease.sha)
  if (!kind || !sha) return undefined
  return {
    kind,
    sha,
    batchId: textValue(lease.batchId),
    stage: textValue(lease.stage),
    builderId: textValue(lease.builderId),
    builderLabel: textValue(lease.builderLabel),
    requestedAt: optionalNumber(lease.requestedAt),
    leaseExpiresAt: optionalNumber(lease.leaseExpiresAt),
  }
}

function normalizeBatch(payload: unknown): ReleaseBatchLedger {
  const batch = objectValue(payload)
  return {
    batchId: textValue(batch.batchId),
    sha: textValue(batch.sha),
    status: textValue(batch.status) || 'unknown',
    createdAt: optionalNumber(batch.createdAt),
    updatedAt: optionalNumber(batch.updatedAt),
    stages: arrayValue(batch.stages).map(normalizeBatchStage).filter((stage) => stage.stage),
  }
}

function normalizeBatchStage(payload: unknown): ReleaseBatchStage {
  const stage = objectValue(payload)
  return {
    stage: textValue(stage.stage),
    kind: textValue(stage.kind),
    status: textValue(stage.status) || 'unknown',
    builderId: textValue(stage.builderId),
    builderLabel: textValue(stage.builderLabel),
    attempt: numberValue(stage.attempt),
    requestedAt: optionalNumber(stage.requestedAt),
    lastHeartbeat: optionalNumber(stage.lastHeartbeat),
    leaseExpiresAt: optionalNumber(stage.leaseExpiresAt),
    completedAt: optionalNumber(stage.completedAt),
    errorMessage: textValue(stage.errorMessage) || undefined,
  }
}

function interruptionSource(value: unknown): SelfEvolutionItem['interruption_source'] {
  const source = textValue(value)
  return source === 'supervisor_intervention' || source === 'node_restart' || source === 'updater_apply'
    ? source
    : undefined
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
