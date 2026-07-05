import { clean } from '../../lib/utils'
import type { ChatMessage, ToolEvent } from './types'
import type { TaskTimelineDiagnostic, TaskTimelineModel, TaskTimelineStageKey, TimelineItem } from './taskTimelineModel'

export function eventUniqueKey(event: ToolEvent, fallbackText: string): string {
  return [
    event.type,
    clean(event.id ?? ''),
    clean(event.tool ?? ''),
    clean(event.status ?? ''),
    clean(event.approval_id ?? ''),
    clean(event.text ?? ''),
    clean(event.message ?? ''),
    clean(event.result ?? ''),
    clean(event.args?.command ?? ''),
    fallbackText,
  ].join(':')
}

export function assistantItemFromText(
  text: string,
  message: ChatMessage,
  index: number,
  event?: ToolEvent,
): TimelineItem {
  const runtime = clean(
    event?.model_used
    ?? event?.model
    ?? message.model_used
    ?? message.modelUsed
    ?? message.cli_name
    ?? message.cliName
    ?? '',
  )
  return {
    id: itemId(message, index),
    kind: 'agent',
    tone: 'running',
    title: 'AI 回复片段',
    detail: text,
    meta: runtime,
    message,
    event,
  }
}

export function assistantProgressNoteText(message: ChatMessage, text: string): string {
  if ((message as Record<string, unknown>).assistant_progress_event !== true) return ''
  return clean(text)
}

export function latestRuntimePhase(items: TimelineItem[], phase: string): TimelineItem | undefined {
  return latestRuntimePhaseEntry(items, phase)?.item
}

export function latestRuntimePhaseEntry(
  items: TimelineItem[],
  phase: string,
): { item: TimelineItem; index: number } | undefined {
  for (let index = items.length - 1; index >= 0; index--) {
    const item = items[index]
    if (item.event?.type === 'runtime_status' && clean(item.event.phase ?? '').toLowerCase() === phase) {
      return { item, index }
    }
  }
  return undefined
}

export function latestNonHeartbeatIndex(items: TimelineItem[]): number {
  for (let index = items.length - 1; index >= 0; index--) {
    if (items[index].kind !== 'heartbeat') return index
  }
  return -1
}

export function latestRuntimeStatusItem(items: TimelineItem[]): TimelineItem | undefined {
  for (let index = items.length - 1; index >= 0; index--) {
    const item = items[index]
    if (item.event?.type === 'runtime_status') return item
  }
  return undefined
}

export function latestMaintenanceRuntimeStatus(items: TimelineItem[]): TimelineItem | undefined {
  const item = latestRuntimeStatusItem(items)
  const phase = clean(item?.event?.phase ?? '').toLowerCase()
  return item && isMaintenanceRuntimePhase(phase) ? item : undefined
}

export function isMaintenanceRuntimePhase(phase: string): boolean {
  return ['server_updating', 'win_client_updating', 'connection_recovering', 'resume_required'].includes(phase)
}

export function maintenanceStageKey(phase: string): TaskTimelineStageKey {
  if (phase === 'server_updating') return 'server-update'
  if (phase === 'win_client_updating') return 'win-update'
  if (phase === 'resume_required') return 'resume-required'
  return 'recovering'
}

export function buildTaskTimelineDiagnostics(
  model: Omit<TaskTimelineModel, 'diagnostics' | 'stage'>,
): TaskTimelineDiagnostic[] {
  const diagnostics: TaskTimelineDiagnostic[] = []
  const { coverage } = model
  const maintenance = latestMaintenanceRuntimeStatus(model.items)
  const recovering = !!latestRuntimePhase(model.items, 'pc_cli_communication_recovering')
    && !latestRuntimePhase(model.items, 'pc_cli_recovery_timeout')
    && !coverage.finalReply
  if (recovering) {
    diagnostics.push({
      tone: 'running',
      title: '通信正在自动恢复',
      detail: '服务器正在更新升级或 Win 端正在更新升级，通信临时中断，会自动恢复；请等待后续公开输出。'
    })
  }
  if (!maintenance && !recovering && coverage.heartbeat && !coverage.finalReply && !coverage.command && !coverage.toolResult && !coverage.assistantEvent) {
    diagnostics.push({
      tone: 'failed',
      title: '只收到等待状态',
      detail: '后端已经派发任务或正在等待 AI CLI，但还没有收到公开的命令、文件修改、工具结果或回复片段。通常卡在 CLI 启动、节点输出、网络连接或旧节点进程。'
    })
  }
  if (!maintenance && coverage.dispatch && !coverage.command && !coverage.assistantEvent && !coverage.finalReply) {
    diagnostics.push({
      tone: 'running',
      title: '已到 PC 节点',
      detail: '任务已进入本机节点链路；如果持续只有这一项，优先检查节点是否重连、AI CLI 是否启动、sidecar 是否仍存活。'
    })
  }
  if (coverage.command && !coverage.toolResult) {
    diagnostics.push({
      tone: 'running',
      title: '命令已开始，等待结果',
      detail: '前端已捕获命令调用，但还没有对应工具结果。长时间停在这里时，通常是命令仍在执行或 CLI 没有 flush 完成事件。'
    })
  }
  if (coverage.assistantEvent && !coverage.finalReply) {
    diagnostics.push({
      tone: 'muted',
      title: '已有公开回复',
      detail: 'AI CLI 的回复片段已经作为气泡显示；如果后面还有等待状态，通常是在同步用量、终态或后续工具事件。'
    })
  }
  if (coverage.finalReply && !coverage.command && !coverage.fileChange && !coverage.testRun) {
    diagnostics.push({
      tone: 'muted',
      title: '本轮无公开工具过程',
      detail: '最终回复已出现，但没有命令、文件修改或测试事件。需要改项目时，应继续检查是否真的产生了命令、文件或测试过程。'
    })
  }
  return diagnostics
}

function itemId(message: ChatMessage, index: number): string {
  return clean(message.id ?? '') || `timeline-${index}`
}
