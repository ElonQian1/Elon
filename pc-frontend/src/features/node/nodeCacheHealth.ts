import { api, type ApiError } from '../../api/client'

const LATEST_SCHEMA = 'elon.rust_cache.fleet_latest.v1'
const STALE_AFTER_MS = 24 * 60 * 60 * 1000
const FUTURE_CLOCK_TOLERANCE_MS = 5 * 60 * 1000

export interface NodeCacheHealthResponse {
  schema: string
  node_id: string
  envelope_id: string
  report_sha256: string
  platform_health: string
  gc_review_recommended: boolean
  active_writer_count: number
  managed_size_bytes: number | null
  generated_at: string
  received_at: string
  destructive_actions_authorized: boolean
  report: {
    cache?: {
      partition_count?: number
      locked_partition_count?: number
      quarantine_partition_count?: number
      legacy_cache_count?: number
      retired_legacy_cache_count?: number
    }
    volume?: {
      free_bytes?: number
      free_percent?: number
      warning_free_percent?: number
    }
    platform?: {
      actionable_checks?: Array<{ id?: string; status?: string }>
    }
  }
}

export interface NodeCacheHealthView {
  tone: 'healthy' | 'attention' | 'critical'
  label: string
  summary: string
  generatedAtMs: number
  ageMs: number
  stale: boolean
}

export async function fetchNodeCacheHealth(nodeId: string): Promise<NodeCacheHealthResponse | null> {
  try {
    const response = await api.get<NodeCacheHealthResponse>(
      `/api/me/nodes/${encodeURIComponent(nodeId)}/cache-reports/latest`,
    )
    assertSafeResponse(response, nodeId)
    return response
  } catch (reason) {
    if (isApiError(reason) && reason.status === 404) return null
    throw reason
  }
}

export function deriveNodeCacheHealth(
  response: NodeCacheHealthResponse,
  nowMs = Date.now(),
): NodeCacheHealthView {
  const generatedAtMs = Date.parse(response.generated_at)
  const ageMs = Number.isFinite(generatedAtMs) ? Math.max(0, nowMs - generatedAtMs) : Number.POSITIVE_INFINITY
  const clockInvalid = !Number.isFinite(generatedAtMs) || generatedAtMs - nowMs > FUTURE_CLOCK_TOLERANCE_MS
  const stale = ageMs > STALE_AFTER_MS
  const health = response.platform_health.trim().toLowerCase()
  const critical = clockInvalid || ['critical', 'failed', 'unhealthy', 'error'].includes(health)

  if (critical) {
    return {
      tone: 'critical',
      label: '报告异常',
      summary: clockInvalid ? '节点报告时间无效，需要检查节点时钟。' : '缓存平台报告故障，需要在对应电脑检查。',
      generatedAtMs,
      ageMs,
      stale,
    }
  }
  if (response.gc_review_recommended) {
    return {
      tone: 'attention',
      label: '建议检查',
      summary: '磁盘余量或退休分区触发了本机 GC 预演建议。',
      generatedAtMs,
      ageMs,
      stale,
    }
  }
  if (stale) {
    return {
      tone: 'attention',
      label: '报告已陈旧',
      summary: '超过 24 小时未收到新的缓存健康报告。',
      generatedAtMs,
      ageMs,
      stale,
    }
  }
  if (!['healthy', 'pass', 'ok'].includes(health)) {
    return {
      tone: 'attention',
      label: '需要关注',
      summary: '缓存平台存在未通过的健康检查，请在节点本机复核。',
      generatedAtMs,
      ageMs,
      stale,
    }
  }
  return {
    tone: 'healthy',
    label: '运行健康',
    summary: '共享缓存、平台完整性和磁盘余量未报告风险。',
    generatedAtMs,
    ageMs,
    stale,
  }
}

export function formatCacheReportAge(ageMs: number): string {
  if (!Number.isFinite(ageMs)) return '时间无效'
  const minutes = Math.floor(ageMs / 60_000)
  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes} 分钟前`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours} 小时前`
  return `${Math.floor(hours / 24)} 天前`
}

function assertSafeResponse(response: NodeCacheHealthResponse, expectedNodeId: string) {
  if (
    response.schema !== LATEST_SCHEMA
    || response.node_id !== expectedNodeId
    || response.destructive_actions_authorized !== false
    || !response.report
  ) {
    throw new Error('缓存健康报告身份或安全合同不匹配')
  }
}

function isApiError(reason: unknown): reason is ApiError {
  return typeof reason === 'object'
    && reason !== null
    && typeof (reason as ApiError).status === 'number'
    && typeof (reason as ApiError).message === 'string'
}
