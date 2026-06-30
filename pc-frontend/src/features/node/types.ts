export interface NodeSummary {
  node_id?: string
  agent_id?: string
  short_id?: string
  display_name?: string
  device_name?: string
  label?: string
  online?: boolean
  capacity_label?: string
  capacity_tone?: string
  can_accept_project?: boolean | number | string
  project_count?: number
  project_limit?: number
  project_slots_remaining?: number
  disk_free_bytes?: number
  storage_ready?: boolean
  storage?: { enabled?: boolean; root_path?: string; disk_free_bytes?: number; git_base_url?: string }
  workspace_provision_ready?: boolean
  cli_project_ready?: boolean
  ai_cli_ready?: boolean
  allowed_clis?: string[]
  hardware?: {
    os?: string
    arch?: string
    cpu_brand?: string
    cpu_cores?: number
    memory_total_bytes?: number
    gpu_names?: string[]
    gpu_memory_total_bytes?: number
  }
  dev_runtime?: {
    workspace_root_path?: string
    workspace_root_writable?: boolean
    git_ready?: boolean
    dev_env_ready?: boolean
    toolchains?: { name: string; version: string; available: boolean }[]
    issues?: string[]
    api_runtime_ready?: boolean
    server_runtime_ready?: boolean
    route_a_ready?: boolean
    api_runtime_contract?: Record<string, unknown>
    server_runtime_status?: Record<string, unknown>
  }
  models?: { model_id?: string; display_name?: string; provider?: string; price_per_1k_credits?: string }[]
  capacity_warnings?: string[]
  registry_online?: boolean
  cli_connected?: boolean
  storage_repo_url_configured?: boolean
  created_at?: string
  connected_at?: number
}

export interface LocalNodeStatus {
  logged_in?: boolean
  connected?: boolean
  device_name?: string
  agent_id?: string
  version?: string
  hardware?: Record<string, unknown>
  storage?: Record<string, unknown>
  cloud_url?: string
  local_admin_token?: string
  local_admin_token_header?: string
  models?: NodeSummary['models']
  allowed_clis?: string[]
  cli_probe?: { refreshing?: boolean; refreshed_at_ms?: number; stale?: boolean }
  codex_cli?: LocalCliToolStatus | null
  cli_tools?: LocalCliToolStatus[]
  local_ai?: {
    cli_tools?: LocalCliToolStatus[]
    models?: NodeSummary['models']
  }
}

export interface LocalCliToolStatus {
  name?: string
  label?: string
  path?: string | null
  installed?: boolean
  runnable?: boolean
  logged_in?: boolean | null
  available?: boolean
  status?: 'checking' | 'ready' | 'not_installed' | 'not_runnable' | 'not_logged_in' | string
  detail?: string | null
  fix_action?: 'none' | 'wait' | 'install' | 'repair_path' | 'login' | string
  backend?: string
}

export interface AutostartStatus {
  ok?: boolean
  supported?: boolean
  enabled?: boolean
  source?: string
  run_value_name?: string
  expected_command?: string | null
  actual_command?: string | null
  install_error?: string | null
  summary?: string
  message?: string
}
