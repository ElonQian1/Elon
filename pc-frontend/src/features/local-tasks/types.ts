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

export interface LocalTaskDetail {
  task: LocalTaskRecord
  events: LocalTaskEvent[]
  approvals: LocalTaskApproval[]
  last_event_seq: number
  has_more: boolean
}

export interface LocalTaskStatusView {
  label: string
  tone: 'running' | 'done' | 'warning' | 'danger' | 'muted'
  terminal: boolean
}

export type LocalTaskApprovalDecision = 'approve' | 'deny'
