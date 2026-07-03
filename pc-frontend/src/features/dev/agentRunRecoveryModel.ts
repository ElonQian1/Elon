import { clean } from '../../lib/utils'

export type RecoveryTone = 'running' | 'done' | 'failed' | 'muted'

export interface RecoveryFact {
  label: string
  value: string
  tone?: RecoveryTone
}

export interface RecoveryView {
  taskId: string
  title: string
  badge: string
  tone: RecoveryTone
  summary: string
  detail: string
  actionLabel: string
  canCancel: boolean
  canContinue: boolean
  continuePrompt: string
  facts: RecoveryFact[]
}

export function recoveryViewFromEntry(entry: Record<string, unknown>): RecoveryView {
  const taskId = clean(entry.task_id ?? entry.taskId)
  const cliName = clean(entry.cli_name ?? entry.cliName ?? 'agent')
  const route = clean(entry.route ?? '')
  const status = clean(entry.status ?? '')
  const action = clean(entry.recommended_action ?? entry.recommendedAction)
  const reason = clean(entry.reason)
  const cwd = clean(entry.cwd)
  const tty = objectOf(entry.tty_reconnect ?? entry.ttyReconnect)
  const resume = objectOf(entry.resume)
  const attach = objectOf(entry.attach)
  const approval = objectOf(
    entry.tool_approval_recovery
      ?? entry.toolApprovalRecovery
      ?? resume.tool_approval_recovery
      ?? resume.toolApprovalRecovery,
  )
  const canCancel = Boolean(entry.can_cancel ?? entry.canCancel ?? resume.can_cancel ?? resume.canCancel)
  const canContinue = Boolean(entry.can_continue ?? entry.canContinue) || action === 'continue_from_snapshot'
  const actionLabel = recoveryActionLabel(action)
  const tone = recoveryTone(status, canCancel, canContinue)
  const facts = recoveryFacts({ status, action, route, cwd, tty, resume, attach, approval, canCancel, canContinue })
  return {
    taskId,
    title: shortRunId(taskId || cliName),
    badge: recoveryBadge(status, action, canCancel, canContinue),
    tone,
    summary: [cliName, route, actionLabel].filter(Boolean).join(' · '),
    detail: reason || recoveryDefaultReason(status, action),
    actionLabel,
    canCancel,
    canContinue,
    continuePrompt: buildContinuePrompt({ taskId, cliName, route, cwd, status, action, reason, tty, resume, attach, approval }),
    facts,
  }
}

export function recoveryViewFromTask(task: Record<string, unknown>): RecoveryView {
  const resume = objectOf(task.resume)
  const attach = objectOf(task.attach)
  return recoveryViewFromEntry({
    task_id: task.task_id ?? task.taskId ?? task.req_id ?? task.reqId,
    cli_name: task.cli_name ?? task.cliName ?? 'agent',
    route: task.route,
    cwd: task.cwd,
    status: resume.status ?? task.status,
    recommended_action: resume.next_action ?? resume.nextAction,
    reason: resume.reason ?? attach.reason,
    can_cancel: resume.can_cancel ?? resume.canCancel,
    can_continue: clean(resume.next_action ?? resume.nextAction) === 'continue_from_snapshot',
    tty_reconnect: resume.tty_reattach ?? resume.ttyReattach,
    resume,
    attach,
  })
}

export function recoveryActionLabel(action: string): string {
  const value = clean(action).toLowerCase()
  if (value === 'wait_or_cancel') return '等待或停止'
  if (value === 'continue_from_snapshot') return '基于快照继续'
  if (value === 'attach_sidecar') return '重接 sidecar'
  if (value === 'refresh_snapshot') return '刷新快照'
  if (value === 'approve_or_deny_current_waiter') return '处理当前审批'
  return value || '查看状态'
}

export function approvalRecoveryLabel(status: string): string {
  const value = clean(status).toLowerCase()
  if (value === 'active_waiter') return '审批可继续'
  if (value === 'lost_after_restart') return '审批已失效'
  if (value === 'closed_by_terminal_task') return '审批已关闭'
  if (value === 'no_active_waiter') return '暂无审批'
  if (value === 'unavailable') return '审批不可判断'
  return value || '暂无审批'
}

export function shortRunId(value: string): string {
  const text = clean(value) || 'agent run'
  if (text.length <= 24) return text
  return `${text.slice(0, 12)}…${text.slice(-8)}`
}

function recoveryFacts(input: {
  status: string
  action: string
  route: string
  cwd: string
  tty: Record<string, unknown>
  resume: Record<string, unknown>
  attach: Record<string, unknown>
  approval: Record<string, unknown>
  canCancel: boolean
  canContinue: boolean
}): RecoveryFact[] {
  const facts: RecoveryFact[] = []
  const attachStatus = clean(input.attach.status ?? input.status)
  if (attachStatus) facts.push({ label: '现场', value: attachStatusLabel(attachStatus), tone: attachTone(attachStatus) })
  facts.push({ label: '下一步', value: recoveryActionLabel(input.action), tone: input.canContinue ? 'running' : 'muted' })
  facts.push({ label: '停止', value: input.canCancel ? '可停止' : '不可停止', tone: input.canCancel ? 'running' : 'muted' })
  if (clean(input.resume.can_replay_journal_events ?? input.resume.canReplayJournalEvents)) {
    facts.push({ label: 'journal', value: '可回放', tone: 'done' })
  }
  const approvalStatus = clean(input.approval.status)
  if (approvalStatus) {
    const count = Number(input.approval.journal_pending_count ?? input.approval.journalPendingCount ?? 0)
    facts.push({
      label: '审批',
      value: count > 0 ? `${approvalRecoveryLabel(approvalStatus)} · ${count}` : approvalRecoveryLabel(approvalStatus),
      tone: approvalStatus === 'active_waiter' ? 'running' : approvalStatus === 'lost_after_restart' ? 'failed' : 'muted',
    })
  }
  const ttySupported = input.tty.supported === true
  const ttyLabel = clean(input.tty.user_label ?? input.tty.userLabel ?? input.tty.status)
  if (ttyLabel || input.tty.supported !== undefined) {
    facts.push({ label: '终端', value: ttySupported ? (ttyLabel || '可重接') : (ttyLabel || '不可重接'), tone: ttySupported ? 'running' : 'muted' })
  }
  if (input.route) facts.push({ label: '路线', value: input.route })
  if (input.cwd) facts.push({ label: '目录', value: input.cwd })
  return facts
}

function buildContinuePrompt(input: {
  taskId: string
  cliName: string
  route: string
  cwd: string
  status: string
  action: string
  reason: string
  tty: Record<string, unknown>
  resume: Record<string, unknown>
  attach: Record<string, unknown>
  approval: Record<string, unknown>
}): string {
  const approvalStatus = clean(input.approval.status)
  const ttyReason = clean(input.tty.reason)
  const resumeReason = clean(input.resume.reason ?? input.reason)
  const attachReason = clean(input.attach.reason)
  return [
    '请基于本机任务恢复信息继续处理这个项目任务。',
    '',
    `任务 ID：${input.taskId || '未知'}`,
    `CLI：${input.cliName || 'agent'}${input.route ? ` / ${input.route}` : ''}`,
    input.cwd ? `工作目录：${input.cwd}` : '',
    `当前状态：${clean(input.status) || '未知'}`,
    `推荐动作：${recoveryActionLabel(input.action)}`,
    resumeReason ? `恢复原因：${resumeReason}` : '',
    attachReason && attachReason !== resumeReason ? `现场原因：${attachReason}` : '',
    ttyReason ? `终端限制：${ttyReason}` : '',
    approvalStatus ? `工具审批恢复：${approvalRecoveryLabel(approvalStatus)}${clean(input.approval.reason) ? `，${clean(input.approval.reason)}` : ''}` : '',
    '',
    '要求：先回放云端消息和本机 journal/任务快照，确认工作区状态；不要批准已经失效的旧审批；如果需要继续执行，请开新一轮任务接着做。',
  ].filter(Boolean).join('\n')
}

function recoveryTone(status: string, canCancel: boolean, canContinue: boolean): RecoveryTone {
  const value = clean(status).toLowerCase()
  if (['failed', 'error'].includes(value)) return 'failed'
  if (['done', 'completed', 'terminal'].includes(value)) return canContinue ? 'running' : 'done'
  if (canCancel || canContinue || value === 'running' || value === 'live' || value === 'sidecar_recoverable') return 'running'
  return 'muted'
}

function recoveryBadge(status: string, action: string, canCancel: boolean, canContinue: boolean): string {
  if (canCancel) return '运行中'
  if (canContinue) return '可继续'
  const value = clean(status).toLowerCase()
  if (value === 'detached') return '已脱离'
  if (value === 'sidecar_recoverable') return '可重接'
  if (['done', 'completed', 'terminal'].includes(value)) return '已结束'
  if (['failed', 'error'].includes(value)) return '失败'
  return recoveryActionLabel(action)
}

function recoveryDefaultReason(status: string, action: string): string {
  const value = clean(status).toLowerCase()
  if (value === 'detached') return '节点没有活动句柄，但本机 journal 可用于恢复快照。'
  if (value === 'sidecar_recoverable') return 'sidecar 会话仍可重接，优先恢复 sidecar 控制面。'
  if (clean(action) === 'continue_from_snapshot') return '原任务不能原地继续，需要基于快照开新一轮。'
  return '已读取本机任务恢复状态。'
}

function attachStatusLabel(status: string): string {
  const value = clean(status).toLowerCase()
  if (value === 'live') return '本机句柄还活着'
  if (value === 'detached') return '句柄丢失'
  if (value === 'sidecar_recoverable') return 'sidecar 可重接'
  if (value === 'terminal') return '已终态'
  if (value === 'missing') return '无本机记录'
  return value
}

function attachTone(status: string): RecoveryTone {
  const value = clean(status).toLowerCase()
  if (value === 'live' || value === 'sidecar_recoverable') return 'running'
  if (value === 'detached') return 'failed'
  if (value === 'terminal') return 'done'
  return 'muted'
}

function objectOf(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : {}
}
