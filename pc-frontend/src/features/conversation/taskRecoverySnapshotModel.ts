export type RecoverySnapshotPhase = 'connection_recovering' | 'resume_required' | null

interface RecoverySnapshotPhaseInput {
  taskStatus?: unknown
  taskError?: unknown
  journalStatus?: unknown
  resume?: Record<string, unknown> | null
  attach?: Record<string, unknown> | null
}

export function recoverySnapshotPhase(input: RecoverySnapshotPhaseInput): RecoverySnapshotPhase {
  const taskStatus = value(input.taskStatus).toLowerCase()
  if (['done', 'completed', 'success', 'failed', 'error', 'canceled', 'cancelled'].includes(taskStatus)) return null

  const journalStatus = value(input.journalStatus).toLowerCase()
  const resumeStatus = value(input.resume?.status).toLowerCase()
  const attachStatus = value(input.attach?.status).toLowerCase()
  const nextAction = value(input.resume?.next_action ?? input.resume?.nextAction).toLowerCase()
  if (
    nextAction === 'continue_from_snapshot'
    || resumeStatus === 'detached'
    || attachStatus === 'detached'
  ) return 'resume_required'

  const taskError = value(input.taskError)
  const explicitlyRecovering = ['recovering', 'interrupted', 'resume_required'].includes(taskStatus)
    || /恢复|重启|通信中断|连接中断|节点.*(?:离线|断开)|journal|sidecar|CliDone/i.test(taskError)
  if (explicitlyRecovering || journalStatus === 'agent_offline_or_timeout') return 'connection_recovering'

  // live/sidecar_recoverable 表示当前任务有正常运行现场，不代表发生过断线。
  return null
}

function value(input: unknown): string {
  return String(input == null ? '' : input).trim()
}
