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
}

export interface LocalTaskSupervisionInput {
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
  terminal_event_seen: boolean
}

export interface LocalTaskSupervisionState {
  protocol: string
  enabled: boolean
  contract?: LocalTaskSupervisionContract
  review?: LocalTaskSupervisionReview
  evidence: LocalTaskSupervisionEvidence
}

export interface LocalTaskDetail {
  task: LocalTaskRecord
  events: LocalTaskEvent[]
  approvals: LocalTaskApproval[]
  last_event_seq: number
  has_more: boolean
  supervision: LocalTaskSupervisionState
}

export interface LocalTaskStatusView {
  label: string
  tone: 'running' | 'done' | 'warning' | 'danger' | 'muted'
  terminal: boolean
}

export type LocalTaskApprovalDecision = 'approve' | 'deny'
