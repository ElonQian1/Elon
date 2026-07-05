import type { UserProgressionSummary } from '../billing/progressionApi'
import styles from './ShareSettlementCard.module.css'

export default function ShareSettlementCard({ progression }: { progression: UserProgressionSummary | null }) {
  const weeklyPoolCents = 4_615
  const fiveHoursCents = Math.round(weeklyPoolCents * 0.3)
  const hourCents = Math.round(fiveHoursCents / 5)
  const weekTokens = progression?.provider_week_tokens ?? 0
  const weekRuns = progression?.provider_week_run_count ?? 0
  const weekEarnedFen = progression?.provider_week_earned_fen ?? 0
  const lifetimeEarnedFen = progression?.provider_earned_fen ?? 0
  const weekLabel = weekRangeLabel(
    progression?.provider_week_start_at,
    progression?.provider_week_end_at,
  )

  return (
    <section className={styles.card}>
      <div className={styles.head}>
        <div>
          <span className={styles.label}>共享算力结算</span>
          <h4>别人使用你的节点</h4>
        </div>
        <span className={styles.state}>{weekLabel}</span>
      </div>
      <div className={styles.grid}>
        <div>
          <span>本周分享</span>
          <strong>{formatTokens(weekTokens)}</strong>
          <small>{weekRuns} 次运行</small>
        </div>
        <div>
          <span>本周实际收益</span>
          <strong>{formatFen(weekEarnedFen)}</strong>
          <small>来自已结算账本</small>
        </div>
        <div>
          <span>累计分享收益</span>
          <strong>{formatFen(lifetimeEarnedFen)}</strong>
          <small>可用于提现账本</small>
        </div>
        <div>
          <span>Pro 成本参考</span>
          <strong>{formatUsdCents(weeklyPoolCents)}/周</strong>
          <small>按 $200/月年化</small>
        </div>
      </div>
      <p className={styles.hint}>
        若约 5 小时等于每周 30% 容量，参考成本约 {formatUsdCents(fiveHoursCents)}，约 {formatUsdCents(hourCents)}/小时。
        这只是容量参考价；真实结算仍以平台账本和节点分账比例为准。
      </p>
    </section>
  )
}

function formatTokens(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '0'
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(value >= 10_000_000 ? 0 : 1)}M`
  if (value >= 1_000) return `${(value / 1_000).toFixed(value >= 10_000 ? 0 : 1)}K`
  return `${Math.round(value)}`
}

function formatFen(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '¥0.00'
  return `¥${(value / 100).toFixed(2)}`
}

function formatUsdCents(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '$0.00'
  return `$${(value / 100).toFixed(2)}`
}

function weekRangeLabel(start?: string | null, end?: string | null) {
  const from = formatShortDate(start)
  const to = formatShortDate(end)
  return from && to ? `${from} - ${to}` : '本周'
}

function formatShortDate(value?: string | null) {
  if (!value) return ''
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return ''
  return `${date.getMonth() + 1}/${date.getDate()}`
}
