import { api, type ApiError } from '../../api/client'

const REQUEST_SCHEMA = 'elon.rust_cache.gc_request.v1'

export interface NodeCacheGcPlan {
  schema: string
  request_id: string
  plan_id: string
  plan_digest: string
  node_id: string
  generated_at_utc: string
  expires_at_utc: string
  action_count: number
  reclaim_bytes: number
  active_writer_count: number
  reasons: Array<{ reason: string; count: number }>
  security: {
    absolute_paths_included: boolean
    secrets_included: boolean
    destructive_actions_authorized: boolean
    approval_binds_plan_digest: boolean
    target_rescan_required: boolean
  }
}

export interface NodeCacheGcResult {
  schema: string
  request_id: string
  node_id: string
  phase: 'plan' | 'apply'
  status: 'completed' | 'partial' | 'failed'
  plan_id: string | null
  plan_digest: string | null
  approved_action_count: number
  removed_action_count: number
  reclaimed_bytes: number
  failure_code?: string | null
  security: {
    absolute_paths_included: boolean
    secrets_included: boolean
    execution_bound_to_plan_digest: boolean
    local_rescan_completed: boolean
  }
}

export interface NodeCacheGcRequest {
  schema: string
  request_id: string
  node_id: string
  status: 'requested' | 'plan_ready' | 'approved' | 'rejected' | 'executing' | 'completed' | 'partial' | 'failed' | 'expired'
  plan: NodeCacheGcPlan | null
  result: NodeCacheGcResult | null
  failure_code: string | null
  created_at: string
  updated_at: string
  expires_at: string
  server_has_absolute_paths: boolean
}

export async function fetchLatestCacheGc(nodeId: string): Promise<NodeCacheGcRequest | null> {
  try {
    return assertRequest(await api.get<NodeCacheGcRequest>(path(nodeId)), nodeId)
  } catch (reason) {
    if (isApiError(reason) && reason.status === 404) return null
    throw reason
  }
}

export async function createCacheGcPlan(nodeId: string): Promise<NodeCacheGcRequest> {
  return assertRequest(await api.post<NodeCacheGcRequest>(path(nodeId), {
    options: {
      force_aged: false,
      workspace_only: false,
      recover_missing_workspaces: false,
      shared_aliases_only: false,
    },
    acknowledge_remote_gc: true,
  }), nodeId)
}

export async function approveCacheGcPlan(nodeId: string, plan: NodeCacheGcPlan): Promise<NodeCacheGcRequest> {
  return assertRequest(await api.post<NodeCacheGcRequest>(
    `${path(nodeId)}/${encodeURIComponent(plan.request_id)}/approve`,
    {
      plan_id: plan.plan_id,
      plan_digest: plan.plan_digest,
      acknowledgement: 'APPROVE_EXACT_GC_PLAN',
    },
  ), nodeId)
}

export async function rejectCacheGcPlan(nodeId: string, requestId: string): Promise<NodeCacheGcRequest> {
  return assertRequest(await api.post<NodeCacheGcRequest>(
    `${path(nodeId)}/${encodeURIComponent(requestId)}/reject`,
    {},
  ), nodeId)
}

export function cacheGcStatusLabel(status: NodeCacheGcRequest['status']): string {
  return {
    requested: '等待目标电脑生成预演',
    plan_ready: '预演待审批',
    approved: '已批准，等待目标电脑',
    executing: '目标电脑正在重新扫描',
    completed: '回收完成',
    partial: '部分完成，需要复核',
    failed: '执行被拒绝或失败',
    rejected: '已取消',
    expired: '预演已过期',
  }[status]
}

function path(nodeId: string) {
  return `/api/me/nodes/${encodeURIComponent(nodeId)}/cache-gc`
}

function assertRequest(request: NodeCacheGcRequest, expectedNodeId: string): NodeCacheGcRequest {
  const validStatus = new Set([
    'requested', 'plan_ready', 'approved', 'rejected', 'executing',
    'completed', 'partial', 'failed', 'expired',
  ]).has(request.status)
  const planValid = !request.plan || (
    request.plan.schema === 'elon.rust_cache.gc_plan_summary.v1'
    && request.plan.request_id === request.request_id
    && request.plan.node_id === expectedNodeId
    && /^[0-9a-f]{32}$/.test(request.plan.plan_id)
    && /^[0-9a-f]{64}$/.test(request.plan.plan_digest)
    && Number.isSafeInteger(request.plan.action_count)
    && request.plan.action_count >= 0
    && !request.plan.security.absolute_paths_included
    && !request.plan.security.secrets_included
    && !request.plan.security.destructive_actions_authorized
    && request.plan.security.approval_binds_plan_digest
    && request.plan.security.target_rescan_required
  )
  const resultValid = !request.result || (
    request.result.schema === 'elon.rust_cache.gc_result_summary.v1'
    && request.result.request_id === request.request_id
    && request.result.node_id === expectedNodeId
    && !request.result.security.absolute_paths_included
    && !request.result.security.secrets_included
    && (request.result.phase !== 'apply' || (
      request.result.security.execution_bound_to_plan_digest
      && (request.result.status === 'failed' || request.result.security.local_rescan_completed)
    ))
  )
  if (
    request.schema !== REQUEST_SCHEMA
    || !/^[0-9a-f]{32}$/.test(request.request_id)
    || request.node_id !== expectedNodeId
    || !validStatus
    || request.server_has_absolute_paths !== false
    || !planValid
    || !resultValid
  ) {
    throw new Error('缓存回收审批响应的身份或安全合同不匹配')
  }
  return request
}

function isApiError(reason: unknown): reason is ApiError {
  return typeof reason === 'object'
    && reason !== null
    && typeof (reason as ApiError).status === 'number'
}
