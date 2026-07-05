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
  lifecycle?: NodeLifecycleReport | null
  models?: { model_id?: string; display_name?: string; provider?: string; price_per_1k_credits?: string }[]
  capacity_warnings?: string[]
  registry_online?: boolean
  cli_connected?: boolean
  storage_repo_url_configured?: boolean
  created_at?: string
  connected_at?: number
}

export interface NodeAgentVersion {
  version?: string
  gitSha?: string
  updated_at?: string
  downloadUrl?: string
  linuxDownloadUrl?: string
  windowsClientDownloadUrl?: string
  ripgrepZipUrl?: string
  ripgrepZipSha256?: string
  fileSize?: number
  linuxFileSize?: number
  windowsClientFileSize?: number
  ripgrepZipFileSize?: number
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
  cloud_http_url?: string
  cloud_network?: {
    cloudWsMode?: string
    cloudHttpMode?: string
    proxyDefault?: string
    userProxyOptIn?: string
    cloudHostsNoProxy?: string[]
    noProxyEnv?: string
  }
  download_router?: {
    ok?: boolean
    routerVersion?: string
    profile?: {
      enabled?: boolean
      mode?: string
      failOpen?: boolean
      cacheMinutes?: number
      updatedAt?: string
    }
    availableModes?: string[]
    wrapperPolicy?: string
    traceScope?: string
  }
  runtime_policy?: Record<string, unknown>
  local_admin_token?: string
  local_admin_token_header?: string
  lifecycle_report_schema_version?: number
  lifecycle?: NodeLifecycleReport | null
  last_event?: string
  models?: NodeSummary['models']
  allowed_clis?: string[]
  cli_probe?: { refreshing?: boolean; refreshed_at_ms?: number; stale?: boolean }
  codex_cli?: LocalCliToolStatus | null
  codex_toolbox?: CodexToolboxStatus | null
  codex_vault?: CodexVaultLocalStatus | null
  cli_tools?: LocalCliToolStatus[]
  local_ai?: {
    cli_tools?: LocalCliToolStatus[]
    models?: NodeSummary['models']
  }
}

export interface NodeLifecycleReport {
  schema?: string
  session_id?: string | null
  state?: string
  severity?: 'ok' | 'warning' | 'danger' | string
  started_at_ms?: number | null
  heartbeat_at_ms?: number | null
  heartbeat_age_ms?: number | null
  connected?: boolean
  logged_in?: boolean
  last_event?: string | null
  previous_session_id?: string | null
  previous_exit_kind?: string | null
  previous_exit_reason?: string | null
  previous_heartbeat_at_ms?: number | null
  previous_heartbeat_age_ms?: number | null
  active_task_count?: number
  sidecar_session_count?: number
  restart_recovery?: boolean
  recommended_action?: string
  summary?: string
}

export interface CodexToolboxStatus {
  ok?: boolean
  codex_program?: string | null
  path_scope?: string
  summary?: string
  tools?: CodexToolboxTool[]
}

export interface CodexToolboxTool {
  id?: string
  name?: string
  aliases?: string[]
  tier?: 'core' | 'profile' | 'optional' | string
  install_policy?: 'AutoSmall' | 'ManualRepair' | 'NeverAuto' | string
  env_path_var?: string
  managed_dir?: string
  installed?: boolean
  runnable?: boolean
  status?: 'ready' | 'missing' | 'not_runnable' | string
  source?: string
  path?: string | null
  version?: string | null
  reason?: string | null
  candidate_count?: number
  will_inject?: boolean
  repair_action?: 'install_env_codex' | 'manual_repair' | 'none' | string
  detail?: string
}

export interface CodexVaultAuthInspection {
  present?: boolean
  path?: string | null
  auth_mode?: string | null
  has_refresh_token?: boolean
  account_hint_hash?: string | null
  problem?: string | null
}

export interface CodexVaultLocalStatus {
  managed_home?: string
  active_codex_home?: string | null
  active_home_managed?: boolean
  managed_auth?: CodexVaultAuthInspection
  default_auth?: CodexVaultAuthInspection
}

export interface CodexVaultCloudStatus {
  configured?: boolean
  bound?: boolean
  auth_mode?: string | null
  account_hint_hash?: string | null
  source_device?: string | null
  credential_version?: number | null
  last_backup_at?: string | null
  last_lease_at?: string | null
  updated_at?: string | null
}

export interface CodexVaultStatusResponse {
  ok?: boolean
  local?: CodexVaultLocalStatus
  cloud?: {
    ok?: boolean
    vault?: CodexVaultCloudStatus
    error?: string
  }
  message?: string
  error?: string
}

export interface LocalCliToolStatus {
  name?: string
  label?: string
  path?: string | null
  version?: string | null
  installed?: boolean
  runnable?: boolean
  logged_in?: boolean | null
  available?: boolean
  status?: 'checking' | 'ready' | 'not_installed' | 'not_runnable' | 'not_logged_in' | string
  detail?: string | null
  reason?: string | null
  diagnosis?: string | null
  fix_hint?: string | null
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

export interface ClientMaintenanceAction {
  id?: string
  kind?: 'open_target' | 'export_diagnostics' | 'repair' | 'update' | 'uninstall' | string
  label?: string
  description?: string
  target?: string
  enabled?: boolean
  tone?: 'primary' | 'neutral' | 'danger' | string
  confirmation?: string | null
  recommended?: boolean
  recommendation?: string
}

export interface ClientMaintenanceStatus {
  ok?: boolean
  platform?: string
  supported?: boolean
  version?: string
  installed?: boolean
  running_from_install_dir?: boolean
  installed_git_sha?: string
  installed_package_version?: string
  version_manifest?: NodeAgentVersion
  layout_status?: string
  product_status?: { status?: string; summary?: string }
  maintenance_overview?: {
    status?: string
    severity?: string
    title?: string
    detail?: string
    primary_action_id?: string
    primary_action_label?: string
    recent_failure_count?: number
  }
  primary_maintenance_action?: ClientMaintenanceAction
  maintenance_actions?: ClientMaintenanceAction[]
  maintenance_recent_events?: Array<{ action?: string; ok?: boolean; detail?: string; ts?: number }>
  message?: string
  error?: string
}
