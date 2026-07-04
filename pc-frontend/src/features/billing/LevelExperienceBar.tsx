import styles from './LevelExperienceBar.module.css'
import type { UserProgressionSummary } from './progressionApi'

interface LevelExperienceBarProps {
  progression: UserProgressionSummary | null
}

export default function LevelExperienceBar({ progression }: LevelExperienceBarProps) {
  if (!progression) return null

  const level = positiveInteger(progression.level, 1)
  const tierName = progression.tier_name || '初阶算力'
  const totalXpTokens = nonNegative(progression.total_xp_tokens)
  const consumedTokens = nonNegative(progression.consumed_tokens)
  const ownCodexTokens = nonNegative(progression.own_codex_tokens)
  const sharedCodexTokens = nonNegative(progression.shared_codex_tokens)
  const platformTokens = nonNegative(
    progression.platform_tokens,
    Math.max(0, consumedTokens - ownCodexTokens - sharedCodexTokens),
  )
  const providedTokens = nonNegative(progression.provided_tokens)
  const levelFloorTokens = nonNegative(progression.level_floor_tokens)
  const nextLevelTokens = Math.max(levelFloorTokens + 1, nonNegative(progression.next_level_tokens, levelFloorTokens + 1))
  const levelSpan = Math.max(1, nextLevelTokens - levelFloorTokens)
  const tokensIntoLevel = Math.min(levelSpan, nonNegative(progression.tokens_into_level))
  const tokensToNextLevel = nonNegative(progression.tokens_to_next_level, Math.max(0, nextLevelTokens - totalXpTokens))
  const consumedCallCount = nonNegative(progression.consumed_call_count)
  const ownCodexCallCount = nonNegative(progression.own_codex_call_count)
  const sharedCodexCallCount = nonNegative(progression.shared_codex_call_count)
  const platformCallCount = nonNegative(
    progression.platform_call_count,
    Math.max(0, consumedCallCount - ownCodexCallCount - sharedCodexCallCount),
  )
  const providedRunCount = nonNegative(progression.provided_run_count)
  const providerEarnedFen = nonNegative(progression.provider_earned_fen)
  const segments = progression.own_codex_progress_ratio == null
    ? [
        {
          key: 'platform',
          className: styles.platform,
          label: '消耗',
          caption: '历史统计',
          value: consumedTokens,
          ratio: safeRatio(progression.consumed_progress_ratio),
          callCount: consumedCallCount,
        },
        {
          key: 'provided',
          className: styles.provided,
          label: '分享给别人',
          caption: '历史统计',
          value: providedTokens,
          ratio: safeRatio(progression.provided_progress_ratio),
          callCount: providedRunCount,
        },
      ]
    : [
        {
          key: 'own',
          className: styles.ownCodex,
          label: '自用 Codex',
          caption: '不扣平台额度',
          value: ownCodexTokens,
          ratio: safeRatio(progression.own_codex_progress_ratio),
          callCount: ownCodexCallCount,
        },
        {
          key: 'shared',
          className: styles.sharedCodex,
          label: '借用 Codex',
          caption: '使用别人节点',
          value: sharedCodexTokens,
          ratio: safeRatio(progression.shared_codex_progress_ratio),
          callCount: sharedCodexCallCount,
        },
        {
          key: 'platform',
          className: styles.platform,
          label: '平台/其他',
          caption: '平台余额承载',
          value: platformTokens,
          ratio: safeRatio(progression.platform_progress_ratio),
          callCount: platformCallCount,
        },
        {
          key: 'provided',
          className: styles.provided,
          label: '分享给别人',
          caption: '别人使用你的节点',
          value: providedTokens,
          ratio: safeRatio(progression.provided_progress_ratio),
          callCount: providedRunCount,
        },
      ]
  const levelProgressRatio = safeRatio(progression.level_progress_ratio, tokensIntoLevel / levelSpan)
  const progressPercent = Math.round(levelProgressRatio * 100)
  const nextLevel = level + 1
  const title = [
    `Lv.${level} ${tierName}`,
    `本级 ${formatTokens(tokensIntoLevel)} / ${formatTokens(levelSpan)}`,
    `距 Lv.${nextLevel} ${formatTokens(tokensToNextLevel)}`,
    `自用 Codex ${formatTokens(ownCodexTokens)} / ${formatCount(ownCodexCallCount)}`,
    `借用 Codex ${formatTokens(sharedCodexTokens)} / ${formatCount(sharedCodexCallCount)}`,
    `分享给别人 ${formatTokens(providedTokens)} / ${formatCount(providedRunCount)}`,
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
        <span>Lv.{level}</span>
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
          <strong>{tierName}</strong>
          <span>Lv.{level}</span>
        </div>
        <div className={styles.nextLine}>
          <span>距 Lv.{nextLevel}</span>
          <strong>{formatTokens(tokensToNextLevel)}</strong>
        </div>
        <div className={styles.detailRow}>
          <span>本级进度</span>
          <strong>{formatTokens(tokensIntoLevel)} / {formatTokens(levelSpan)}</strong>
        </div>
        <div className={styles.detailRow}>
          <span>总经验</span>
          <strong>{formatTokens(totalXpTokens)}</strong>
        </div>
        <div className={styles.split}>
          {segments.map((segment) => (
            <div key={segment.key}>
              <span className={`${styles.dot} ${segment.className}`} />
              <span className={styles.segmentInfo}>
                <span>{segment.label}</span>
                <small>{segment.caption}</small>
              </span>
              <strong>{formatTokens(segment.value)}</strong>
              <em>{formatCount(segment.callCount)}</em>
            </div>
          ))}
        </div>
        <div className={styles.detailRow}>
          <span>调用 / 分享</span>
          <strong>{formatPlainCount(consumedCallCount)} / {formatPlainCount(providedRunCount)}</strong>
        </div>
        {providerEarnedFen > 0 && (
          <div className={styles.detailRow}>
            <span>贡献收益</span>
            <strong>{formatFen(providerEarnedFen)}</strong>
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

function safeRatio(value: number | undefined, fallback = 0) {
  const ratio = typeof value === 'number' && Number.isFinite(value) ? value : fallback
  return Math.max(0, Math.min(1, ratio))
}

function nonNegative(value: number | undefined, fallback = 0) {
  const resolved = typeof value === 'number' && Number.isFinite(value) ? value : fallback
  return Math.max(0, resolved)
}

function positiveInteger(value: number | undefined, fallback: number) {
  return Math.max(1, Math.round(nonNegative(value, fallback)))
}

function formatTokens(value: number) {
  if (!Number.isFinite(value)) return '0'
  const abs = Math.abs(value)
  if (abs >= 1_000_000) return `${(value / 1_000_000).toFixed(abs >= 10_000_000 ? 0 : 1)}M`
  if (abs >= 1_000) return `${(value / 1_000).toFixed(abs >= 10_000 ? 0 : 1)}K`
  return `${Math.round(value)}`
}

function formatCount(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '0 次'
  return `${Math.round(value)} 次`
}

function formatPlainCount(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '0'
  return `${Math.round(value)}`
}

function formatFen(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '¥0.00'
  return `¥${(value / 100).toFixed(2)}`
}
