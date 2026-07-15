import type { Message } from '../features/conversation/types'

export type ReplayIssueSeverity = 'error' | 'warning' | 'info'
export type ReplayFrameSource = 'start' | 'message' | 'event'

export interface ReplayRawEvent {
  seq?: number
  createdAt: string
  event: Record<string, unknown>
}

export interface ReplayCapture {
  version: 1
  id: string
  title: string
  description?: string
  source: 'golden' | 'snapshot' | 'import'
  projectId?: string
  channelId?: string
  conversationId: string
  taskId: string
  startedAt: string
  taskStatus: string
  taskError?: string
  messages: Message[]
  events: ReplayRawEvent[]
  hasMoreEvents?: boolean
  lastEventSeq?: number
}

export interface ReplayFrame {
  index: number
  atMs: number
  absoluteAt: string
  source: ReplayFrameSource
  sourceIndex: number
  messageCount: number
  eventCount: number
  title: string
  visualChange: boolean
}

export interface ReplayIssue {
  id: string
  severity: ReplayIssueSeverity
  title: string
  detail: string
  frameIndex?: number
}

export interface ReplayPreviewConfig {
  enabled: boolean
  captureId: string
  projectId: string
  channelId: string
  conversationId: string
  taskId: string
  frame: number | null
  speed: number
  expandTools: boolean
  filmstrip: boolean
}

interface TimedItem {
  source: Exclude<ReplayFrameSource, 'start'>
  sourceIndex: number
  at: number
  tieBreaker: number
  title: string
}

const TERMINAL_STATUSES = new Set(['done', 'completed', 'success', 'failed', 'error', 'canceled', 'cancelled', 'interrupted'])

export function parseReplayPreviewConfig(search: string): ReplayPreviewConfig {
  const params = new URLSearchParams(search)
  const source = clean(params.get('source'))
  const requestedFrame = Number(params.get('frame'))
  const requestedSpeed = Number(params.get('speed'))
  return {
    enabled: source === 'replay' || params.get('replay') === '1',
    captureId: clean(params.get('capture')) || 'data-root-failure',
    projectId: clean(params.get('project') ?? params.get('projectId')) || 'elon-self',
    channelId: clean(params.get('channel') ?? params.get('channelId')),
    conversationId: clean(params.get('conversation') ?? params.get('conversationId')),
    taskId: clean(params.get('task') ?? params.get('taskId')),
    frame: Number.isInteger(requestedFrame) && requestedFrame >= 0 ? requestedFrame : null,
    speed: [0.25, 1, 4].includes(requestedSpeed) ? requestedSpeed : 1,
    expandTools: params.get('tools') === '1' || params.get('expand') === '1',
    filmstrip: params.get('filmstrip') !== '0',
  }
}

export function buildReplayFrames(capture: ReplayCapture): ReplayFrame[] {
  const startedAt = captureStartMs(capture)
  const timed: TimedItem[] = [
    ...capture.messages.map((message, sourceIndex) => ({
      source: 'message' as const,
      sourceIndex,
      at: timestampMs(message.created_at, startedAt),
      tieBreaker: sourceIndex,
      title: messageFrameTitle(message),
    })),
    ...capture.events.map((rawEvent, sourceIndex) => ({
      source: 'event' as const,
      sourceIndex,
      at: timestampMs(rawEvent.createdAt, startedAt),
      tieBreaker: Number.isFinite(rawEvent.seq) ? Number(rawEvent.seq) : sourceIndex,
      title: eventFrameTitle(rawEvent.event),
    })),
  ].sort((left, right) => left.at - right.at
    || sourcePriority(left.source) - sourcePriority(right.source)
    || left.tieBreaker - right.tieBreaker)

  let messageCount = 0
  let eventCount = 0
  const frames: ReplayFrame[] = [{
    index: 0,
    atMs: 0,
    absoluteAt: new Date(startedAt).toISOString(),
    source: 'start',
    sourceIndex: -1,
    messageCount: 0,
    eventCount: 0,
    title: '任务开始',
    visualChange: false,
  }]

  for (const item of timed) {
    if (item.source === 'message') messageCount += 1
    else eventCount += 1
    frames.push({
      index: frames.length,
      atMs: Math.max(0, item.at - startedAt),
      absoluteAt: new Date(item.at).toISOString(),
      source: item.source,
      sourceIndex: item.sourceIndex,
      messageCount,
      eventCount,
      title: item.title,
      visualChange: item.source === 'message',
    })
  }
  return frames
}

export function replayMessagesAtFrame(capture: ReplayCapture, frame: ReplayFrame): Message[] {
  const ordered = capture.messages
    .map((message, sourceIndex) => ({ message, sourceIndex }))
    .sort((left, right) => timestampMs(left.message.created_at, 0) - timestampMs(right.message.created_at, 0)
      || left.sourceIndex - right.sourceIndex)
    .slice(0, frame.messageCount)
    .map(({ message }) => message)
  const terminalVisible = ordered.some((message) => messageKind(message) === 'ai_result')

  return ordered.map((message) => {
    if (messageTaskId(message) !== capture.taskId) return message
    const shared = {
      ...message,
      conversation_id: clean(message.conversation_id) || capture.conversationId,
      conversationId: clean(message.conversationId) || capture.conversationId,
    }
    if (!terminalVisible) {
      return {
        ...shared,
        task_status: 'running',
        taskStatus: 'running',
        task_error: undefined,
        taskError: undefined,
      }
    }
    return {
      ...shared,
      task_status: clean(message.task_status ?? message.taskStatus) || capture.taskStatus,
      taskStatus: clean(message.taskStatus ?? message.task_status) || capture.taskStatus,
      task_error: message.task_error ?? message.taskError ?? capture.taskError,
      taskError: message.taskError ?? message.task_error ?? capture.taskError,
    }
  })
}

export function selectReplayKeyFrames(frames: ReplayFrame[], maxFrames = 8): ReplayFrame[] {
  if (frames.length <= maxFrames) return frames
  const candidates = frames.filter((frame) => frame.index === 0 || frame.visualChange || frame.index === frames.length - 1)
  if (candidates.length <= maxFrames) return candidates
  const selected = new Map<number, ReplayFrame>()
  for (let index = 0; index < maxFrames; index += 1) {
    const position = Math.round(index * (candidates.length - 1) / (maxFrames - 1))
    const frame = candidates[position]
    if (frame) selected.set(frame.index, frame)
  }
  return [...selected.values()].sort((left, right) => left.index - right.index)
}

export function captureReplayIssues(capture: ReplayCapture, frames = buildReplayFrames(capture)): ReplayIssue[] {
  const issues: ReplayIssue[] = []
  if (capture.messages.length === 0) {
    issues.push(issue('capture-empty', 'error', '没有可回放消息', '快照未包含任何频道消息，无法还原用户看到的对话。'))
  }

  appendTimestampRegressionIssues(capture.messages.map((message) => message.created_at), '消息', issues)
  appendTimestampRegressionIssues(capture.events.map((event) => event.createdAt), '事件', issues)

  const sequence = capture.events.map((event) => event.seq).filter((value): value is number => Number.isFinite(value))
  for (let index = 1; index < sequence.length; index += 1) {
    if (sequence[index] !== sequence[index - 1] + 1) {
      issues.push(issue('event-sequence-gap', 'warning', '事件序号不连续', `事件 ${sequence[index - 1]} 之后直接出现 ${sequence[index]}，回放可能缺帧。`))
      break
    }
  }

  for (let index = 1; index < capture.messages.length; index += 1) {
    const previous = capture.messages[index - 1]
    const current = capture.messages[index]
    if (messageKind(previous) === messageKind(current)
      && clean(previous.content ?? previous.text) === clean(current.content ?? current.text)
      && clean(current.content ?? current.text)) {
      issues.push(issue(`duplicate-message-${index}`, 'warning', '连续消息内容重复', `第 ${index + 1} 条消息与上一条完全相同。`))
    }
  }

  const terminal = TERMINAL_STATUSES.has(clean(capture.taskStatus).toLowerCase())
  const hasResult = capture.messages.some((message) => messageKind(message) === 'ai_result')
  if (terminal && !hasResult) {
    issues.push(issue('terminal-without-result', 'error', '终态缺少最终消息', `任务状态为 ${capture.taskStatus}，但消息流里没有 ai_result。`))
  }
  if (capture.hasMoreEvents) {
    issues.push(issue('capture-truncated', 'warning', '事件快照未完整', '服务器表示仍有后续事件，请继续分页录制后再做视觉验收。'))
  }

  const firstVisible = frames.find((frame) => frame.visualChange)
  if (firstVisible && firstVisible.atMs > 1200) {
    issues.push(issue('slow-first-visible-frame', 'warning', '首个可见反馈过慢', `用户发送后 ${formatDuration(firstVisible.atMs)} 才出现首个可见变化。`, firstVisible.index))
  }
  const rapidCount = frames.slice(1).filter((frame, index) => frame.atMs - frames[index].atMs < 100).length
  if (rapidCount >= 3) {
    issues.push(issue('rapid-startup-events', 'info', '启动事件过于密集', `${rapidCount} 个相邻事件间隔不足 100ms；逐帧播放会自动延长这些帧便于检查。`))
  }
  return issues
}

export function replayFrameDelay(current: ReplayFrame, next: ReplayFrame, speed: number): number {
  const realDelay = Math.max(0, next.atMs - current.atMs)
  return Math.round(Math.min(3000, Math.max(500, realDelay)) / Math.max(0.25, speed))
}

function appendTimestampRegressionIssues(values: Array<unknown>, label: string, issues: ReplayIssue[]) {
  for (let index = 1; index < values.length; index += 1) {
    const previous = timestampMs(values[index - 1], 0)
    const current = timestampMs(values[index], 0)
    if (current < previous) {
      issues.push(issue(`${label}-timestamp-regression-${index}`, 'warning', `${label}时间倒退`, `第 ${index + 1} 条${label}早于上一条，回放已按时间重排。`))
      return
    }
  }
}

function captureStartMs(capture: ReplayCapture): number {
  const candidates = [
    timestampMs(capture.startedAt, Number.POSITIVE_INFINITY),
    ...capture.messages.map((message) => timestampMs(message.created_at, Number.POSITIVE_INFINITY)),
    ...capture.events.map((event) => timestampMs(event.createdAt, Number.POSITIVE_INFINITY)),
  ].filter(Number.isFinite)
  return candidates.length > 0 ? Math.min(...candidates) : Date.now()
}

function timestampMs(value: unknown, fallback: number): number {
  const parsed = Date.parse(String(value ?? ''))
  return Number.isFinite(parsed) ? parsed : fallback
}

function sourcePriority(source: TimedItem['source']): number {
  return source === 'message' ? 0 : 1
}

function messageFrameTitle(message: Message): string {
  const kind = messageKind(message)
  if (kind === 'ai_task') return '用户发起任务'
  if (kind === 'ai_result') return '最终回复到达'
  const event = parseEvent(message.content ?? message.text)
  return event ? eventFrameTitle(event) : kind === 'ai_progress' ? 'AI 公开进展' : `消息：${kind || 'unknown'}`
}

function eventFrameTitle(event: Record<string, unknown>): string {
  const type = clean(event.type)
  const message = clean(event.message)
  if (message) return message.length > 46 ? `${message.slice(0, 46)}…` : message
  if (type === 'tool_call') return `开始运行 ${clean(event.tool) || '工具'}`
  if (type === 'tool_result') return `${clean(event.tool) || '工具'}返回结果`
  if (type === 'file_change') return '文件修改'
  if (type === 'runtime_summary') return '运行摘要'
  return type || '原始事件'
}

function parseEvent(value: unknown): Record<string, unknown> | null {
  const text = clean(value)
  if (!text.startsWith('{')) return null
  try {
    return JSON.parse(text) as Record<string, unknown>
  } catch {
    return null
  }
}

function messageKind(message: Message): string {
  return clean(message.kind ?? message.role ?? message.message_kind).toLowerCase()
}

function messageTaskId(message: Message): string {
  return clean(message.task_id ?? message.taskId)
}

function issue(id: string, severity: ReplayIssueSeverity, title: string, detail: string, frameIndex?: number): ReplayIssue {
  return { id, severity, title, detail, frameIndex }
}

function clean(value: unknown): string {
  return String(value ?? '').trim()
}

function formatDuration(milliseconds: number): string {
  if (milliseconds < 1000) return `${milliseconds}ms`
  return `${(milliseconds / 1000).toFixed(1)}s`
}
