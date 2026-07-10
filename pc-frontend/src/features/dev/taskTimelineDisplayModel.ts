import { coverageLabels } from './taskTimelineModel'
import type { TaskTimelineModel, TaskTimelineStage, TimelineItem } from './taskTimelineModel'
import type { TaskTone } from './types'

export type PrimaryTimelineBlock =
  | { type: 'item'; item: TimelineItem }
  | { type: 'commands'; id: string; items: TimelineItem[] }

export interface TaskTimelineDisplayOptions {
  completed?: boolean
  hideAssistantReplies?: boolean
  hideCommands?: boolean
}

export function taskTimelineHasVisibleDetails(model: TaskTimelineModel, options: TaskTimelineDisplayOptions = {}) {
  return buildTimelineDisplay(model, options).hasVisibleTimeline
}

export function buildTimelineDisplay(model: TaskTimelineModel, {
  completed = false,
  hideAssistantReplies = false,
  hideCommands = false,
}: TaskTimelineDisplayOptions) {
  const rawDisplayItems = hideAssistantReplies ? model.items.filter((item) => !isAssistantTimelineItem(item)) : model.items
  const displayItems = rawDisplayItems
    .filter((item) => !isCurrentStageSourceItem(model.stage, item))
    .filter((item) => !isRedundantTerminalSummary(item, model.stage.tone))
  const grouped = groupTimelineItems(displayItems, completed, model.stage.tone)
  const visibleGrouped = {
    ...grouped,
    connection: shouldShowConnectionDetails(model) ? grouped.connection : [],
  }
  const primaryItems = hideCommands ? visibleGrouped.primary.filter((item) => !isCommandTimelineItem(item)) : visibleGrouped.primary
  const primaryBlocks = groupPrimaryTimelineBlocks(primaryItems)
  const hasApprovalItem = primaryItems.some((item) => item.kind === 'approval' && item.tone === 'approval')
  const showStageAtTop = model.stage.key === 'approval' && !hasApprovalItem
  const showDiagnosticDetails = shouldShowDiagnosticDetails(model, visibleGrouped.connection)
  const showCoverageInDiagnostics = showDiagnosticDetails && hasActiveCoverage(model)
  const diagnosticCount = showDiagnosticDetails
    ? model.diagnostics.length + (showCoverageInDiagnostics ? 1 : 0)
    : 0
  const hasVisibleTimeline = showStageAtTop
    || primaryBlocks.length > 0
    || diagnosticCount > 0
    || visibleGrouped.connection.length > 0
    || visibleGrouped.summary.length > 0
  return {
    grouped: visibleGrouped,
    primaryBlocks,
    showStageAtTop,
    showDiagnosticDetails,
    showCoverageInDiagnostics,
    diagnosticCount,
    hasVisibleTimeline,
  }
}

function shouldShowConnectionDetails(model: TaskTimelineModel) {
  if (model.stage.stuck || model.stage.tone === 'failed') return true
  return [
    'recovery',
    'recovering',
    'recovery-timeout',
    'resume-required',
    'timeout',
    'tool-timeout',
  ].includes(model.stage.key)
}

export function diagnosticFoldTitle(model: TaskTimelineModel) {
  const hasProblemDiagnostic = model.diagnostics.some((diagnostic) => diagnostic.tone === 'failed')
  return model.stage.stuck || model.stage.tone === 'failed' || hasProblemDiagnostic ? '诊断' : '提示'
}

export function isAssistantTimelineItem(item: TimelineItem): boolean {
  return item.event?.type === 'assistant_message'
    || item.event?.type === 'assistant_chunk'
    || (item.message as Record<string, unknown> | undefined)?.assistant_progress_event === true
}

function groupPrimaryTimelineBlocks(items: TimelineItem[]): PrimaryTimelineBlock[] {
  const blocks: PrimaryTimelineBlock[] = []
  let commandItems: TimelineItem[] = []

  const flushCommands = () => {
    if (!commandItems.length) return
    blocks.push({
      type: 'commands',
      id: `commands-${commandItems[0].id}-${commandItems.length}`,
      items: commandItems,
    })
    commandItems = []
  }

  for (const item of items) {
    if (isCommandTimelineItem(item)) {
      commandItems.push(item)
      continue
    }
    flushCommands()
    blocks.push({ type: 'item', item })
  }
  flushCommands()
  return blocks
}

function shouldShowDiagnosticDetails(model: TaskTimelineModel, connectionItems: TimelineItem[]) {
  if (model.diagnostics.length > 0) return true
  if (model.stage.stuck) return connectionItems.length > 0 || hasActiveCoverage(model)
  if (model.stage.tone === 'failed') return connectionItems.length > 0
  return false
}

function hasActiveCoverage(model: TaskTimelineModel) {
  return coverageLabels(model.coverage).some((item) => item.active)
}

function isCommandTimelineItem(item: TimelineItem): boolean {
  return Boolean(item.process?.commandText)
}

function groupTimelineItems(items: TimelineItem[], completed: boolean, terminalTone: TaskTone) {
  const grouped: { primary: TimelineItem[]; connection: TimelineItem[]; summary: TimelineItem[] } = {
    primary: [],
    connection: [],
    summary: [],
  }
  for (const item of items) {
    if (isRedundantTerminalSummary(item, terminalTone)) continue
    if (isTerminalRuntimeDetail(item, terminalTone)) {
      grouped.connection.push(item)
      continue
    }
    if (completed && isCompletedTechnicalItem(item)) grouped.connection.push(item)
    else if (isConnectionItem(item)) grouped.connection.push(item)
    else if (isSummaryItem(item)) grouped.summary.push(item)
    else grouped.primary.push(item)
  }
  return grouped
}

function isRedundantTerminalSummary(item: TimelineItem, terminalTone: TaskTone) {
  return (terminalTone === 'failed' || terminalTone === 'canceled')
    && item.event?.type === 'runtime_summary'
}

function isTerminalRuntimeDetail(item: TimelineItem, terminalTone: TaskTone) {
  return (terminalTone === 'failed' || terminalTone === 'canceled')
    && item.event?.type === 'runtime_status'
}

function isCurrentStageSourceItem(stage: TaskTimelineStage, item: TimelineItem) {
  if (item.event?.type !== 'runtime_status') return false
  const phase = String(item.event.phase ?? '').trim().toLowerCase()
  const status = String(item.event.status ?? '').trim().toLowerCase()
  if (stage.key === 'timeout') return phase === 'pc_cli_no_output_timeout'
  if (stage.key === 'tool-timeout') return phase === 'pc_tool_result_timeout'
  if (stage.key === 'recovery-timeout') return phase === 'pc_cli_recovery_timeout'
  if (stage.key === 'resume-required') return phase === 'resume_required'
  if (stage.key === 'dispatch') return phase === 'pc_dispatched'
  if (stage.key === 'recovery' || stage.key === 'recovering') return phase === 'pc_cli_communication_recovering' || phase === 'connection_recovering'
  if (stage.key === 'server-update') return phase === 'server_updating'
  if (stage.key === 'win-update') return phase === 'win_client_updating'
  if (stage.key === 'latest') return item.event?.type === 'runtime_status'
  if (stage.key === 'finished' && stage.tone === 'failed') return phase === 'failed' || status === 'error'
  if (stage.key === 'finished' && stage.tone === 'canceled') return phase === 'canceled' || status === 'canceled'
  return false
}

function isCompletedTechnicalItem(item: TimelineItem) {
  if (item.kind === 'heartbeat') return true
  const text = [item.title, item.detail, item.meta].filter(Boolean).join(' ')
  return /(同步 PC 构建产物|构建产物失败|安装按钮|AI CLI 正在处理中|正在处理中|累计前面全部流程总耗时)/i.test(text)
}

function isConnectionItem(item: TimelineItem) {
  if (item.kind === 'heartbeat') return true
  if (item.kind === 'node') return true
  if (item.event?.type === 'pc_dispatch_started') return true
  const phase = String(item.event?.phase ?? '')
  const text = [item.title, item.detail, item.meta, phase].filter(Boolean).join(' ')
  if (/^(Codex|Claude|Copilot|Gemini|AI CLI)$/i.test(text.trim())) return true
  return /(执行权|会话隔离|PC 节点|pc 节点|绑定的 PC|正在直连|正在连接|快速检查|巡检|派发|确认接收|等待 .*CLI|正在处理中|重连|更新升级|恢复通信|自动恢复)/i.test(text)
}

function isSummaryItem(item: TimelineItem) {
  if (item.event?.type === 'usage' || item.event?.type === 'runtime_summary') return true
  const text = [item.title, item.detail].filter(Boolean).join(' ')
  return item.kind === 'status' && item.tone !== 'failed' && /(用量统计|运行完成|任务完成)/.test(text)
}
