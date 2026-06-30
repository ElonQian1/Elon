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
  const title = [
    `Lv.${progression.level} ${progression.tier_name}`,
    `本级 ${formatTokens(progression.tokens_into_level)} / ${formatTokens(progression.next_level_tokens - progression.level_floor_tokens)}`,
    `消耗 ${formatTokens(progression.consumed_tokens)}`,
    `分享 ${formatTokens(progression.provided_tokens)}`,
  ].join(' · ')

  return (
    <div className={styles.wrap} title={title} aria-label={title}>
      <div className={styles.meta}>
        <span>Lv.{progression.level}</span>
        <span>{Math.round(progression.level_progress_ratio * 100)}%</span>
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
