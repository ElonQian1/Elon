/** 对应 pc_app_dev_tasks.js 的纯业务逻辑层 */
import { clean } from '../../lib/utils'
import { displayMessageContentOrAttachment } from '../../lib/messageDisplay'
import type { TaskState, TaskTone, ToolEvent, ApprovalState, ChatMessage, TaskContext } from './types'

const DONE_TASK_STATUSES = new Set(['done', 'completed', 'success', 'succeeded', 'finished'])
const FAILED_TASK_STATUSES = new Set(['failed', 'error'])
const CANCELED_TASK_STATUSES = new Set(['canceled', 'cancelled', 'interrupted', 'stopped'])

export function messageKind(msg: ChatMessage): string {
  return clean(msg.kind ?? msg.role ?? msg.message_kind).toLowerCase()
}

export function messageText(msg: ChatMessage): string {
  return displayMessageContentOrAttachment(msg.content ?? msg.text ?? msg.message)
}

export function taskResultDisplayText(msg: ChatMessage): string {
  const content = messageText(msg)
  if (content) return content

  const taskError = clean(msg.task_error ?? msg.taskError ?? '')
  if (taskError) return taskError

  const status = clean(msg.task_status ?? msg.taskStatus ?? '').toLowerCase()
  if (FAILED_TASK_STATUSES.has(status)) return '任务失败，未收到详细错误。'
  if (CANCELED_TASK_STATUSES.has(status)) return '任务已停止。'
  return ''
}

export function taskRequestLooksMarkdown(value: string): boolean {
  return /!\[[^\]]*]\([^)]+\)/.test(value)
    || /\[[^\]]+]\([^)]+\)/.test(value)
    || /^#{1,6}\s+/m.test(value)
    || /(^|\n)\s*[-*]\s+/.test(value)
    || /`[^`]+`/.test(value)
    || /^\s*https?:\/\/\S+?(?:\.(?:png|jpe?g|gif|webp)|\/(?:chat-)?attachments\/\S+)/im.test(value)
}

export function taskIdOf(msg: ChatMessage): string {
  return clean(msg.task_id ?? msg.taskId)
}

function isAssistantReplyKind(kind: string): boolean {
  return ['assistant', 'ai', 'bot'].includes(kind)
}

export function emptyTask(taskId: string): TaskState {
  return {
    taskId, progressCount: 0, result: null, request: '', resultText: '',
    failed: false, canceled: false, status: '', error: '', apkUrl: '',
    attach: null, resume: null, approvalState: null, pcReqId: '', lastEventSeq: 0,
  }
}

export function parseToolEvent(content: string): ToolEvent | null {
  const text = clean(content)
  if (!text || text[0] !== '{') return null
  try {
    const e = JSON.parse(text) as ToolEvent
    const type = clean(e.type)
    const validTypes = [
      'pc_dispatch_started',
      'runtime_status',
      'runtime_summary',
      'tool_call',
      'tool_result',
      'tool_approval_required',
      'tool_approval_decision',
      'assistant_message',
      'assistant_chunk',
      'usage',
    ]
    if (!validTypes.includes(type)) return null
    if (['tool_call', 'tool_result', 'tool_approval_required', 'tool_approval_decision'].includes(type) && !clean(e.tool)) return null
    return e
  } catch { return null }
}

export function buildContext(messages: ChatMessage[]): TaskContext {
  const tasks = new Map<string, TaskState>()
  const approvals = new Map<string, ApprovalState>()
  for (const msg of messages) {
    const kind = messageKind(msg)
    const taskId = taskIdOf(msg)
    if (!taskId) continue
    if (!tasks.has(taskId)) tasks.set(taskId, emptyTask(taskId))
    const task = tasks.get(taskId)!
    const status = clean(msg.task_status ?? msg.taskStatus ?? '').toLowerCase()
    if (status) task.status = status
    const taskError = clean(msg.task_error ?? msg.taskError ?? '')
    if (taskError) task.error = taskError
    const apkUrl = clean(msg.task_apk_url ?? msg.taskApkUrl ?? '')
    if (apkUrl) task.apkUrl = apkUrl
    rememberTaskRecoveryState(task, msg)
    if (kind === 'ai_task') task.request = clean(messageText(msg)).replace(/^发起\s*AI\s*开发任务[:：]\s*/i, '') || task.request
    if (kind === 'ai_progress') {
      const event = parseToolEvent(messageText(msg))
      if (event) rememberTaskRecoveryEvent(task, event)
      if (event?.type === 'assistant_message' || event?.type === 'assistant_chunk') continue
      task.progressCount += 1
      if (event) rememberApprovalState(approvals, taskId, event)
    }
    if (
      kind === 'ai_result'
      || (isAssistantReplyKind(kind) && !isAssistantProgressDisplay(msg) && isTerminalStatus(status))
    ) {
      const content = taskResultDisplayText(msg)
      task.result = msg
      task.resultText = content
      const tone = taskResultTone(status, content)
      task.canceled = tone === 'canceled'
      task.failed = tone === 'failed'
    }
  }
  return { tasks, approvals }
}

function rememberTaskRecoveryState(task: TaskState, msg: ChatMessage) {
  const source = msg as Record<string, unknown>
  const attach = objectValue(source.task_attach ?? source.taskAttach ?? source.attach)
  if (attach) task.attach = attach
  const resume = objectValue(source.task_resume ?? source.taskResume ?? source.resume)
  if (resume) task.resume = resume
  const approvalState = objectValue(source.approval_state ?? source.approvalState)
  if (approvalState) task.approvalState = approvalState
  const pcReqId = clean(source.pc_req_id ?? source.pcReqId)
  if (pcReqId) task.pcReqId = pcReqId
  const lastEventSeq = Number(source.last_event_seq ?? source.lastEventSeq ?? 0)
  if (Number.isFinite(lastEventSeq) && lastEventSeq > task.lastEventSeq) task.lastEventSeq = lastEventSeq
}

function rememberTaskRecoveryEvent(task: TaskState, event: ToolEvent) {
  const attach = objectValue(event.attach)
  if (attach) task.attach = attach
  const resume = objectValue(event.resume)
  if (resume) task.resume = resume
  const approvalState = objectValue(event.approval_state ?? event.approvalState)
  if (approvalState) task.approvalState = approvalState
  const pcReqId = clean(event.pc_req_id ?? event.pcReqId)
  if (pcReqId) task.pcReqId = pcReqId
}

function objectValue(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null
}

export function taskResultTone(statusValue: unknown, content: string): TaskTone {
  const status = clean(statusValue ?? '').toLowerCase()
  if (DONE_TASK_STATUSES.has(status)) return 'done'
  if (CANCELED_TASK_STATUSES.has(status) || taskResultContentLooksCanceled(content)) return 'canceled'
  if (FAILED_TASK_STATUSES.has(status) || taskResultContentLooksFailed(content)) return 'failed'
  return 'done'
}

function taskResultContentLooksCanceled(content: string): boolean {
  return /任务已?(?:停止|取消|中断)|已(?:停止|取消|中断)|canceled|cancelled|interrupted/i.test(clean(content))
}

function taskResultContentLooksFailed(content: string): boolean {
  const text = clean(content)
  if (!text) return false
  return /^(?:任务|执行|运行|发布|部署|构建|编译|测试|检查|上传|合并|推送|提交)?\s*(?:失败|错误)[:：。！!\s]/.test(text)
    || /(?:任务|执行|运行|发布|部署|构建|编译|测试|检查|上传|合并|推送|提交).{0,16}(?:失败|错误)/.test(text)
    || /\b(error|failed|failure)\b/i.test(text)
}

function isAssistantProgressDisplay(msg: ChatMessage): boolean {
  return (msg as Record<string, unknown>).assistant_progress_event === true
}

function isTerminalStatus(status: string): boolean {
  return ['done', 'failed', 'error', 'canceled', 'cancelled', 'interrupted'].includes(status)
}

function rememberApprovalState(approvals: Map<string, ApprovalState>, taskId: string, event: ToolEvent) {
  const approvalId = clean(event.approval_id ?? '')
  if (!approvalId) return
  const key = `${taskId}:${approvalId}`
  if (event.type === 'tool_approval_required') {
    if (!approvals.has(key)) {
      approvals.set(key, { status: 'pending', tool: clean(event.tool ?? ''), tone: 'approval', label: '等待确认', meta: '批准前不会执行' })
    }
    return
  }
  if (event.type === 'tool_approval_decision') {
    approvals.set(key, approvalFinalState(event))
  }
}

export function approvalFinalState(event: ToolEvent): ApprovalState {
  const decision = clean(event.decision ?? '').toLowerCase()
  const status = clean(event.status ?? '').toLowerCase()
  if (decision === 'approve' || status === 'approved') return { status: 'approved', tone: 'done', label: '已批准', meta: '继续执行工具' }
  if (['deny', 'denied', 'reject', 'rejected'].includes(decision) || status === 'denied') return { status: 'denied', tone: 'canceled', label: '已拒绝', meta: '工具不会执行' }
  if (decision === 'timeout' || status === 'expired') return { status: 'expired', tone: 'canceled', label: '已过期', meta: '审批已过期' }
  if (['cancel', 'canceled', 'cancelled'].includes(decision) || ['canceled', 'cancelled'].includes(status)) return { status: 'canceled', tone: 'canceled', label: '已取消', meta: '任务已停止' }
  return { status: 'processed', tone: 'done', label: '已处理', meta: '审批已处理' }
}

export function taskIsTerminal(task: TaskState | null): boolean {
  if (!task) return false
  if (task.result) return true
  return DONE_TASK_STATUSES.has(task.status) || FAILED_TASK_STATUSES.has(task.status) || CANCELED_TASK_STATUSES.has(task.status)
}

export function taskIsCanceled(task: TaskState): boolean {
  return !!task.canceled || CANCELED_TASK_STATUSES.has(task.status)
}

export function taskIsFailed(task: TaskState): boolean {
  return !!task.failed || FAILED_TASK_STATUSES.has(task.status) || task.status === 'interrupted'
}

export function statusForTask(task: TaskState | null): { tone: TaskTone; label: string } {
  if (!task) return { tone: 'queued', label: '已排队' }
  if (taskIsTerminal(task)) {
    if (taskIsCanceled(task)) return { tone: 'canceled', label: task.status === 'interrupted' ? '已中断' : '已停止' }
    return taskIsFailed(task) ? { tone: 'failed', label: '任务失败' } : { tone: 'done', label: '任务完成' }
  }
  if (task.result) {
    if (task.canceled) return { tone: 'canceled', label: '已停止' }
    return task.failed ? { tone: 'failed', label: '任务失败' } : { tone: 'done', label: '任务完成' }
  }
  if (['queued', 'pending'].includes(task.status)) return { tone: 'queued', label: '已排队' }
  if (task.status === 'recovering') return { tone: 'running', label: '正在恢复通信' }
  if (['running', 'in_progress', 'processing'].includes(task.status)) {
    return { tone: 'running', label: task.progressCount > 0 ? '执行中' : '等待AI响应' }
  }
  if (task.progressCount > 0) return { tone: 'running', label: '执行中' }
  return { tone: 'queued', label: '已排队' }
}

export function runtimeStatusLabel(phase: string): { tone: TaskTone; title: string; body: string } {
  if (phase === 'pc_dispatched') return { tone: 'running', title: '已派发到 PC 节点', body: '正在等待本机 AI CLI 输出。' }
  if (phase === 'server_updating') return { tone: 'running', title: '服务器正在更新升级', body: '通信临时中断，会自动恢复。' }
  if (phase === 'win_client_updating') return { tone: 'running', title: 'Win 端正在更新升级', body: '通信临时中断，会自动恢复。' }
  if (phase === 'connection_recovering') return { tone: 'running', title: '通信正在恢复', body: '正在重连节点并回放任务现场。' }
  if (phase === 'resume_required') return { tone: 'failed', title: '需要继续任务', body: '自动恢复没有完成，请点击继续让 AI 检查当前工作区后接着处理。' }
  if (phase === 'pc_cli_communication_recovering') return { tone: 'running', title: '通信正在自动恢复', body: '服务器正在更新升级或 Win 端正在更新升级，通信临时中断，会自动恢复。' }
  if (phase === 'pc_cli_recovery_timeout') return { tone: 'failed', title: '通信自动恢复超时', body: '系统已经等待恢复，但仍未收到新的命令、工具结果、回复片段或最终完成事件。' }
  if (phase === 'pc_cli_no_output_timeout') return { tone: 'failed', title: '未收到 AI CLI 输出', body: 'PC 节点没有返回命令、工具结果或最终完成事件。' }
  if (phase === 'pc_tool_result_timeout') return { tone: 'failed', title: '工具结果超时', body: 'PC 节点已开始执行工具，但没有返回工具结果或最终完成事件。' }
  if (phase === 'waiting_approval') return { tone: 'approval', title: '等待工具审批', body: '批准前不会执行工具。' }
  if (phase === 'completed') return { tone: 'done', title: '运行时完成', body: '没有更多运行时动作。' }
  if (phase === 'failed') return { tone: 'failed', title: '运行时受阻', body: '需要继续处理。' }
  if (phase === 'canceled') return { tone: 'canceled', title: '运行时已停止', body: '任务已停止。' }
  return { tone: 'running', title: '运行时正在思考', body: '正在生成下一步计划。' }
}

export function toolEventTitle(event: ToolEvent): string {
  const type = clean(event.type)
  const tool = clean(event.tool ?? 'tool')
  const isResult = type === 'tool_result'
  if (tool === 'shell' && shellEventLooksLikeValidation(event)) return isResult ? '验证完成' : '运行测试/构建'
  if (tool === 'shell') return isResult ? '命令完成' : '执行命令'
  if (tool === 'file_change') return '文件修改'
  if (tool === 'web_search') return isResult ? '搜索完成' : '搜索网络'
  return `${isResult ? '完成' : '调用'} ${tool}`
}

function shellEventLooksLikeValidation(event: ToolEvent): boolean {
  const command = clean(event.args?.command ?? '').toLowerCase()
  const result = clean(event.result ?? '').toLowerCase()
  return (
    /\b(cargo|npm|pnpm|yarn|bun|pytest|gradle|go|mvn|ruff|eslint|tsc)\b/.test(command)
    && /\b(test|check|build|clippy|lint|typecheck|assemble|verify)\b/.test(command)
  ) || result.includes('test result:')
    || result.includes('finished `test`')
    || result.includes('finished `dev`')
    || result.includes('cargo check')
    || result.includes('cargo test')
    || result.includes('build successful')
}

export function toolEventSummary(event: ToolEvent, maxLen = 120): string {
  const type = clean(event.type)
  if (type === 'tool_result') {
    return clean(event.result ?? '').slice(0, maxLen) || '完成'
  }
  const args = event.args
  if (!args) return ''
  const command = clean(args.command)
  if (command) return command.slice(0, maxLen)
  const path = clean(args.path ?? args.file)
  if (path) return path.slice(0, maxLen)
  const changes = Array.isArray(args.changes) ? args.changes : null
  if (changes?.length) {
    const files = changes
      .map((change) => clean((change as Record<string, unknown>).path ?? (change as Record<string, unknown>).file))
      .filter(Boolean)
    if (files.length) return files.slice(0, 3).join(', ').slice(0, maxLen)
  }
  return clean(args.query ?? args.input ?? args.content).slice(0, maxLen)
}

export function usageEventSummary(event: ToolEvent): string {
  const input = numberField(event, 'input_tokens')
  const output = numberField(event, 'output_tokens')
  const total = numberField(event, 'total_tokens')
  const cached = numberField(event, 'cached_input_tokens')
  const reasoning = numberField(event, 'reasoning_output_tokens')
  const parts = [
    input !== null ? `输入 ${formatNumber(input)}` : '',
    cached !== null ? `缓存 ${formatNumber(cached)}` : '',
    output !== null ? `输出 ${formatNumber(output)}` : '',
    reasoning !== null ? `推理 ${formatNumber(reasoning)}` : '',
    total !== null ? `总计 ${formatNumber(total)}` : '',
  ].filter(Boolean)
  return parts.join(' · ') || '已记录本轮用量'
}

function numberField(event: ToolEvent, key: string): number | null {
  const value = event[key]
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

function formatNumber(value: number): string {
  return new Intl.NumberFormat('zh-CN').format(value)
}

export function approvalStateFor(context: TaskContext, taskId: string, approvalId: string): ApprovalState | null {
  return context.approvals.get(`${taskId}:${approvalId}`) ?? null
}

export function shortId(value: string): string {
  const v = clean(value)
  return v.length > 12 ? `${v.slice(0, 8)}...` : v
}
