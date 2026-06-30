export interface ProjectAiNodeAuthorization {
  id: string
  project_id: string
  provider_user_id: string
  node_id: string
  allowed_clis: string[]
  permission_level: string
  enabled: boolean
  created_by_user_id: string
  created_at: string
  updated_at: string
}

export interface AvailableGroupAiNode {
  node_id: string
  provider_user_id: string
  display_name: string
  short_id: string
  online: boolean
  cli_connected: boolean
  allowed_clis: string[]
  authorized: boolean
  authorization?: ProjectAiNodeAuthorization | null
}

export interface ProjectAiBot {
  bot_id: string
  project_id: string
  provider_user_id: string
  node_id: string
  display_name: string
  runtime_route: string
  cli_name: string
  capabilities: string[]
  risk_level: string
  online: boolean
  cli_connected: boolean
}

export interface ProjectAiMatter {
  id: string
  project_id: string
  channel_id: string
  requester_user_id: string
  decision_user_id?: string | null
  source_message_id?: string | null
  title: string
  brief: string
  collaboration_mode: 'solo' | 'critic' | 'split' | string
  status: 'plan_ready' | 'running' | 'review_ready' | 'done' | 'canceled' | 'failed' | string
  participant_user_ids: string[]
  node_policy?: Record<string, unknown>
  acceptance_criteria: string[]
  plan?: Record<string, unknown>
  final_summary?: string | null
  final_decision?: string | null
  created_at: string
  updated_at: string
}

export interface ProjectAiMatterAssignment {
  id: string
  matter_id: string
  bot_id: string
  assignee_user_id?: string | null
  provider_user_id: string
  node_id: string
  role: string
  runtime_route: string
  cli_name: string
  worktree_path?: string | null
  branch_name?: string | null
  status: string
  result_summary?: string | null
  created_at: string
  updated_at: string
}

export interface ProjectAiEvent {
  id: string
  matter_id: string
  project_id: string
  actor_user_id?: string | null
  event_type: string
  payload?: Record<string, unknown> | null
  created_at: string
}

export interface MatterDetailResponse {
  ok: boolean
  matter: ProjectAiMatter
  assignments?: ProjectAiMatterAssignment[]
  events?: ProjectAiEvent[]
}

export interface AutomationMatterResponse extends MatterDetailResponse {
  scheduled_count?: number
  skipped_count?: number
  errors?: Array<{
    assignment_id: string
    role: string
    reason: string
  }>
}

export interface MatterEventsDeltaResponse {
  ok: boolean
  events?: ProjectAiEvent[]
  latest_event_id?: string | null
  latest_event_created_at?: string | null
  has_more?: boolean
}

export interface ProjectExecutionSession {
  id: string
  project_id: string
  conversation_id: string
  user_id: string
  node_id: string
  request_id: string
  base_workspace_path?: string | null
  active_workspace_path?: string | null
  branch?: string | null
  isolated: boolean
  status: string
  merge_status?: string | null
  last_error?: string | null
  model?: string | null
  prompt_tokens: number
  cached_input_tokens: number
  completion_tokens: number
  reasoning_tokens: number
  total_tokens: number
  token_usage_event_id?: string | null
  billing_event_id?: string | null
  created_at: string
  updated_at: string
}

export interface NodeComputeRun {
  id: string
  compute_call_id: string
  consumer_user_id: string
  provider_user_id?: string | null
  node_id: string
  model_id?: string | null
  feature: string
  usage_mode: string
  status: string
  started_at: string
  finished_at?: string | null
  duration_ms?: number | null
  prompt_tokens: number
  completion_tokens: number
  billed_cost_rmb_fen: number
  provider_earned_fen: number
  settlement_status?: string | null
  route_reason?: string | null
  error_message?: string | null
  created_at: string
  updated_at: string
}

export interface NodeQualityScore {
  node_id: string
  total_runs: number
  successful_runs: number
  failed_runs: number
  avg_duration_ms?: number | null
  last_finished_at?: string | null
  total_provider_earned_fen: number
  success_rate_x1000: number
}

export interface LocalDiffProbe {
  available: boolean
  reason?: string | null
  status_short: string[]
  diff_stat: string[]
}

export interface ArtifactMergeGuide {
  manual_merge_required: boolean
  worktree_path?: string | null
  branch_name?: string | null
  merge_status?: string | null
  can_server_diff: boolean
  recommended_action: string
}

export interface AssignmentArtifact {
  assignment: ProjectAiMatterAssignment
  latest_event?: ProjectAiEvent | null
  compute_call_id?: string | null
  pc_req_id?: string | null
  execution_session?: ProjectExecutionSession | null
  compute_run?: NodeComputeRun | null
  node_quality?: NodeQualityScore | null
  merge: ArtifactMergeGuide
  local_diff: LocalDiffProbe
}

export interface AssignmentArtifactResponse {
  ok: boolean
  artifact: AssignmentArtifact
}

export interface NodesResponse {
  ok: boolean
  project_id: string
  can_authorize_nodes?: boolean
  nodes?: AvailableGroupAiNode[]
}

export interface BotsResponse {
  ok: boolean
  project_id: string
  bots?: ProjectAiBot[]
}

export interface MattersResponse {
  ok: boolean
  project_id: string
  matters?: ProjectAiMatter[]
}

export interface CreateMatterPlanPayload {
  channelId: string
  title?: string
  brief: string
  collaborationMode: string
  acceptanceCriteria: string[]
}

export type AssignmentAction = 'run' | 'complete' | 'fail' | 'retry' | 'settlement'

export type MatterAutomationAction = 'run-all' | 'review'

export interface AssignmentActionPayload {
  comment?: string
  resultSummary?: string
  computeCallId?: string
  status?: string
  accountingStatus?: string
  billedCostRmbFen?: number
  providerEarnedFen?: number
}
