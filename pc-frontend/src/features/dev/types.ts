/** 对应旧 pc_app_dev_tasks.js + pc_app_agent_runs.js 的共享类型 */

export type TaskTone = 'queued' | 'running' | 'done' | 'failed' | 'canceled' | 'approval' | 'muted'

export interface TaskState {
  taskId: string
  progressCount: number
  result: ChatMessage | null
  request: string
  resultText: string
  failed: boolean
  canceled: boolean
  status: string
  error: string
  apkUrl: string
  attach: Record<string, unknown> | null
  resume: Record<string, unknown> | null
  approvalState: Record<string, unknown> | null
  pcReqId: string
  lastEventSeq: number
}

export interface ChatMessage {
  kind?: string
  role?: string
  message_kind?: string
  content?: string
  text?: string
  message?: string
  task_id?: string
  taskId?: string
  task_status?: string
  taskStatus?: string
  task_error?: string
  taskError?: string
  task_apk_url?: string
  taskApkUrl?: string
  [key: string]: unknown
}

export interface ToolEvent {
  type: string
  id?: string
  tool?: string
  args?: Record<string, unknown>
  result?: string
  status?: string
  decision?: string
  approval_id?: string
  diff?: { preview?: string; files?: string[]; truncated?: boolean }
  phase?: string
  runtime?: string
  turn?: number
  message?: string
  total_tools?: number
  failed_tools?: number
  [key: string]: unknown
}

export interface ApprovalState {
  status: string
  tool?: string
  tone: TaskTone
  label: string
  meta: string
  detail?: string
  nextAction?: string
  requiresNewTask?: boolean
}

export interface TaskContext {
  tasks: Map<string, TaskState>
  approvals: Map<string, ApprovalState>
}

/* Agent Runs */
export interface AgentRunEntry {
  run_id?: string
  runId?: string
  file_name?: string
  fileName?: string
  status?: string
  mode?: string
  turn_count?: number
  turnCount?: number
  tool_count?: number
  toolCount?: number
  tool_names?: string[]
  updated_at?: string
  updatedAt?: string
  started_at?: string
  startedAt?: string
  last_error?: string
  lastError?: string
}

export interface SidecarCapabilities {
  terminal_attach?: boolean
  output_stream_replay?: boolean
  terminal_input?: boolean
  terminal_resize?: boolean
  tool_approval_recovery?: boolean
  cancel?: boolean
}

export interface SidecarSession {
  session_id?: string
  sessionId?: string
  task_id?: string
  taskId?: string
  cli_name?: string
  cliName?: string
  route?: string
  state?: string
  transport?: string
  endpoint?: string | null
  sidecar_pid?: number
  sidecarPid?: number
  child_pid?: number
  childPid?: number
  started_at_ms?: number
  startedAtMs?: number
  last_seen_at_ms?: number
  lastSeenAtMs?: number
  attachable_after_restart?: boolean
  attachableAfterRestart?: boolean
  approval_recoverable_after_restart?: boolean
  approvalRecoverableAfterRestart?: boolean
  capabilities?: SidecarCapabilities
}

export interface SidecarOutputRecord {
  type?: string
  record_type?: string
  stream?: string | null
  text?: string | null
  child_pid?: number | null
  exit_code?: number | null
  success?: boolean | null
  canceled?: boolean | null
  error?: string | null
  at_ms?: number
}

export interface SidecarAttachResponse {
  ok?: boolean
  task_id?: string
  attached?: boolean
  session?: SidecarSession
  output_records?: SidecarOutputRecord[]
  next_offset?: number
  transport?: string
  error?: string
}

export interface AgentRunsData {
  runs: AgentRunEntry[]
  activeControls: Record<string, unknown>[]
  recentTasks: Record<string, unknown>[]
  sidecarSessions: SidecarSession[]
  recoveryEntry: Record<string, unknown> | null
  logDir: string
  workspacePath: string
  loadedAt: number
  error?: string
}
