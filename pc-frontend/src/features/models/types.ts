/** 对应旧 pc_app_models.js 的数据结构 */

export interface AgentOption {
  label: string
  agentName: string
  provider: string
  backend: string
  modelId: string
  reasoningEffort: string
  reasoningSummary: string
  verbosity: string
  subtitle: string
  source?: 'server' | 'local_cli' | 'local_model'
  selectable?: boolean
  unavailableReason?: string
}

export interface RawAgentItem {
  name?: string
  provider?: string
  backend?: string
  model?: string
  display_model?: string
  label?: string
  reasoning_effort?: string
  reasoning_summary?: string
  verbosity?: string
  api_base?: string
}

export interface AgentConfig {
  use_agent?: string
  api_base?: string
  model?: string
}

export interface AgentConfigResponse {
  codex_cli_only?: boolean
  user_byok_api_enabled?: boolean
  available_agents?: RawAgentItem[]
  config?: AgentConfig
  default_agent?: string
}
