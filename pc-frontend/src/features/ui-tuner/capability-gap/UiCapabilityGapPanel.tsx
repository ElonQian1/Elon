import { useCapabilityGaps } from './useCapabilityGaps'
import type {
  CapabilityGapDocument,
  CapabilityGapStatus,
  CapabilityReadiness,
} from './types'
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
      {state.readiness && <ReadinessStatus readiness={state.readiness} />}
      {!gap && !state.error && state.readiness?.status === 'READY' && (
        <p className={styles.ready}>全部准备完成；UI 请求会直接使用真实渲染、拟合和源码写回工具。</p>
      )}
      {gap && <GapStatus gap={gap} />}
      {state.error && <p className={styles.error}>{state.error}</p>}
    </section>
  )
}

function ReadinessStatus({ readiness }: { readiness: CapabilityReadiness }) {
  if (readiness.status === 'READY') {
    return <span className={styles.readinessReady}>真实渲染工作流已就绪</span>
  }
  if (readiness.status === 'PLATFORM_GAP') {
    return (
      <div className={styles.readinessGap}>
        <strong>平台暂不支持</strong>
        <span>{readiness.missing.map(capabilityLabel).join('、')}</span>
      </div>
    )
  }
  return (
    <div className={styles.preparation}>
      <strong>还需完成 {readiness.preparationRequired.length} 项准备</strong>
      {readiness.preparationDetails.map((detail) => (
        <span key={`${detail.capability}:${detail.reason}`}>
          {capabilityLabel(detail.capability)}：{preparationLabel(detail.reason)}
        </span>
      ))}
      <small>下一步：{nextActionLabel(readiness.next)}</small>
    </div>
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

function capabilityLabel(capability: string) {
  return ({
    PROJECT_UI_PROFILE: '识别项目 UI 技术栈',
    NEW_SCREEN_BOOTSTRAP: '创建新页面骨架',
    REAL_ANDROID_RENDERER: '连接真实 Android 渲染器',
    LIVE_STYLE_PATCH: '实时修改样式',
    LOCAL_VISUAL_SOLVER: '本地视觉拟合',
    PERSISTENT_FIT_RUN: '持续自动拟合',
    PATCH_FREE_BUILD_VERIFY: '清除临时样式后构建验收',
  } as Record<string, string>)[capability] ?? capability
}

function preparationLabel(reason: string) {
  return ({
    PROJECT_UI_PROFILE_REQUIRED: '先导入桌面端 UI 任务',
    ANDROID_UI_TOOLKIT_SELECTION_REQUIRED: '请选择 Compose 或传统 View/XML',
    DEBUG_RUNTIME_NOT_CONNECTED: '安装并连接隔离的 Debug Runtime 包',
  } as Record<string, string>)[reason] ?? reason
}

function nextActionLabel(next: string) {
  return ({
    ui_import_desktop_task: '导入当前 UI 任务',
    ui_create_android_screen_scaffold: '创建项目匹配的 Android 页面骨架',
    ui_prepare_debug_runtime: '构建、安装并连接 Debug Runtime',
    ui_report_capability_gap: '交给 Codex 补齐平台能力',
    CONTINUE_UI_WORKFLOW: '继续 UI 工作流',
  } as Record<string, string>)[next] ?? next
}
