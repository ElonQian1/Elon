/** 对应 pc_app_dev_tasks.js 的纯业务逻辑层 */
import { clean } from '../../lib/utils'
import type { TaskState, TaskTone, ToolEvent, ApprovalState, ChatMessage, TaskContext } from './types'

export function messageKind(msg: ChatMessage): string {
  return clean(msg.kind ?? msg.role ?? msg.message_kind).toLowerCase()
}

export function messageText(msg: ChatMessage): string {
  return clean(msg.content ?? msg.text ?? msg.message)
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
    const validTypes = ['runtime_status', 'runtime_summary', 'tool_call', 'tool_result', 'tool_approval_required', 'tool_approval_decision']
    if (!validTypes.includes(type)) return null
    if (!['runtime_status', 'runtime_summary'].includes(type) && !clean(e.tool)) return null
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
    if (kind === 'ai_task') task.request = clean(messageText(msg)).replace(/^发起\s*AI\s*开发任务[:：]\s*/i, '') || task.request
    if (kind === 'ai_progress') {
      task.progressCount += 1
      const event = parseToolEvent(messageText(msg))
      if (event) rememberApprovalState(approvals, taskId, event)
    }
    if (kind === 'ai_result' || isAssistantReplyKind(kind)) {
      const content = messageText(msg)
      task.result = msg
      task.resultText = content
      task.canceled = /停止|取消|canceled|cancelled/i.test(content)
      task.failed = !task.canceled && /失败|错误|error|failed/i.test(content)
    }
  }
  return { tasks, approvals }
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
  return ['done', 'failed', 'canceled', 'cancelled', 'interrupted'].includes(task.status)
}

export function taskIsCanceled(task: TaskState): boolean {
  return !!task.canceled || ['canceled', 'cancelled', 'interrupted'].includes(task.status)
}

export function taskIsFailed(task: TaskState): boolean {
  return !!task.failed || ['failed', 'interrupted'].includes(task.status)
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
  if (['running', 'in_progress', 'processing'].includes(task.status)) {
    return { tone: 'running', label: task.progressCount > 0 ? '执行中' : '等待AI响应' }
  }
  if (task.progressCount > 0) return { tone: 'running', label: '执行中' }
  return { tone: 'queued', label: '已排队' }
}

export function runtimeStatusLabel(phase: string): { tone: TaskTone; title: string; body: string } {
  if (phase === 'waiting_approval') return { tone: 'approval', title: '等待工具审批', body: '批准前不会执行工具。' }
  if (phase === 'completed') return { tone: 'done', title: '运行时完成', body: '没有更多运行时动作。' }
  if (phase === 'failed') return { tone: 'failed', title: '运行时受阻', body: '需要继续处理。' }
  if (phase === 'canceled') return { tone: 'canceled', title: '运行时已停止', body: '任务已停止。' }
  return { tone: 'running', title: '运行时正在思考', body: '正在生成下一步计划。' }
}

export function approvalStateFor(context: TaskContext, taskId: string, approvalId: string): ApprovalState | null {
  return context.approvals.get(`${taskId}:${approvalId}`) ?? null
}

export function shortId(value: string): string {
  const v = clean(value)
  return v.length > 12 ? `${v.slice(0, 8)}...` : v
}
