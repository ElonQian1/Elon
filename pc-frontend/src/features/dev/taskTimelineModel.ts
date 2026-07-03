import { clean } from '../../lib/utils'
import {
  messageText,
  parseToolEvent,
  runtimeStatusLabel,
  toolEventSummary,
  toolEventTitle,
  usageEventSummary,
} from './devTaskUtils'
import type { ChatMessage, TaskTone, ToolEvent } from './types'

export type TimelineItemKind =
  | 'node'
  | 'codex'
  | 'tool'
  | 'test'
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
  coverage: TaskTimelineCoverage
  diagnostics: TaskTimelineDiagnostic[]
}

export interface TaskTimelineCoverage {
  dispatch: boolean
  heartbeat: boolean
  toolCall: boolean
  command: boolean
  testRun: boolean
  fileChange: boolean
  toolResult: boolean
  usage: boolean
  assistantEvent: boolean
  finalReply: boolean
}

export interface TaskTimelineDiagnostic {
  tone: TaskTone
  title: string
  detail: string
}

export function buildTaskTimeline(messages: ChatMessage[], finalMessage?: ChatMessage): TaskTimelineModel {
  const items: TimelineItem[] = []
  const seenText = new Set<string>()
  const finalText = finalMessage ? normalizedProgressText(messageText(finalMessage)) : ''
  let lastHeartbeat: TimelineItem | undefined
  let latestHeartbeat: TimelineItem | undefined
  let heartbeatCount = 0
  const coverage: TaskTimelineCoverage = {
    dispatch: false,
    heartbeat: false,
    toolCall: false,
    command: false,
    testRun: false,
    fileChange: false,
    toolResult: false,
    usage: false,
    assistantEvent: false,
    finalReply: !!finalText,
  }

  const flushHeartbeat = () => {
    if (!lastHeartbeat) return
    items.push(lastHeartbeat)
    lastHeartbeat = undefined
  }

  messages.forEach((message, index) => {
    const text = normalizedProgressText(messageText(message))
    if (!text) return
    const parsedEvent = parseToolEvent(text)
    const echoText = eventTextForEcho(parsedEvent) || text
    if (isFinalAnswerEcho(echoText, finalText)) return
    if (isAssistantOutputEvent(parsedEvent)) {
      coverage.assistantEvent = true
      return
    }

    const heartbeat = parseHeartbeat(text, message, index)
    if (heartbeat) {
      heartbeatCount += 1
      coverage.heartbeat = true
      lastHeartbeat = heartbeat
      latestHeartbeat = heartbeat
      return
    }

    flushHeartbeat()
    const event = parsedEvent
    const item = event
      ? itemFromEvent(event, message, index)
      : itemFromText(text, message, index)
    if (event) markCoverage(coverage, event)

    const uniqueKey = event
      ? `${event.type}:${clean(event.tool ?? '')}:${clean(event.status ?? '')}:${clean(event.approval_id ?? '')}:${clean(event.message ?? '')}`
      : text
    if (!event && seenText.has(uniqueKey)) return
    seenText.add(uniqueKey)
    items.push(item)
  })

  flushHeartbeat()

  const model = {
    items,
    visibleStepCount: items.length,
    heartbeatCount,
    lastHeartbeat: latestHeartbeat,
    coverage,
  }
  return {
    ...model,
    diagnostics: buildDiagnostics(model),
  }
}

export function timelineSummary(model: TaskTimelineModel, taskId: string, shortTaskId: string): string {
  const parts: string[] = []
  if (model.visibleStepCount > 0) parts.push(`${model.visibleStepCount} 步过程`)
  if (model.heartbeatCount > 1) parts.push(`合并 ${model.heartbeatCount} 条等待状态`)
  if (model.coverage.command) parts.push('有命令')
  if (model.coverage.fileChange) parts.push('有文件修改')
  if (model.coverage.testRun) parts.push('有测试/构建')
  if (model.coverage.heartbeat && !model.coverage.command && !model.coverage.toolResult && !model.coverage.assistantEvent) {
    parts.push('未收到 CLI 输出')
  }
  if (shortTaskId || taskId) parts.push(shortTaskId || taskId)
  return parts.join(' · ')
}

function itemFromEvent(event: ToolEvent, message: ChatMessage, index: number): TimelineItem {
  const type = clean(event.type)
  if (type === 'pc_dispatch_started') {
    const cli = clean(event.cli ?? 'Codex')
    const agentId = clean(event.agent_id ?? '')
    const cwdConfigured = Boolean(event.cwd_configured)
    return {
      id: itemId(message, index),
      kind: 'node',
      tone: 'running',
      title: '已派发到 PC 节点',
      detail: cwdConfigured ? `等待 ${cli} CLI 输出` : `等待 ${cli} CLI 确认`,
      meta: agentId ? shortNode(agentId) : '',
      message,
      event,
      compact: true,
    }
  }

  if (type === 'runtime_status') {
    const phase = clean(event.phase ?? '').toLowerCase()
    const label = runtimeStatusLabel(phase)
    const runtime = clean(event.runtime ?? '')
    const turn = Number(event.turn ?? 0)
    const eventMessage = clean(event.message ?? '')
    if (phase === 'pc_cli_no_output_timeout') {
      return {
        id: itemId(message, index),
        kind: 'status',
        tone: 'failed',
        title: label.title,
        detail: eventMessage || label.body,
        meta: runtime,
        message,
        event,
      }
    }
    return {
      id: itemId(message, index),
      kind: 'codex',
      tone: label.tone,
      title: eventMessage || label.title,
      detail: eventMessage ? undefined : label.body,
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

  if (type === 'usage') {
    return {
      id: itemId(message, index),
      kind: 'status',
      tone: 'done',
      title: '用量统计',
      detail: usageEventSummary(event),
      meta: clean(event.model ?? ''),
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
  const isShell = clean(event.tool ?? '') === 'shell'
  const validation = isShell && eventLooksLikeValidation(event)
  return {
    id: itemId(message, index),
    kind: validation ? 'test' : 'tool',
    tone: failed ? 'failed' : isResult ? 'done' : 'running',
    title: toolEventTitle(event),
    detail: toolEventSummary(event, 140),
    message,
    event,
  }
}

function markCoverage(coverage: TaskTimelineCoverage, event: ToolEvent) {
  const type = clean(event.type)
  const tool = clean(event.tool ?? '')
  if (type === 'pc_dispatch_started') coverage.dispatch = true
  if (type === 'runtime_status' && clean(event.phase ?? '') === 'pc_dispatched') coverage.dispatch = true
  if (type === 'tool_call') {
    coverage.toolCall = true
    if (tool === 'shell') coverage.command = true
    if (tool === 'shell' && eventLooksLikeValidation(event)) coverage.testRun = true
    if (tool === 'file_change') coverage.fileChange = true
  }
  if (type === 'tool_result') {
    coverage.toolResult = true
    if (tool === 'shell') coverage.command = true
    if (tool === 'shell' && eventLooksLikeValidation(event)) coverage.testRun = true
    if (tool === 'file_change') coverage.fileChange = true
  }
  if (type === 'usage') coverage.usage = true
  if (type === 'assistant_message' || type === 'assistant_chunk') coverage.assistantEvent = true
}

function isAssistantOutputEvent(event: ToolEvent | null): boolean {
  return event?.type === 'assistant_message' || event?.type === 'assistant_chunk'
}

function buildDiagnostics(model: Omit<TaskTimelineModel, 'diagnostics'>): TaskTimelineDiagnostic[] {
  const diagnostics: TaskTimelineDiagnostic[] = []
  const { coverage } = model
  if (coverage.heartbeat && !coverage.command && !coverage.toolResult && !coverage.assistantEvent) {
    diagnostics.push({
      tone: 'failed',
      title: '只收到等待状态',
      detail: '后端已经派发任务或正在等待 Codex，但还没有收到公开的命令、文件修改、工具结果或回复片段。通常卡在 CLI 启动、节点输出、网络连接或旧节点进程。'
    })
  }
  if (coverage.dispatch && !coverage.command && !coverage.assistantEvent && !coverage.finalReply) {
    diagnostics.push({
      tone: 'running',
      title: '已到 PC 节点',
      detail: '任务已进入本机节点链路；如果持续只有这一项，优先检查节点是否重连、Codex CLI 是否启动、sidecar 是否仍存活。'
    })
  }
  if (coverage.command && !coverage.toolResult) {
    diagnostics.push({
      tone: 'running',
      title: '命令已开始，等待结果',
      detail: '前端已捕获命令调用，但还没有对应工具结果。长时间停在这里时，通常是命令仍在执行或 CLI 没有 flush 完成事件。'
    })
  }
  if (coverage.finalReply && !coverage.command && !coverage.fileChange && !coverage.testRun) {
    diagnostics.push({
      tone: 'muted',
      title: '本轮像普通问答',
      detail: '最终回复已出现，但没有命令、文件修改或测试事件。普通问答这是正常的；涉及项目修改时应继续检查 Codex 是否真的执行了公开过程。'
    })
  }
  return diagnostics
}

export function coverageLabels(coverage: TaskTimelineCoverage): Array<{ key: keyof TaskTimelineCoverage; label: string; active: boolean }> {
  return [
    { key: 'dispatch', label: '派发', active: coverage.dispatch },
    { key: 'heartbeat', label: '等待', active: coverage.heartbeat },
    { key: 'command', label: '命令', active: coverage.command },
    { key: 'fileChange', label: '文件', active: coverage.fileChange },
    { key: 'testRun', label: '测试/构建', active: coverage.testRun },
    { key: 'assistantEvent', label: '回复片段', active: coverage.assistantEvent },
    { key: 'usage', label: '用量', active: coverage.usage },
    { key: 'finalReply', label: '最终回复', active: coverage.finalReply },
  ]
}

function eventLooksLikeValidation(event: ToolEvent): boolean {
  const command = clean(event.args?.command ?? '')
  const result = clean(event.result ?? '')
  return commandLooksLikeValidation(command) || resultLooksLikeValidation(result)
}

function commandLooksLikeValidation(command: string): boolean {
  const value = command.toLowerCase()
  return /\b(cargo|npm|pnpm|yarn|bun|pytest|gradle|go|mvn|ruff|eslint|tsc)\b/.test(value)
    && /\b(test|check|build|clippy|lint|typecheck|assemble|verify)\b/.test(value)
}

function resultLooksLikeValidation(result: string): boolean {
  const value = result.toLowerCase()
  return value.includes('test result:')
    || value.includes('finished `test`')
    || value.includes('finished `dev`')
    || value.includes('npm run build')
    || value.includes('vite')
    || value.includes('cargo check')
    || value.includes('cargo test')
    || value.includes('build successful')
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
  if (!text || !finalText) return false
  if (text === finalText) return true
  if (finalText.length < 40) return false
  const sample = finalText.slice(0, Math.min(60, finalText.length))
  return sample.length >= 40 && text.includes(sample)
}

function eventTextForEcho(event: ToolEvent | null): string {
  if (!event) return ''
  const type = clean(event.type)
  if (type === 'assistant_message' || type === 'assistant_chunk') return clean(event.text ?? '')
  return ''
}

function itemId(message: ChatMessage, index: number): string {
  return clean(message.id ?? '') || `timeline-${index}`
}

function extractNodeId(text: string): string {
  return clean(text.match(/node-[A-Za-z0-9_-]+(?:\.\.\.[A-Za-z0-9_-]+)?/)?.[0] ?? '')
}

function shortNode(value: string): string {
  const cleanValue = clean(value)
  if (cleanValue.length <= 18) return cleanValue
  return `${cleanValue.slice(0, 11)}...${cleanValue.slice(-6)}`
}

function shortText(text: string): string {
  if (text.length <= 34) return text
  return `${text.slice(0, 34)}...`
}
