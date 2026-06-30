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

export type AssignmentAction = 'complete' | 'fail' | 'retry' | 'settlement'

export interface AssignmentActionPayload {
  comment?: string
  resultSummary?: string
  computeCallId?: string
  status?: string
  accountingStatus?: string
  billedCostRmbFen?: number
  providerEarnedFen?: number
}
