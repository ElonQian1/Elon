import type { NodeLifecycleReport } from './types'

export type LifecycleTone = 'ok' | 'warning' | 'danger' | 'neutral'

export interface LifecycleView {
  tone: LifecycleTone
  badge: string
  title: string
  detail: string
  actionLabel: string
  facts: Array<{ label: string; value: string }>
}

interface FallbackState {
  connected?: boolean
  loggedIn?: boolean
  lastEvent?: string
  online?: boolean
}

export function lifecycleView(report: NodeLifecycleReport | null | undefined, fallback: FallbackState = {}): LifecycleView {
  const state = String(report?.state ?? fallbackState(fallback))
  const tone = lifecycleTone(report?.severity, state)
  const title = lifecycleTitle(state, report)
  const detail = report?.summary || lifecycleDetail(state, report, fallback)
  const actionLabel = actionLabelFor(report?.recommended_action, state)
  const facts = lifecycleFacts(report, fallback)

  return {
    tone,
    badge: badgeFor(state, tone),
    title,
    detail,
    actionLabel,
    facts,
  }
}

function fallbackState(fallback: FallbackState): string {
  if (fallback.loggedIn === false) return 'needs_login'
  if (fallback.connected === true || fallback.online === true) return 'healthy'
  if (fallback.connected === false) return 'cloud_disconnected'
  return 'unknown'
}

function lifecycleTone(severity: string | undefined, state: string): LifecycleTone {
  if (severity === 'danger') return 'danger'
  if (severity === 'warning') return 'warning'
  if (severity === 'ok') return 'ok'
  if (state === 'stale_heartbeat' || state === 'unexpected_exit') return 'danger'
  if (state === 'healthy') return 'ok'
  if (state === 'unknown') return 'neutral'
  return 'warning'
}

function lifecycleTitle(state: string, report: NodeLifecycleReport | null | undefined): string {
  if (report?.previous_exit_kind === 'unexpected_exit' && state === 'healthy') {
    return '已重连，上次可能异常退出'
  }
  switch (state) {
    case 'healthy': return 'Win 端运行正常'
    case 'recovered_after_unexpected_exit': return '已重连，上次可能异常退出'
    case 'stale_heartbeat': return '疑似卡住'
    case 'cloud_disconnected': return '本机正常，云端未连接'
    case 'needs_login': return '等待绑定账号'
    default: return '状态待确认'
  }
}

function lifecycleDetail(
  state: string,
  report: NodeLifecycleReport | null | undefined,
  fallback: FallbackState,
): string {
  if (report?.previous_exit_reason) return report.previous_exit_reason
  if (fallback.lastEvent) return fallback.lastEvent
  switch (state) {
    case 'healthy': return '本机进程和云端连接都可用。'
    case 'stale_heartbeat': return '本机接口可访问，但生命周期心跳过期，建议重启 Win 端并查看任务日志。'
    case 'cloud_disconnected': return '本机进程已启动，但尚未连上云端。'
    case 'needs_login': return '需要先用当前账号注册或重新绑定本机节点。'
    default: return '暂无更具体的生命周期记录。'
  }
}

function actionLabelFor(action: string | undefined, state: string): string {
  switch (action) {
    case 'restart_client': return '重启 Win 端'
    case 'review_task_recovery': return '查看任务恢复'
    case 'review_previous_session': return '查看上次会话'
    case 'wait_or_reconnect': return '等待或重新检测'
    case 'login': return '绑定账号'
    case 'none': return '无需处理'
    default:
      if (state === 'cloud_disconnected') return '重新检测'
      if (state === 'needs_login') return '绑定账号'
      return '查看状态'
  }
}

function badgeFor(state: string, tone: LifecycleTone): string {
  if (state === 'healthy') return '正常'
  if (state === 'recovered_after_unexpected_exit') return '已恢复'
  if (state === 'stale_heartbeat') return '心跳过期'
  if (state === 'cloud_disconnected') return '断云'
  if (state === 'needs_login') return '待绑定'
  if (tone === 'ok') return '正常'
  if (tone === 'danger') return '异常'
  if (tone === 'warning') return '注意'
  return '未知'
}

function lifecycleFacts(report: NodeLifecycleReport | null | undefined, fallback: FallbackState) {
  const facts: Array<{ label: string; value: string }> = []
  const connected = report?.connected ?? fallback.connected ?? fallback.online
  const loggedIn = report?.logged_in ?? fallback.loggedIn
  facts.push({ label: '云端', value: connected === true ? '已连接' : connected === false ? '未连接' : '未知' })
  facts.push({ label: '账号', value: loggedIn === true ? '已绑定' : loggedIn === false ? '未绑定' : '未知' })
  if (typeof report?.heartbeat_age_ms === 'number') {
    facts.push({ label: '心跳', value: formatDuration(report.heartbeat_age_ms) })
  }
  if (report?.previous_exit_kind) {
    facts.push({ label: '上次退出', value: previousExitLabel(report.previous_exit_kind) })
  }
  const recoverable = (report?.active_task_count ?? 0) + (report?.sidecar_session_count ?? 0)
  if (recoverable > 0) {
    facts.push({ label: '可恢复', value: `${recoverable} 项` })
  }
  return facts
}

function previousExitLabel(kind: string): string {
  switch (kind) {
    case 'unexpected_exit': return '疑似异常'
    case 'planned_update': return '计划升级'
    case 'planned_uninstall': return '计划卸载'
    case 'planned_restart': return '计划重启'
    case 'user_closed': return '用户关闭'
    default: return kind
  }
}

function formatDuration(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return '未知'
  if (ms < 1000) return '刚刚'
  const seconds = Math.round(ms / 1000)
  if (seconds < 60) return `${seconds} 秒前`
  const minutes = Math.round(seconds / 60)
  if (minutes < 60) return `${minutes} 分钟前`
  const hours = Math.round(minutes / 60)
  return `${hours} 小时前`
}
