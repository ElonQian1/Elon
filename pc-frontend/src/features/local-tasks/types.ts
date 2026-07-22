export type LocalTaskRuntimePermission =
  | 'project_write'
  | 'full_access'
  | 'danger_full_access'

export interface LocalTaskCreateInput {
  project_id: string
  channel_id?: string
  conversation_id: string
  workspace_path: string
  prompt: string
  runtime_permission: LocalTaskRuntimePermission
  supervision?: LocalTaskSupervisionInput
  contract_revision?: {
    schema: 'elon.supervision.contract_revision.v1'
    reason: string
  }
}

export interface LocalTaskSupervisionInput {
  protocol?: 'elon.desktop_pc_supervision.v1'
  supervisor?: string
  task_role?: 'requirement' | 'capability_repair' | 'resume_original' | 'post_task_improvement'
  parent_task_id?: string
  root_task_id?: string
  acceptance_criteria?: string[]
  improvement_policy?: 'after_task_or_unblock' | 'after_task_only' | 'observe_only'
}

export interface LocalFullAccessGrant {
  project_id: string
  workspace_path: string
  granted_at_ms?: number
}

export interface LocalTaskTokenUsage {
  input_tokens: number
  output_tokens: number
  total_tokens: number
}

export interface LocalTaskRecord {
  id: string
  project_id: string
  channel_id: string
  conversation_id: string
  workspace_path: string
  prompt: string
  runtime_permission: string
  cli_name: string
  status: string
  sync_state: string
  final_reply: string
  error: string
  started_at_ms?: number
  updated_at_ms?: number
  finished_at_ms?: number
  token_usage: LocalTaskTokenUsage
  can_cancel: boolean
}

export interface LocalTaskCancelAudit {
  requested_by: string
  source: string
  reason: string
  interruption_source?: 'supervisor_intervention' | 'node_restart' | 'updater_apply'
  requested_at_ms?: number
}

export interface LocalTaskEvent {
  seq: number
  type: string
  text: string
  at_ms?: number
  stream: string
  raw: Record<string, unknown>
}

export interface LocalTaskApproval {
  approval_id: string
  tool: string
  status: string
  decision: string
  actionable: boolean
  label: string
  meta: string
  checkpoint?: unknown
}

export interface LocalTaskSupervisionContract {
  protocol: string
  supervisor: string
  task_role: string
  parent_task_id?: string
  root_task_id?: string
  acceptance_criteria: string[]
  improvement_policy: string
}

export interface LocalTaskSupervisionReview {
  protocol: string
  verdict: string
  summary: string
  improvements: string[]
  reviewed_by: string
  reviewed_at_ms?: number
}

export interface LocalTaskSupervisionEvidence {
  event_count: number
  tool_calls: number
  tool_results: number
  failed_tools: number
  file_change_events: number
  changed_files: string[]
  command_exit_codes: Array<{ command: string; exit_code: number }>
  failure_summaries: string[]
  agent_messages: number
  terminal_event_seen: boolean
}

export interface LocalTaskRuntimeState {
  phase: string
  current_command: string
  last_progress?: number
  heartbeat?: number
  idle_duration: number
  timeout_policy: {
    mode: string
    total_timeout_secs: number
    idle_timeout_secs: number
    heartbeat_secs: number
    progress_aware: boolean
  }
}

export interface LocalTaskSupervisionState {
  protocol: string
  enabled: boolean
  contract?: LocalTaskSupervisionContract
  review?: LocalTaskSupervisionReview
  evidence: LocalTaskSupervisionEvidence
}

export interface LocalTaskUpdateRecovery {
  protocol: string
  update_id: string
  state: string
  state_reason: string
  original_task_id: string
  resume_task_id: string
  from_version: string
  from_git_sha: string
  to_version: string
  to_git_sha: string
  sidecar_session_id: string
  sidecar_output_offset: number
  sidecar_output_sequence: number
  journal_cursor: number
  transport_kind: string
  transport_protocol: string
  capabilities: string[]
  replay_from_cursor: boolean
  resume_strategy: string
  completion_event_id: string
  terminal_task_status: string
  terminal_finished_at_ms?: number
  expected_downtime_ms: number
  review_verdict: string
  review_summary: string
}

export interface LocalTaskResumeWorkspaceStatus {
  eligible: boolean
  derivation: string
  active_workspace_path: string
  branch: string
  git_head: string
  occupied: boolean
  reason: string
}

export interface LocalTaskContinuationInput {
  mode: 'resume' | 'supersede'
  prompt: string
  acceptance_criteria: string[]
  reason: string
}

export interface LocalTaskRecoveryTiming {
  mode: 'resume' | 'supersede'
  parent_task_id: string
  handoff_ms?: number
  resumed_work_ms: number
  total_since_parent_finished_ms?: number
  handoff_target_ms: number
  handoff_within_target?: boolean
}

export interface LocalTaskDetail {
  task: LocalTaskRecord
  events: LocalTaskEvent[]
  approvals: LocalTaskApproval[]
  last_event_seq: number
  has_more: boolean
  supervision: LocalTaskSupervisionState
  runtime: LocalTaskRuntimeState
  cancel_audit?: LocalTaskCancelAudit
  update_recovery?: LocalTaskUpdateRecovery
  resume_workspace_status?: LocalTaskResumeWorkspaceStatus
  recovery_timing?: LocalTaskRecoveryTiming
}

export interface LocalTaskStatusView {
  label: string
  tone: 'running' | 'done' | 'warning' | 'danger' | 'muted'
  terminal: boolean
}

export type LocalTaskApprovalDecision = 'approve' | 'deny'

export interface SelfEvolutionItem {
  logical_id: string
  root_task_id: string
  parent_task_id: string
  project_id: string
  conversation_id: string
  workspace_path: string
  execution_worktree?: string
  execution_branch?: string
  execution_isolated: boolean
  prompt: string
  status: string
  active_task_id?: string
  generation: number
  pause_reason?: string
  yield_reason?: string
  interruption_source?: 'supervisor_intervention' | 'node_restart' | 'updater_apply'
  review_verdict?: string
  review_note?: string
  reviewed_by?: string
  review_source?: string
  reviewed_at_ms?: number
  retry_count: number
  max_retries: number
  next_retry_at_ms?: number
  last_error?: string
  created_at_ms?: number
  updated_at_ms?: number
}

export interface SelfEvolutionGates {
  foreground_task_ids: string[]
  publish_active: boolean
  publish_status: string
  publish_owner?: string
  publish_waiter_count: number
  update_active: boolean
  resource_pressure: boolean
  checked_at_ms?: number
}

export interface SelfEvolutionQueue {
  items: SelfEvolutionItem[]
  gates: SelfEvolutionGates
}

export interface GlobalPublishLeaseEntry {
  kind: string
  sha: string
  batchId: string
  stage: string
  builderId: string
  builderLabel: string
  requestedAt?: number
  leaseExpiresAt?: number
}

export interface ReleaseBatchStage {
  stage: string
  kind: string
  status: string
  phase?: string
  phaseStatus?: string
  builderId: string
  builderLabel: string
  attempt: number
  requestedAt?: number
  lastHeartbeat?: number
  leaseExpiresAt?: number
  completedAt?: number
  errorMessage?: string
}

export interface ReleaseBatchLedger {
  batchId: string
  sha: string
  expectedStages: string[]
  status: string
  createdAt?: number
  updatedAt?: number
  stages: ReleaseBatchStage[]
}

export interface GlobalPublishStatus {
  owner?: GlobalPublishLeaseEntry
  waiters: GlobalPublishLeaseEntry[]
  waiterCount: number
  queuePolicy: string
  coalescingKey: string
  immutableReleaseSha: boolean
  batchIdentity: string
  stateHealth: string
  batches: ReleaseBatchLedger[]
}
