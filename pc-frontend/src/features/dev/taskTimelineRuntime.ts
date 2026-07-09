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
  const value = clean(text)
  return isStatusEchoProgressText(value) ? '' : value
}

export function isStatusEchoProgressText(value: string): boolean {
  const text = value.replace(/\s+/g, ' ').trim()
  return /我正在恢复本轮任务连接|先确认本地会话状态，再接上后续步骤|我已经收到部分回复，正在等待收尾|我正在等待本机 AI 输出|我正在连接本机节点/.test(text)
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
  const latestRuntime = latestRuntimeStatusItem(model.items)
  const latestRuntimeStatus = clean(latestRuntime?.event?.status ?? '').toLowerCase()
  const latestRuntimePhaseName = clean(latestRuntime?.event?.phase ?? '').toLowerCase()
  const heartbeatWaitedSeconds = heartbeatWaitSeconds(model.lastHeartbeat)
  const heartbeatWaitedLong = heartbeatWaitedSeconds !== null && heartbeatWaitedSeconds >= 60
  const endedByRuntimeProblem = latestRuntimeStatus === 'error'
    || latestRuntimePhaseName === 'failed'
    || latestRuntimePhaseName === 'canceled'
    || latestRuntimePhaseName === 'resume_required'
  const recovering = !!latestRuntimePhase(model.items, 'pc_cli_communication_recovering')
    && !latestRuntimePhase(model.items, 'pc_cli_recovery_timeout')
    && !coverage.finalReply
  if (!maintenance && !recovering && heartbeatWaitedLong && coverage.heartbeat && !coverage.finalReply && !coverage.command && !coverage.toolResult && !coverage.assistantEvent) {
    diagnostics.push({
      tone: 'failed',
      title: '只收到等待状态',
      detail: '后端已经派发任务或正在等待 AI CLI，但还没有收到公开的命令、文件修改、工具结果或回复片段。通常卡在 CLI 启动、节点输出、网络连接或旧节点进程。'
    })
  }
  if (coverage.finalReply && !endedByRuntimeProblem && !coverage.command && !coverage.fileChange && !coverage.testRun) {
    diagnostics.push({
      tone: 'muted',
      title: '本轮无公开工具过程',
      detail: '最终回复已出现，但没有命令、文件修改或测试事件。需要改项目时，应继续检查是否真的产生了命令、文件或测试过程。'
    })
  }
  return diagnostics
}

function heartbeatWaitSeconds(item: TimelineItem | undefined): number | null {
  const value = clean(item?.meta ?? '').match(/([0-9]+)\s*s/i)?.[1]
  if (!value) return null
  const seconds = Number(value)
  return Number.isFinite(seconds) ? seconds : null
}

export function removeMatchingShellCommandEcho(items: TimelineItem[], item: TimelineItem): void {
  const event = item.event
  if (event?.type !== 'tool_call' || clean(event.tool ?? '') !== 'shell') return
  const command = clean(event.args?.command ?? '')
  if (!command) return
  for (let index = items.length - 1; index >= 0 && index >= items.length - 5; index--) {
    const previous = items[index]
    if (previous.event) continue
    const text = clean(previous.detail ?? previous.title)
    if (isShellCommandEcho(text) && shellEchoMatchesCommand(text, command)) {
      items.splice(index, 1)
      return
    }
  }
}

export function isAssistantEvent(event: ToolEvent | null): boolean {
  const type = clean(event?.type ?? '')
  return type === 'assistant_message' || type === 'assistant_chunk'
}

export function isShellCommandEcho(text: string): boolean {
  return /^AI\s*执行命令\s*[：:]/i.test(text)
}

function shellEchoMatchesCommand(text: string, command: string): boolean {
  const echoCommand = clean(text.replace(/^AI\s*执行命令\s*[：:]\s*/i, ''))
  if (!echoCommand) return false
  const left = normalizeCommandForCompare(echoCommand)
  const right = normalizeCommandForCompare(command)
  return left === right || left.includes(right) || right.includes(left)
}

function normalizeCommandForCompare(value: string): string {
  return value.replace(/\s+/g, ' ').trim()
}

function itemId(message: ChatMessage, index: number): string {
  return clean(message.id ?? '') || `timeline-${index}`
}
