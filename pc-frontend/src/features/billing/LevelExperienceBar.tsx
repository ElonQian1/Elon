import styles from './LevelExperienceBar.module.css'
import type { UserProgressionSummary } from './progressionApi'

interface LevelExperienceBarProps {
  progression: UserProgressionSummary | null
}

export default function LevelExperienceBar({ progression }: LevelExperienceBarProps) {
  if (!progression) return null

  const ownCodexTokens = progression.own_codex_tokens ?? 0
  const sharedCodexTokens = progression.shared_codex_tokens ?? 0
  const platformTokens = progression.platform_tokens ?? Math.max(0, progression.consumed_tokens - ownCodexTokens - sharedCodexTokens)
  const segments = progression.own_codex_progress_ratio == null
    ? legacySegments(progression)
    : [
        { key: 'own', className: styles.ownCodex, label: '自用 Codex', value: ownCodexTokens, ratio: progression.own_codex_progress_ratio ?? 0 },
        { key: 'shared', className: styles.sharedCodex, label: '用别人 Codex', value: sharedCodexTokens, ratio: progression.shared_codex_progress_ratio ?? 0 },
        { key: 'platform', className: styles.platform, label: '平台/其他', value: platformTokens, ratio: progression.platform_progress_ratio ?? 0 },
        { key: 'provided', className: styles.provided, label: '分享给别人', value: progression.provided_tokens, ratio: progression.provided_progress_ratio },
      ]
  const progressPercent = Math.round(progression.level_progress_ratio * 100)
  const levelSpan = progression.next_level_tokens - progression.level_floor_tokens
  const nextLevel = progression.level + 1
  const title = [
    `Lv.${progression.level} ${progression.tier_name}`,
    `本级 ${formatTokens(progression.tokens_into_level)} / ${formatTokens(levelSpan)}`,
    `距 Lv.${nextLevel} ${formatTokens(progression.tokens_to_next_level)}`,
    `自用 Codex ${formatTokens(ownCodexTokens)}`,
    `用别人 Codex ${formatTokens(sharedCodexTokens)}`,
    `分享给别人 ${formatTokens(progression.provided_tokens)}`,
  ].join(' · ')
  let left = 0

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
        {segments.map((segment) => {
          const width = Math.max(0, Math.min(clampPercent(segment.ratio), 100 - left))
          const style = { left: `${left}%`, width: `${width}%` }
          left += width
          return <span key={segment.key} className={`${styles.segment} ${segment.className}`} style={style} />
        })}
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
          {segments.map((segment) => (
            <div key={segment.key}>
              <span className={`${styles.dot} ${segment.className}`} />
              <span>{segment.label}</span>
              <strong>{formatTokens(segment.value)}</strong>
            </div>
          ))}
        </div>
        <div className={styles.detailRow}>
          <span>调用 / 分享</span>
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

function legacySegments(progression: UserProgressionSummary) {
  return [
    { key: 'platform', className: styles.platform, label: '消耗', value: progression.consumed_tokens, ratio: progression.consumed_progress_ratio },
    { key: 'provided', className: styles.provided, label: '分享给别人', value: progression.provided_tokens, ratio: progression.provided_progress_ratio },
  ]
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
