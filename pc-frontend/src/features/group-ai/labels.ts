import type { ProjectAiEvent, ProjectAiMatter, ProjectAiMatterAssignment } from './types'

export function modeLabel(mode: string) {
  if (mode === 'split') return 'Split'
  if (mode === 'critic') return 'Critic'
  return 'Solo'
}

export function statusLabel(status: string, decision?: string | null) {
  if (status === 'done') return '已完成'
  if (status === 'review_ready') return '待验收'
  if (status === 'running') return '运行中'
  if (status === 'canceled') return '已取消'
  if (status === 'failed') return '失败'
  if (decision === 'approved') return '已批准'
  if (decision === 'changes_requested') return '待调整'
  return '计划就绪'
}

export function assignmentStatusLabel(status: string) {
  if (status === 'completed') return '已完成'
  if (status === 'settled') return '已结算'
  if (status === 'settled_no_provider') return '无提供者结算'
  if (status === 'failed') return '失败'
  if (status === 'running') return '执行中'
  return '待执行'
}

export function eventHint(event: ProjectAiEvent) {
  const payload = event.payload ?? {}
  const computeCallId = stringPayload(payload, 'compute_call_id')
  const assignmentId = stringPayload(payload, 'assignment_id')
  const accountingStatus = stringPayload(payload, 'accounting_status')
  const branchName = stringPayload(payload, 'branch_name')
  const parts = [
    computeCallId && `compute ${computeCallId}`,
    branchName && `branch ${branchName}`,
    assignmentId && `assignment ${assignmentId}`,
    accountingStatus,
  ]
    .filter((item): item is string => Boolean(item))
    .slice(0, 2)
  return parts.join(' · ')
}

export function isDone(matter: ProjectAiMatter) {
  return matter.status === 'done' || matter.status === 'canceled'
}

export function canRunAssignment(matter: ProjectAiMatter, assignment: ProjectAiMatterAssignment) {
  return (
    !isDone(matter) &&
    !['running', 'completed', 'settled', 'settled_no_provider'].includes(assignment.status)
  )
}

function stringPayload(payload: Record<string, unknown>, key: string) {
  const value = payload[key]
  return typeof value === 'string' && value.trim() ? value.trim() : ''
}
