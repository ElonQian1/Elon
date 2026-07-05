export interface ProjectGitWorktreeAuditResponse {
  project: ProjectGitWorktreeAuditProject
  workspace_path: string
  git_root?: string | null
  warnings?: string[]
  summary: ProjectGitWorktreeAuditSummary
  worktrees: ProjectGitWorktreeAuditEntry[]
}

export interface ProjectGitWorktreeAuditProject {
  id: string
  name: string
  workspace_path?: string | null
  node_id?: string | null
  role: string
}

export interface ProjectGitWorktreeAuditSummary {
  total_worktrees: number
  dirty_worktrees: number
  uncommitted_entries: number
  untracked_entries: number
  matched_worktrees: number
  unknown_dirty_worktrees: number
}

export interface ProjectGitWorktreeAuditEntry {
  path: string
  branch?: string | null
  head?: string | null
  detached?: boolean
  bare?: boolean
  current?: boolean
  has_uncommitted_changes: boolean
  uncommitted_count: number
  untracked_count: number
  modified_count: number
  staged_count: number
  status_preview?: string[]
  status_truncated?: boolean
  status_error?: string | null
  conversation?: ProjectGitWorktreeConversation | null
  recommended_action: string
}

export interface ProjectGitWorktreeConversation {
  conversation_id: string
  user_id: string
  user_account?: string | null
  title?: string | null
  conversation_status?: string | null
  message_count?: number | null
  task_count?: number | null
  last_task_status?: string | null
  execution_session_id?: string | null
  execution_status?: string | null
  merge_status?: string | null
  active_workspace_path?: string | null
  branch?: string | null
  updated_at?: string | null
  codex_thread_id?: string | null
  match_kind: string
  match_confidence: number
}
