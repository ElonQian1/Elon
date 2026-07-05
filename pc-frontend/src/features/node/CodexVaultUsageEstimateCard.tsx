import { useEffect, useMemo, useState } from 'react'
import { BarChart3, RefreshCcw } from 'lucide-react'
import { api } from '../../api/client'
import type { CodexVaultEmergencyStatus } from './types'
import styles from './CodexVaultUsageEstimateCard.module.css'

interface UsageEstimateResponse {
  ok?: boolean
  method?: string
  caveat?: string
  report?: UsageEstimateReport
}

interface UsageEstimateReport {
  provider_user_id: string
  limit_id: string
  days: number
  monthly_usd_cents: number
  windows: UsageEstimateWindow[]
}

interface UsageEstimateWindow {
  resets_at?: string | null
  first_snapshot_at: string
  last_snapshot_at: string
  consumed_percent: number
  official_token_delta?: number | null
  shared_token_total: number
  denominator_tokens?: number | null
  confidence: string
  estimated_window_cost_usd_cents: number
  allocations: UsageEstimateAllocation[]
  unattributed_percent: number
  unattributed_cost_usd_cents: number
}

interface UsageEstimateAllocation {
  consumer_user_id: string
  consumer_account: string
  consumer_nickname?: string | null
  consumer_node_id: string
  lease_id: string
  tokens: number
  input_tokens: number
  output_tokens: number
  token_share: number
  estimated_percent: number
  estimated_cost_usd_cents: number
  billed_cost_rmb_fen: number
  provider_earned_fen: number
  event_count: number
}

interface ProviderOption {
  id: string
  label: string
  role: 'provider' | 'consumer'
}

interface RobotUsageRow {
  userId: string
  label: string
  nodes: Set<string>
  tokens: number
  inputTokens: number
  outputTokens: number
  estimatedPercent: number
  estimatedCostUsdCents: number
  billedCostRmbFen: number
  providerEarnedFen: number
  eventCount: number
}

export default function CodexVaultUsageEstimateCard({
  sharing,
  currentUserId,
}: {
  sharing?: CodexVaultEmergencyStatus
  currentUserId?: string
}) {
  const providers = useMemo(() => providerOptions(sharing, currentUserId), [sharing, currentUserId])
  const [providerId, setProviderId] = useState('')
  const [days, setDays] = useState(30)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [report, setReport] = useState<UsageEstimateReport | null>(null)

  useEffect(() => {
    if (!providers.length) {
      setProviderId('')
      return
    }
    if (!providerId || !providers.some((item) => item.id === providerId)) {
      setProviderId(providers[0].id)
    }
  }, [providerId, providers])

  useEffect(() => {
    if (!providerId) {
      setReport(null)
      return
    }
    void loadReport()
  }, [providerId, days])

  const summary = useMemo(() => summarizeReport(report), [report])
  const periodPoolCents = report ? Math.round((report.monthly_usd_cents || 0) * (report.days || days) / 30) : 0
  const provider = providers.find((item) => item.id === providerId)

  async function loadReport() {
    if (!providerId) return
    setLoading(true)
    setError('')
    try {
      const params = new URLSearchParams({
        provider_user_id: providerId,
        limit_id: 'codex',
        days: String(days),
        monthly_usd_cents: '20000',
      })
      const data = await api.get<UsageEstimateResponse>(`/api/me/codex-vault/sharing/usage-estimate?${params.toString()}`)
      setReport(data.report ?? null)
    } catch (err) {
      setReport(null)
      setError(errorMessage(err))
    } finally {
      setLoading(false)
    }
  }

  if (!providers.length) {
    return (
      <section className={styles.card}>
        <div className={styles.head}>
          <div>
            <span className={styles.label}>Codex 共享用量</span>
            <h4>暂无可查看的共享账号</h4>
          </div>
          <span className={styles.state}>未授权</span>
        </div>
      </section>
    )
  }

  return (
    <section className={styles.card}>
      <div className={styles.head}>
        <div>
          <span className={styles.label}>Codex 共享用量</span>
          <h4>{provider?.label ?? '共享账号'} · 机器人分摊</h4>
        </div>
        <span className={styles.state}>{loading ? '刷新中' : `${summary.windowCount} 个窗口`}</span>
      </div>

      <div className={styles.toolbar}>
        <div className={styles.providerTabs}>
          {providers.map((item) => (
            <button
              key={item.id}
              type="button"
              className={[styles.tab, item.id === providerId ? styles.tabActive : ''].join(' ')}
              onClick={() => setProviderId(item.id)}
              title={item.role === 'provider' ? '我的共享账号' : '对方共享给我的账号'}
            >
              {item.label}
            </button>
          ))}
        </div>
        <div className={styles.actions}>
          {[7, 30].map((value) => (
            <button
              key={value}
              type="button"
              className={[styles.tab, days === value ? styles.tabActive : ''].join(' ')}
              onClick={() => setDays(value)}
            >
              {value}天
            </button>
          ))}
          <button
            type="button"
            className={[styles.btn, styles.iconBtn].join(' ')}
            onClick={() => void loadReport()}
            disabled={loading}
            title="刷新共享用量估算"
          >
            <RefreshCcw size={14} strokeWidth={2.2} aria-hidden="true" />
            刷新
          </button>
        </div>
      </div>

      {error && <p className={styles.error}>{error}</p>}

      <div className={styles.summaryGrid}>
        <div>
          <span>官方百分比</span>
          <strong>{formatPercent(summary.consumedPercent)}</strong>
          <small>窗口消耗合计</small>
        </div>
        <div>
          <span>机器人 token</span>
          <strong>{formatTokens(summary.tokens)}</strong>
          <small>{summary.eventCount} 次记录</small>
        </div>
        <div>
          <span>权益消耗估算</span>
          <strong>{formatUsdCents(summary.estimatedCostUsdCents)}</strong>
          <small>按百分比实际用掉</small>
        </div>
        <div>
          <span>包月回本池</span>
          <strong>{formatUsdCents(periodPoolCents)}</strong>
          <small>$200 按周期摊销</small>
        </div>
      </div>

      {summary.rows.length > 0 ? (
        <div className={styles.tableWrap}>
          <table className={styles.table}>
            <thead>
              <tr>
                <th>机器人</th>
                <th>token</th>
                <th>百分比</th>
                <th>权益成本</th>
                <th>回本分摊</th>
                <th>平台扣费</th>
                <th>全嘉收益</th>
              </tr>
            </thead>
            <tbody>
              {summary.rows.map((row) => {
                const recoveryCents = summary.allocatedPercent > 0
                  ? Math.round(periodPoolCents * row.estimatedPercent / summary.allocatedPercent)
                  : 0
                return (
                  <tr key={row.userId}>
                    <td>
                      <strong>{row.label}</strong>
                      <small>{row.nodes.size ? `${row.nodes.size} 个节点 · ${row.eventCount} 次` : `${row.eventCount} 次`}</small>
                    </td>
                    <td>{formatTokens(row.tokens)}</td>
                    <td>{formatPercent(row.estimatedPercent)}</td>
                    <td>{formatUsdCents(row.estimatedCostUsdCents)}</td>
                    <td>{formatUsdCents(recoveryCents)}</td>
                    <td>{formatFen(row.billedCostRmbFen)}</td>
                    <td>{formatFen(row.providerEarnedFen)}</td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      ) : (
        <div className={styles.empty}>
          <BarChart3 size={18} strokeWidth={2.2} aria-hidden="true" />
          <span>等待官方快照和 shared_codex token 记录形成同一窗口。</span>
        </div>
      )}

      <div className={styles.meta}>
        <span>未归因 {formatPercent(summary.unattributedPercent)}</span>
        <span>{confidenceLabel(summary.confidence)}</span>
        <span>{formatShortTime(summary.lastSnapshotAt)}</span>
      </div>
    </section>
  )
}

function providerOptions(sharing?: CodexVaultEmergencyStatus, currentUserId?: string): ProviderOption[] {
  const map = new Map<string, ProviderOption>()
  for (const grant of sharing?.grants ?? []) {
    if (!grant.provider_user_id) continue
    if (grant.provider_user_id === currentUserId) {
      map.set(grant.provider_user_id, {
        id: grant.provider_user_id,
        label: '我的 Codex 账号',
        role: 'provider',
      })
    }
    if (grant.consumer_user_id === currentUserId) {
      map.set(grant.provider_user_id, {
        id: grant.provider_user_id,
        label: robotLabel(grant.provider_nickname, grant.provider_account, grant.provider_user_id),
        role: 'consumer',
      })
    }
  }
  for (const lease of sharing?.leases ?? []) {
    if (!lease.provider_user_id) continue
    if (lease.provider_user_id === currentUserId) {
      map.set(lease.provider_user_id, {
        id: lease.provider_user_id,
        label: '我的 Codex 账号',
        role: 'provider',
      })
    }
    if (lease.consumer_user_id === currentUserId) {
      map.set(lease.provider_user_id, {
        id: lease.provider_user_id,
        label: robotLabel(lease.provider_nickname, lease.provider_account, lease.provider_user_id),
        role: 'consumer',
      })
    }
  }
  return Array.from(map.values())
}

function summarizeReport(report: UsageEstimateReport | null) {
  const rows = new Map<string, RobotUsageRow>()
  let consumedPercent = 0
  let allocatedPercent = 0
  let tokens = 0
  let eventCount = 0
  let estimatedCostUsdCents = 0
  let unattributedPercent = 0
  let confidence = ''
  let lastSnapshotAt = ''

  for (const window of report?.windows ?? []) {
    consumedPercent += Number(window.consumed_percent || 0)
    unattributedPercent += Number(window.unattributed_percent || 0)
    if (window.confidence) confidence = confidence ? `${confidence},${window.confidence}` : window.confidence
    if (window.last_snapshot_at && (!lastSnapshotAt || window.last_snapshot_at > lastSnapshotAt)) {
      lastSnapshotAt = window.last_snapshot_at
    }
    for (const allocation of window.allocations ?? []) {
      const key = allocation.consumer_user_id || allocation.consumer_account || allocation.lease_id
      const existing = rows.get(key) ?? {
        userId: key,
        label: robotLabel(allocation.consumer_nickname, allocation.consumer_account, allocation.consumer_user_id),
        nodes: new Set<string>(),
        tokens: 0,
        inputTokens: 0,
        outputTokens: 0,
        estimatedPercent: 0,
        estimatedCostUsdCents: 0,
        billedCostRmbFen: 0,
        providerEarnedFen: 0,
        eventCount: 0,
      }
      if (allocation.consumer_node_id) existing.nodes.add(allocation.consumer_node_id)
      existing.tokens += Number(allocation.tokens || 0)
      existing.inputTokens += Number(allocation.input_tokens || 0)
      existing.outputTokens += Number(allocation.output_tokens || 0)
      existing.estimatedPercent += Number(allocation.estimated_percent || 0)
      existing.estimatedCostUsdCents += Number(allocation.estimated_cost_usd_cents || 0)
      existing.billedCostRmbFen += Number(allocation.billed_cost_rmb_fen || 0)
      existing.providerEarnedFen += Number(allocation.provider_earned_fen || 0)
      existing.eventCount += Number(allocation.event_count || 0)
      rows.set(key, existing)
    }
  }

  const sortedRows = Array.from(rows.values()).sort((a, b) => b.estimatedPercent - a.estimatedPercent)
  for (const row of sortedRows) {
    tokens += row.tokens
    eventCount += row.eventCount
    allocatedPercent += row.estimatedPercent
    estimatedCostUsdCents += row.estimatedCostUsdCents
  }

  return {
    rows: sortedRows,
    windowCount: report?.windows?.length ?? 0,
    consumedPercent,
    allocatedPercent,
    tokens,
    eventCount,
    estimatedCostUsdCents,
    unattributedPercent,
    confidence,
    lastSnapshotAt,
  }
}

function robotLabel(nickname?: string | null, account?: string | null, userId?: string | null): string {
  return nickname || account || userId || '机器人账号'
}

function formatTokens(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '0'
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(value >= 10_000_000 ? 0 : 1)}M`
  if (value >= 1_000) return `${(value / 1_000).toFixed(value >= 10_000 ? 0 : 1)}K`
  return `${Math.round(value)}`
}

function formatPercent(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '0%'
  return `${value.toFixed(value >= 10 ? 1 : 2)}%`
}

function formatUsdCents(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '$0.00'
  return `$${(value / 100).toFixed(2)}`
}

function formatFen(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '¥0.00'
  return `¥${(value / 100).toFixed(2)}`
}

function formatShortTime(value?: string | null) {
  if (!value) return '暂无快照'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return `更新 ${date.toLocaleString()}`
}

function confidenceLabel(value: string) {
  if (!value) return '等待校准'
  if (value.includes('official_lifetime_calibrated')) return '官方 token 校准'
  if (value.includes('official_delta_below_shared_tokens')) return 'token 口径需复核'
  if (value.includes('shared_token_proportional')) return '共享 token 比例'
  if (value.includes('insufficient_token_data')) return '数据不足'
  return value
}

function errorMessage(err: unknown) {
  if (typeof err === 'object' && err && 'message' in err) {
    return String((err as { message?: unknown }).message || '读取失败')
  }
  return '读取失败'
}
