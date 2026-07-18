import type {
  LocalTaskApproval,
  LocalTaskDetail,
  LocalTaskEvent,
  LocalTaskRecord,
  LocalTaskRuntimeState,
  LocalTaskSupervisionState,
  LocalTaskUpdateRecovery,
  LocalTaskResumeWorkspaceStatus,
  LocalTaskStatusView,
  LocalTaskTokenUsage,
} from './types'

type JsonObject = Record<string, unknown>

const TERMINAL_STATUSES = new Set([
  'done', 'finished', 'success', 'succeeded', 'failed', 'error',
  'canceled', 'cancelled', 'interrupted', 'resume_required',
])

export function normalizeLocalTaskList(payload: unknown): LocalTaskRecord[] {
  const root = objectValue(payload)
  const records = arrayValue(root.records ?? root.tasks ?? root.items ?? payload)
  return records
    .map((item) => normalizeLocalTaskRecord(item))
    .filter((task) => Boolean(task.id))
}

export function normalizeLocalTaskDetail(payload: unknown): LocalTaskDetail {
  const root = objectValue(payload)
  const recordSource = root.task ?? root.record ?? root
  const events = arrayValue(root.events).map(normalizeLocalTaskEvent)
  const task = normalizeLocalTaskRecord(recordSource)
  if (!task.id) task.id = textValue(root.task_id ?? root.id)

  const eventUsage = usageFromEvents(events)
  if (eventUsage.total_tokens > task.token_usage.total_tokens) task.token_usage = eventUsage
  if (!task.final_reply) task.final_reply = finalReplyFromEvents(events)

  const approvalRoot = objectValue(root.approval_state)
  const approvals = arrayValue(approvalRoot.approvals ?? root.approvals)
    .map(normalizeLocalTaskApproval)
    .filter((approval) => Boolean(approval.approval_id))
  const lastSeq = numberValue(root.last_event_seq)
    ?? events.reduce((max, event) => Math.max(max, event.seq), 0)

  return {
    task,
    events,
    approvals,
    last_event_seq: lastSeq,
    has_more: Boolean(root.has_more),
    supervision: normalizeSupervisionState(root.supervision),
    runtime: normalizeRuntimeState(root.runtime),
    update_recovery: normalizeUpdateRecovery(root.update_recovery),
    resume_workspace_status: normalizeResumeWorkspaceStatus(root.resume_workspace_status),
  }
}

export function normalizeUpdateRecovery(payload: unknown): LocalTaskUpdateRecovery | undefined {
  const root = objectValue(payload)
  if (!Object.keys(root).length) return undefined
  const from = objectValue(root.from_release)
  const to = objectValue(root.to_release)
  const transport = objectValue(root.transport)
  const review = objectValue(root.final_review)
  return {
    protocol: textValue(root.protocol),
    update_id: textValue(root.update_id),
    state: textValue(root.state),
    state_reason: textValue(root.state_reason ?? root.final_reason),
    original_task_id: textValue(root.original_task_id),
    resume_task_id: textValue(root.resume_task_id),
    from_version: textValue(from.version),
    from_git_sha: textValue(from.git_sha),
    to_version: textValue(to.version),
    to_git_sha: textValue(to.git_sha),
    sidecar_session_id: textValue(root.sidecar_session_id),
    sidecar_output_offset: numberValue(root.sidecar_output_offset) ?? 0,
    sidecar_output_sequence: numberValue(root.sidecar_output_sequence) ?? 0,
    journal_cursor: numberValue(root.journal_cursor) ?? 0,
    transport_kind: textValue(transport.kind),
    transport_protocol: textValue(transport.protocol),
    capabilities: textArray(transport.capabilities),
    replay_from_cursor: Boolean(transport.replay_from_cursor),
    resume_strategy: textValue(root.resume_strategy),
    completion_event_id: textValue(root.completion_event_id),
    terminal_task_status: textValue(root.terminal_task_status),
    terminal_finished_at_ms: timestampValue(root.terminal_finished_at_ms),
    expected_downtime_ms: numberValue(root.expected_downtime_ms) ?? 0,
    review_verdict: textValue(review.verdict),
    review_summary: textValue(review.summary),
  }
}

function normalizeResumeWorkspaceStatus(payload: unknown): LocalTaskResumeWorkspaceStatus | undefined {
  const root = objectValue(payload)
  if (!Object.keys(root).length) return undefined
  return {
    eligible: Boolean(root.eligible),
    derivation: textValue(root.derivation),
    active_workspace_path: textValue(root.active_workspace_path),
    branch: textValue(root.branch),
    git_head: textValue(root.git_head),
    occupied: Boolean(root.occupied),
    reason: textValue(root.reason),
  }
}

export function normalizeSupervisionState(payload: unknown): LocalTaskSupervisionState {
  const root = objectValue(payload)
  const contractRoot = objectValue(root.contract)
  const reviewRoot = objectValue(root.review)
  const evidenceRoot = objectValue(root.evidence)
  const enabled = Boolean(root.enabled)
  return {
    protocol: textValue(root.protocol) || 'elon.desktop_pc_supervision.v1',
    enabled,
    contract: enabled ? {
      protocol: textValue(contractRoot.protocol),
      supervisor: textValue(contractRoot.supervisor),
      task_role: textValue(contractRoot.task_role),
      parent_task_id: textValue(contractRoot.parent_task_id) || undefined,
      root_task_id: textValue(contractRoot.root_task_id) || undefined,
      acceptance_criteria: textArray(contractRoot.acceptance_criteria),
      improvement_policy: textValue(contractRoot.improvement_policy),
    } : undefined,
    review: Object.keys(reviewRoot).length ? {
      protocol: textValue(reviewRoot.protocol),
      verdict: textValue(reviewRoot.verdict),
      summary: textValue(reviewRoot.summary),
      improvements: textArray(reviewRoot.improvements),
      reviewed_by: textValue(reviewRoot.reviewed_by),
      reviewed_at_ms: timestampValue(reviewRoot.reviewed_at_ms),
    } : undefined,
    evidence: {
      event_count: numberValue(evidenceRoot.event_count) ?? 0,
      tool_calls: numberValue(evidenceRoot.tool_calls) ?? 0,
      tool_results: numberValue(evidenceRoot.tool_results) ?? 0,
      failed_tools: numberValue(evidenceRoot.failed_tools) ?? 0,
      file_change_events: numberValue(evidenceRoot.file_change_events) ?? 0,
      changed_files: textArray(evidenceRoot.changed_files),
      command_exit_codes: arrayValue(evidenceRoot.command_exit_codes).map((item) => {
        const entry = objectValue(item)
        return { command: textValue(entry.command), exit_code: numberValue(entry.exit_code) ?? 0 }
      }),
      failure_summaries: textArray(evidenceRoot.failure_summaries),
      agent_messages: numberValue(evidenceRoot.agent_messages) ?? 0,
      terminal_event_seen: Boolean(evidenceRoot.terminal_event_seen),
    },
  }
}

export function normalizeRuntimeState(payload: unknown): LocalTaskRuntimeState {
  const root = objectValue(payload)
  const policy = objectValue(root.timeout_policy)
  return {
    phase: textValue(root.phase) || 'reasoning',
    current_command: textValue(root.current_command),
    last_progress: timestampValue(root.last_progress),
    heartbeat: timestampValue(root.heartbeat),
    idle_duration: numberValue(root.idle_duration) ?? 0,
    timeout_policy: {
      mode: textValue(policy.mode) || 'fixed_total',
      total_timeout_secs: numberValue(policy.total_timeout_secs) ?? 0,
      idle_timeout_secs: numberValue(policy.idle_timeout_secs) ?? 0,
      heartbeat_secs: numberValue(policy.heartbeat_secs) ?? 0,
      progress_aware: Boolean(policy.progress_aware),
    },
  }
}

export function normalizeLocalTaskRecord(payload: unknown): LocalTaskRecord {
  const root = objectValue(payload)
  const usage = normalizeUsage(root.token_usage ?? root.usage ?? root)
  const resume = objectValue(root.resume)
  const status = textValue(root.status || root.task_status) || 'unknown'
  const workspace = textValue(root.workspace_path ?? root.cwd)
  return {
    id: textValue(root.task_id ?? root.req_id ?? root.id),
    project_id: textValue(root.project_id),
    channel_id: textValue(root.channel_id),
    conversation_id: textValue(root.conversation_id),
    workspace_path: workspace,
    prompt: textValue(root.prompt ?? root.user_prompt),
    runtime_permission: textValue(root.runtime_permission),
    cli_name: textValue(root.cli_name ?? root.cli) || 'codex',
    status,
    sync_state: textValue(root.sync_state ?? root.accounting_sync_state) || 'local_only',
    final_reply: textValue(root.final_reply ?? root.reply ?? root.result),
    error: textValue(root.error ?? root.error_message),
    started_at_ms: timestampValue(root.started_at_ms ?? root.started_at),
    updated_at_ms: timestampValue(root.updated_at_ms ?? root.updated_at ?? root.finished_at_ms ?? root.finished_at),
    finished_at_ms: timestampValue(root.finished_at_ms ?? root.finished_at),
    token_usage: usage,
    can_cancel: booleanValue(root.can_cancel ?? resume.can_cancel)
      ?? !TERMINAL_STATUSES.has(status.toLowerCase()),
  }
}

export function normalizeLocalTaskEvent(payload: unknown): LocalTaskEvent {
  const wrapper = objectValue(payload)
  const raw = objectValue(wrapper.event ?? payload)
  const inner = objectValue(raw.event)
  const eventPayload = Object.keys(inner).length ? inner : raw
  const type = textValue(eventPayload.type ?? raw.type) || 'event'
  return {
    seq: numberValue(wrapper.seq ?? raw.seq) ?? 0,
    type,
    text: eventText(eventPayload, raw),
    at_ms: timestampValue(eventPayload.at_ms ?? raw.at_ms ?? eventPayload.timestamp),
    stream: textValue(raw.stream ?? eventPayload.stream),
    raw,
  }
}

export function mergeLocalTaskEvents(
  current: LocalTaskEvent[],
  incoming: LocalTaskEvent[],
): LocalTaskEvent[] {
  const merged = new Map<string, LocalTaskEvent>()
  for (const event of [...current, ...incoming]) {
    const key = event.seq > 0
      ? `seq:${event.seq}`
      : `fallback:${event.type}:${event.at_ms ?? ''}:${event.text}`
    merged.set(key, event)
  }
  return [...merged.values()].sort((left, right) => {
    if (left.seq && right.seq) return left.seq - right.seq
    return (left.at_ms ?? 0) - (right.at_ms ?? 0)
  })
}

export function mergeLocalTaskDetail(
  current: LocalTaskDetail | null,
  incoming: LocalTaskDetail,
): LocalTaskDetail {
  if (!current || current.task.id !== incoming.task.id) return incoming
  return {
    ...incoming,
    task: mergeTaskRecord(current.task, incoming.task),
    events: mergeLocalTaskEvents(current.events, incoming.events),
    last_event_seq: Math.max(current.last_event_seq, incoming.last_event_seq),
    supervision: incoming.supervision.enabled ? incoming.supervision : current.supervision,
    update_recovery: incoming.update_recovery ?? current.update_recovery,
    resume_workspace_status: incoming.resume_workspace_status ?? current.resume_workspace_status,
  }
}

export function taskIdFromCreateResponse(payload: unknown): string {
  const root = objectValue(payload)
  return textValue(
    root.task_id
      ?? root.id
      ?? objectValue(root.task).task_id
      ?? objectValue(root.task).id
      ?? objectValue(root.record).req_id,
  )
}

export function localTaskStatus(status: string): LocalTaskStatusView {
  switch (status.trim().toLowerCase()) {
    case 'queued': return { label: '等待执行', tone: 'warning', terminal: false }
    case 'starting': return { label: '正在启动', tone: 'running', terminal: false }
    case 'running': return { label: '运行中', tone: 'running', terminal: false }
    case 'waiting_approval': return { label: '等待审批', tone: 'warning', terminal: false }
    case 'cancel_requested': return { label: '正在取消', tone: 'warning', terminal: false }
    case 'done':
    case 'finished':
    case 'success':
    case 'succeeded': return { label: '已完成', tone: 'done', terminal: true }
    case 'failed':
    case 'error': return { label: '失败', tone: 'danger', terminal: true }
    case 'canceled':
    case 'cancelled':
    case 'interrupted': return { label: '已停止', tone: 'muted', terminal: true }
    case 'resume_required': return { label: '可继续', tone: 'warning', terminal: true }
    default: return { label: status || '未知', tone: 'muted', terminal: false }
  }
}

export function syncStateLabel(syncState: string): string {
  switch (syncState.trim().toLowerCase()) {
    case 'synced':
    case 'reported':
    case 'acked': return '已同步云端'
    case 'syncing':
    case 'reporting': return '正在同步'
    case 'pending':
    case 'pending_sync': return '待云端恢复后同步'
    case 'retrying':
    case 'sync_failed': return '同步重试中'
    case 'rejected': return '云端拒绝，请检查账号或任务'
    case 'local_only': return '本机执行记录'
    default: return '仅保存在本机'
  }
}

export function pendingLocalTaskSyncCount(tasks: LocalTaskRecord[]): number {
  return tasks.filter((task) => {
    const state = task.sync_state.trim().toLowerCase()
    return state === 'pending' || state === 'pending_sync' || state === 'retrying' || state === 'sync_failed'
  }).length
}

export function pendingSyncCountFromList(payload: unknown): number {
  const root = objectValue(payload)
  const declared = numberValue(root.pending_sync_count)
  return declared == null
    ? pendingLocalTaskSyncCount(normalizeLocalTaskList(payload))
    : Math.max(0, Math.floor(declared))
}

function normalizeLocalTaskApproval(payload: unknown): LocalTaskApproval {
  const root = objectValue(payload)
  return {
    approval_id: textValue(root.approval_id ?? root.id),
    tool: textValue(root.tool ?? root.tool_name),
    status: textValue(root.status),
    decision: textValue(root.decision),
    actionable: Boolean(root.actionable),
    label: textValue(root.label) || '工具操作审批',
    meta: textValue(root.meta),
    checkpoint: root.checkpoint,
  }
}

function normalizeUsage(payload: unknown): LocalTaskTokenUsage {
  const root = objectValue(payload)
  const input = numberValue(root.input_tokens ?? root.prompt_tokens) ?? 0
  const output = numberValue(root.output_tokens ?? root.completion_tokens) ?? 0
  return {
    input_tokens: input,
    output_tokens: output,
    total_tokens: numberValue(root.total_tokens) ?? input + output,
  }
}

function usageFromEvents(events: LocalTaskEvent[]): LocalTaskTokenUsage {
  let usage = normalizeUsage({})
  for (const event of events) {
    const raw = objectValue(event.raw.event ?? event.raw)
    if (event.type === 'usage' || raw.total_tokens != null || raw.input_tokens != null) {
      usage = normalizeUsage(raw.usage ?? raw)
    }
  }
  return usage
}

function finalReplyFromEvents(events: LocalTaskEvent[]): string {
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const event = events[index]
    if (['final_reply', 'assistant_message', 'done'].includes(event.type) && event.text) {
      return event.text
    }
  }
  return ''
}

function mergeTaskRecord(current: LocalTaskRecord, incoming: LocalTaskRecord): LocalTaskRecord {
  return {
    ...current,
    ...incoming,
    prompt: incoming.prompt || current.prompt,
    final_reply: incoming.final_reply || current.final_reply,
    error: incoming.error || current.error,
    workspace_path: incoming.workspace_path || current.workspace_path,
    token_usage: incoming.token_usage.total_tokens >= current.token_usage.total_tokens
      ? incoming.token_usage
      : current.token_usage,
  }
}

function eventText(primary: JsonObject, fallback: JsonObject): string {
  const candidates = [
    primary.final_reply, primary.message, primary.text, primary.content,
    primary.result, primary.output, primary.error, primary.delta,
    fallback.text, fallback.message,
  ]
  for (const candidate of candidates) {
    if (typeof candidate === 'string' && candidate.trim()) return candidate.trim()
    if (candidate != null && typeof candidate !== 'object') return String(candidate)
  }
  return JSON.stringify(primary, null, 2)
}

function objectValue(value: unknown): JsonObject {
  return value != null && typeof value === 'object' && !Array.isArray(value)
    ? value as JsonObject
    : {}
}

function arrayValue(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function textArray(value: unknown): string[] {
  return arrayValue(value).map(textValue).filter(Boolean)
}

function textValue(value: unknown): string {
  return typeof value === 'string' || typeof value === 'number' ? String(value).trim() : ''
}

function numberValue(value: unknown): number | undefined {
  const number = typeof value === 'number' ? value : Number(value)
  return Number.isFinite(number) ? number : undefined
}

function timestampValue(value: unknown): number | undefined {
  const numeric = numberValue(value)
  if (numeric != null) return numeric
  if (typeof value !== 'string' || !value.trim()) return undefined
  const parsed = Date.parse(value)
  return Number.isNaN(parsed) ? undefined : parsed
}

function booleanValue(value: unknown): boolean | undefined {
  if (typeof value === 'boolean') return value
  if (value === 1 || value === '1' || value === 'true') return true
  if (value === 0 || value === '0' || value === 'false') return false
  return undefined
}
