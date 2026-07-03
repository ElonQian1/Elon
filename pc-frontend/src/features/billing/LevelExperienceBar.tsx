import styles from './LevelExperienceBar.module.css'
import type { UserProgressionSummary } from './progressionApi'

interface LevelExperienceBarProps {
  progression: UserProgressionSummary | null
}

export default function LevelExperienceBar({ progression }: LevelExperienceBarProps) {
  if (!progression) return null

  const consumed = clampPercent(progression.consumed_progress_ratio)
  const provided = clampPercent(progression.provided_progress_ratio)
  const providedLeft = Math.min(consumed, 100)
  const progressPercent = Math.round(progression.level_progress_ratio * 100)
  const levelSpan = progression.next_level_tokens - progression.level_floor_tokens
  const nextLevel = progression.level + 1
  const title = [
    `Lv.${progression.level} ${progression.tier_name}`,
    `本级 ${formatTokens(progression.tokens_into_level)} / ${formatTokens(levelSpan)}`,
    `距 Lv.${nextLevel} ${formatTokens(progression.tokens_to_next_level)}`,
    `消耗 ${formatTokens(progression.consumed_tokens)}`,
    `贡献 ${formatTokens(progression.provided_tokens)}`,
  ].join(' · ')

  return (
    <div
      className={styles.wrap}
      title={title}
      role="meter"
      aria-label={title}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={progressPercent}
      tabIndex={0}
    >
      <div className={styles.meta}>
        <span>Lv.{progression.level}</span>
        <span>{progressPercent}%</span>
      </div>
      <div className={styles.track}>
        <span
          className={styles.consumed}
          style={{ width: `${consumed}%` }}
        />
        <span
          className={styles.provided}
          style={{ left: `${providedLeft}%`, width: `${Math.max(0, Math.min(provided, 100 - providedLeft))}%` }}
        />
      </div>
      <div className={styles.panel} aria-hidden="true">
        <div className={styles.panelHead}>
          <strong>{progression.tier_name}</strong>
          <span>Lv.{progression.level}</span>
        </div>
        <div className={styles.nextLine}>
          <span>距 Lv.{nextLevel}</span>
          <strong>{formatTokens(progression.tokens_to_next_level)}</strong>
        </div>
        <div className={styles.detailRow}>
          <span>本级进度</span>
          <strong>{formatTokens(progression.tokens_into_level)} / {formatTokens(levelSpan)}</strong>
        </div>
        <div className={styles.detailRow}>
          <span>总经验</span>
          <strong>{formatTokens(progression.total_xp_tokens)}</strong>
        </div>
        <div className={styles.split}>
          <div>
            <span className={styles.dotConsumed} />
            <span>消耗</span>
            <strong>{formatTokens(progression.consumed_tokens)}</strong>
          </div>
          <div>
            <span className={styles.dotProvided} />
            <span>贡献</span>
            <strong>{formatTokens(progression.provided_tokens)}</strong>
          </div>
        </div>
        <div className={styles.detailRow}>
          <span>调用 / 贡献</span>
          <strong>{progression.consumed_call_count} / {progression.provided_run_count}</strong>
        </div>
        {progression.provider_earned_fen > 0 && (
          <div className={styles.detailRow}>
            <span>贡献收益</span>
            <strong>{formatFen(progression.provider_earned_fen)}</strong>
          </div>
        )}
      </div>
    </div>
  )
}

function clampPercent(value: number) {
  if (!Number.isFinite(value)) return 0
  return Math.max(0, Math.min(100, value * 100))
}

function formatTokens(value: number) {
  if (!Number.isFinite(value)) return '0'
  const abs = Math.abs(value)
  if (abs >= 1_000_000) return `${(value / 1_000_000).toFixed(abs >= 10_000_000 ? 0 : 1)}M`
  if (abs >= 1_000) return `${(value / 1_000).toFixed(abs >= 10_000 ? 0 : 1)}K`
  return `${Math.round(value)}`
}

function formatFen(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '¥0.00'
  return `¥${(value / 100).toFixed(2)}`
}
