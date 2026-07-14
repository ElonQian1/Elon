import { useCapabilityGaps } from './useCapabilityGaps'
import type { CapabilityGapDocument, CapabilityGapStatus } from './types'
import styles from './UiCapabilityGapPanel.module.css'

interface UiCapabilityGapPanelProps {
  sessionId?: string
}

export function UiCapabilityGapPanel({ sessionId }: UiCapabilityGapPanelProps) {
  const state = useCapabilityGaps(sessionId)
  const gap = state.gaps[0]
  return (
    <section className={styles.panel} aria-label="Codex 平台能力升级">
      <header>
        <div>
          <strong>Codex 平台能力</strong>
          <span>当前一龙项目本地 Git 工作区专用</span>
        </div>
        <button type="button" disabled={!sessionId || state.loading} onClick={() => { void state.refresh() }}>
          {state.loading ? '读取中…' : '刷新'}
        </button>
      </header>
      {!gap && !state.error && (
        <p className={styles.ready}>尚无平台能力缺口；UI 请求将直接使用真实渲染、拟合和源码写回工具。</p>
      )}
      {gap && <GapStatus gap={gap} />}
      {state.error && <p className={styles.error}>{state.error}</p>}
    </section>
  )
}

function GapStatus({ gap }: { gap: CapabilityGapDocument }) {
  const lastAttempt = gap.attempts[gap.attempts.length - 1]
  return (
    <div className={styles.gap}>
      <div className={styles.summary}>
        <span className={styles[statusTone(gap.status)]}>{statusLabel(gap.status)}</span>
        <small>第 {gap.upgradeRounds}/{gap.policy.maxUpgradeRounds} 轮</small>
        {lastAttempt?.version && <small>版本 {lastAttempt.version}</small>}
      </div>
      <div className={styles.capabilities}>
        {gap.missingCapabilities.map((capability) => <code key={capability}>{capability}</code>)}
      </div>
      <p>{statusDescription(gap)}</p>
      <small className={styles.resume}>完成后恢复：{gap.resumeTarget}</small>
      {lastAttempt?.commitId && <small className={styles.commit}>提交 {lastAttempt.commitId.slice(0, 12)}</small>}
      {gap.lastError && <p className={styles.error}>{gap.lastError}</p>}
    </div>
  )
}

function statusLabel(status: CapabilityGapStatus) {
  return ({
    APPROVED: '准备自动升级',
    UPGRADING: '正在升级平台',
    PUBLISHED: '已发布，正在复检',
    RESUMED: '已恢复原 UI 任务',
    HUMAN_REQUIRED: '自动流程已熔断',
  } as const)[status]
}

function statusTone(status: CapabilityGapStatus) {
  if (status === 'RESUMED') return 'success'
  if (status === 'HUMAN_REQUIRED') return 'danger'
  return 'active'
}

function statusDescription(gap: CapabilityGapDocument) {
  if (gap.status === 'APPROVED') return 'Codex 已确认是平台工具缺口，将自动修改本地源码并按项目规则发布。'
  if (gap.status === 'UPGRADING') return 'Codex 正在补齐平台能力；不会把临时绕过当作 UI 任务完成。'
  if (gap.status === 'PUBLISHED') return '新平台版本已经发布，Codex 必须重新执行原任务验证能力是否真正可用。'
  if (gap.status === 'RESUMED') return '平台复检通过，Codex 已回到最初的设计稿或 APK UI 修改任务。'
  return '重复失败、空发布或轮次预算触发了熔断，需要开发者判断产品或安全边界。'
}
