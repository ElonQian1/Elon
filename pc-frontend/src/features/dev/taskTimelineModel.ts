import { clean } from '../../lib/utils'
import { messageText, parseToolEvent, runtimeStatusLabel } from './devTaskUtils'
import type { ChatMessage, TaskTone, ToolEvent } from './types'

export type TimelineItemKind =
  | 'node'
  | 'codex'
  | 'tool'
  | 'approval'
  | 'artifact'
  | 'status'
  | 'heartbeat'

export interface TimelineItem {
  id: string
  kind: TimelineItemKind
  tone: TaskTone
  title: string
  detail?: string
  meta?: string
  message?: ChatMessage
  event?: ToolEvent
  compact?: boolean
}

export interface TaskTimelineModel {
  items: TimelineItem[]
  visibleStepCount: number
  heartbeatCount: number
  lastHeartbeat?: TimelineItem
}

export function buildTaskTimeline(messages: ChatMessage[], finalMessage?: ChatMessage): TaskTimelineModel {
  const items: TimelineItem[] = []
  const seenText = new Set<string>()
  const finalText = finalMessage ? normalizedProgressText(messageText(finalMessage)) : ''
  let lastHeartbeat: TimelineItem | undefined
  let latestHeartbeat: TimelineItem | undefined
  let heartbeatCount = 0

  const flushHeartbeat = () => {
    if (!lastHeartbeat) return
    items.push(lastHeartbeat)
    lastHeartbeat = undefined
  }

  messages.forEach((message, index) => {
    const text = normalizedProgressText(messageText(message))
    if (!text) return
    if (!text.startsWith('{') && isFinalAnswerEcho(text, finalText)) return

    const heartbeat = parseHeartbeat(text, message, index)
    if (heartbeat) {
      heartbeatCount += 1
      lastHeartbeat = heartbeat
      latestHeartbeat = heartbeat
      return
    }

    flushHeartbeat()
    const event = parseToolEvent(text)
    const item = event
      ? itemFromEvent(event, message, index)
      : itemFromText(text, message, index)

    const uniqueKey = event
      ? `${event.type}:${clean(event.tool ?? '')}:${clean(event.status ?? '')}:${clean(event.approval_id ?? '')}:${clean(event.message ?? '')}`
      : text
    if (!event && seenText.has(uniqueKey)) return
    seenText.add(uniqueKey)
    items.push(item)
  })

  flushHeartbeat()

  return {
    items,
    visibleStepCount: items.length,
    heartbeatCount,
    lastHeartbeat: latestHeartbeat,
  }
}

export function timelineSummary(model: TaskTimelineModel, taskId: string, shortTaskId: string): string {
  const parts: string[] = []
  if (model.visibleStepCount > 0) parts.push(`${model.visibleStepCount} 步过程`)
  if (model.heartbeatCount > 1) parts.push(`合并 ${model.heartbeatCount} 条等待状态`)
  if (shortTaskId || taskId) parts.push(shortTaskId || taskId)
  return parts.join(' · ')
}

function itemFromEvent(event: ToolEvent, message: ChatMessage, index: number): TimelineItem {
  const type = clean(event.type)
  if (type === 'runtime_status') {
    const phase = clean(event.phase ?? '').toLowerCase()
    const label = runtimeStatusLabel(phase)
    const runtime = clean(event.runtime ?? '')
    const turn = Number(event.turn ?? 0)
    return {
      id: itemId(message, index),
      kind: 'codex',
      tone: label.tone,
      title: clean(event.message ?? '') || label.title,
      detail: clean(event.message ?? '') ? undefined : label.body,
      meta: [runtime, turn > 0 ? `第 ${turn} 轮` : ''].filter(Boolean).join(' · '),
      message,
      event,
      compact: true,
    }
  }

  if (type === 'runtime_summary') {
    const total = Number(event.total_tools ?? 0)
    const failed = Number(event.failed_tools ?? 0)
    const status = clean(event.status ?? '').toLowerCase()
    const canceled = ['canceled', 'cancelled', 'stopped'].includes(status)
    const failedRun = failed > 0 || ['error', 'failed'].includes(status)
    return {
      id: itemId(message, index),
      kind: 'codex',
      tone: canceled ? 'canceled' : failedRun ? 'failed' : 'done',
      title: clean(event.message ?? '') || `运行完成，${total} 个工具事件`,
      meta: failed > 0 ? `${failed} 个失败` : '',
      message,
      event,
      compact: true,
    }
  }

  if (type === 'tool_approval_required' || type === 'tool_approval_decision') {
    return {
      id: itemId(message, index),
      kind: 'approval',
      tone: type === 'tool_approval_required' ? 'approval' : 'done',
      title: type === 'tool_approval_required'
        ? `等待确认 ${clean(event.tool ?? 'tool')}`
        : `${clean(event.tool ?? 'tool')} 审批已处理`,
      meta: clean(event.approval_id ?? ''),
      message,
      event,
    }
  }

  const isResult = type === 'tool_result'
  const failed = isResult && clean(event.status ?? '').toLowerCase() === 'error'
  return {
    id: itemId(message, index),
    kind: 'tool',
    tone: failed ? 'failed' : isResult ? 'done' : 'running',
    title: `${isResult ? '完成' : '调用'} ${clean(event.tool ?? 'tool')}`,
    detail: isResult ? clean(event.result ?? '').slice(0, 120) : briefArgs(event.args),
    message,
    event,
  }
}

function itemFromText(text: string, message: ChatMessage, index: number): TimelineItem {
  const nodeId = extractNodeId(text)

  if (text.includes('会话隔离')) {
    return textItem(message, index, 'node', 'done', '本机会话隔离已启用', text, nodeId)
  }
  if (text.includes('执行权')) {
    return textItem(message, index, 'node', 'done', '已获得 PC 会话执行权', text, nodeId)
  }
  if (text.includes('正在连接 PC 节点')) {
    return textItem(message, index, 'node', 'running', '正在连接 PC 节点', text, nodeId)
  }
  if (text.includes('已派发到 PC 节点')) {
    return textItem(message, index, 'node', 'running', '已派发到 PC 节点', text, nodeId)
  }
  if (text.includes('同步 PC 构建产物')) {
    return textItem(message, index, 'artifact', 'running', '同步 PC 构建产物', text)
  }
  if (text.includes('没有发现 APK') || text.includes('不会生成安装按钮')) {
    return textItem(message, index, 'artifact', 'muted', '本轮没有 APK 产物', text)
  }
  if (text.includes('读取') || text.includes('看完项目规则') || text.includes('分析')) {
    return textItem(message, index, 'codex', 'running', shortText(text), text)
  }
  if (text.includes('任务完成')) {
    return textItem(message, index, 'status', 'done', '任务完成', text)
  }
  if (text.includes('失败') || text.includes('错误')) {
    return textItem(message, index, 'status', 'failed', shortText(text), text)
  }

  return textItem(message, index, 'status', 'running', shortText(text), text, nodeId)
}

function parseHeartbeat(text: string, message: ChatMessage, index: number): TimelineItem | null {
  if (!/正在处理中/.test(text) || !/已等待/.test(text)) return null
  const wait = text.match(/已等待\s*([0-9]+)\s*s/i)?.[1]
  const nodeId = extractNodeId(text)
  return {
    id: itemId(message, index),
    kind: 'heartbeat',
    tone: 'running',
    title: 'Codex 正在处理中',
    detail: nodeId ? `PC 节点 ${nodeId}` : undefined,
    meta: wait ? `已等待 ${wait}s` : undefined,
    message,
    compact: true,
  }
}

function textItem(
  message: ChatMessage,
  index: number,
  kind: TimelineItemKind,
  tone: TaskTone,
  title: string,
  detail?: string,
  meta?: string,
): TimelineItem {
  return {
    id: itemId(message, index),
    kind,
    tone,
    title,
    detail: detail && detail !== title ? detail : undefined,
    meta,
    message,
    compact: kind === 'node' || kind === 'artifact',
  }
}

function normalizedProgressText(text: string): string {
  return clean(text)
    .replace(/^Codex\s*\n\s*/i, '')
    .replace(/\s+/g, ' ')
    .trim()
}

function isFinalAnswerEcho(text: string, finalText: string): boolean {
  if (finalText.length < 40) return false
  if (text === finalText) return true
  const sample = finalText.slice(0, Math.min(60, finalText.length))
  return sample.length >= 40 && text.includes(sample)
}

function itemId(message: ChatMessage, index: number): string {
  return clean(message.id ?? '') || `timeline-${index}`
}

function extractNodeId(text: string): string {
  return clean(text.match(/node-[A-Za-z0-9_-]+(?:\.\.\.[A-Za-z0-9_-]+)?/)?.[0] ?? '')
}

function shortText(text: string): string {
  if (text.length <= 34) return text
  return `${text.slice(0, 34)}...`
}

function briefArgs(args: unknown): string {
  if (!args) return ''
  if (typeof args === 'string') return args.slice(0, 80)
  const obj = args as Record<string, unknown>
  const value = obj.command ?? obj.path ?? obj.file ?? obj.query ?? obj.input ?? obj.content ?? ''
  return clean(value).slice(0, 80)
}
