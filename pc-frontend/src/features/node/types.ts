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
  agent_version?: string | null
  public_dev_enabled?: boolean
  public_dev_allowed_clis?: string[]
  public_dev_permission_level?: string
  public_dev_handshake_ready?: boolean
  public_dev_handshake_status?: string
  last_handshake_at?: string | null
  last_handshake_agent_version?: string | null
  last_handshake_allowed_clis?: string[]
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
  models?: { model_id?: string; display_name?: string; provider?: string; price_per_1k_credits?: number | string }[]
  compute_sharing?: NodeComputeSharingStatus
  capacity_warnings?: string[]
  owner_user_id?: string
  hardware_summary?: string
  route_a_ready?: boolean
  api_runtime_ready?: boolean
  server_runtime_ready?: boolean
  registry_online?: boolean
  cli_connected?: boolean
  storage_repo_url_configured?: boolean
  created_at?: string
  connected_at?: number
}

export interface NodeComputeSharingPolicy {
  node_id: string
  owner_user_id: string
  enabled: boolean
  allowed_model_ids: string[]
  max_concurrent_runs: number
  daily_token_limit: number
  created_at?: string | null
  updated_at?: string | null
}

export interface NodeComputeSharingStatus {
  policy: NodeComputeSharingPolicy
  active_runs: number
  tokens_used_today: number
  tokens_reserved_today: number
  available: boolean
  availability: string
}

export interface NodeComputeSharingRuntimeHealth {
  node_id: string
  status: 'healthy' | 'warning' | 'critical' | string
  completed_runs_24h: number
  failed_runs_24h: number
  budget_overrun_runs_24h: number
  budget_overrun_tokens_24h: number
  expired_active_runs: number
  attention_codes: string[]
  evaluated_at: string
}

export interface NodeComputeSharingResponse {
  ok?: boolean
  compute_sharing: NodeComputeSharingStatus
  runtime_health?: NodeComputeSharingRuntimeHealth
  observed_models?: NonNullable<NodeSummary['models']>
}

export interface NodeAgentVersion {
  version?: string
  gitSha?: string
  changelog?: string
  releaseNotes?: string
  changes?: string[]
  updated_at?: string
  downloadUrl?: string
  linuxDownloadUrl?: string
  windowsClientDownloadUrl?: string
  windowsInstallerDownloadUrl?: string
  ripgrepZipUrl?: string
  sha256?: string
  fileSha256?: string
  linuxSha256?: string
  windowsClientSha256?: string
  windowsInstallerSha256?: string
  ripgrepZipSha256?: string
  fileSize?: number
  linuxFileSize?: number
  windowsClientFileSize?: number
  windowsInstallerFileSize?: number
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
  active_slot_id?: string | null
  active_account_hint_hash?: string | null
  active_provider_user_id?: string | null
  active_provider_nickname?: string | null
  active_billing_source?: string | null
  active_lease_expires_at?: string | null
  managed_auth?: CodexVaultAuthInspection
  default_auth?: CodexVaultAuthInspection
  managed_slots?: CodexVaultManagedSlot[]
}

export interface CodexVaultManagedSlot {
  slot_id?: string
  account_hint_hash?: string | null
  lease_id?: string | null
  provider_user_id?: string | null
  provider_nickname?: string | null
  billing_source?: string | null
  lease_expires_at?: string | null
  active?: boolean
  home?: string
  auth?: CodexVaultAuthInspection
}

export interface CodexVaultCloudStatus {
  configured?: boolean
  bound?: boolean
  active_slot_id?: string | null
  available_count?: number
  auth_mode?: string | null
  account_hint_hash?: string | null
  source_device?: string | null
  credential_version?: number | null
  last_backup_at?: string | null
  last_lease_at?: string | null
  updated_at?: string | null
  slots?: CodexVaultCloudSlot[]
}

export interface CodexVaultCloudSlot {
  slot_id?: string
  auth_mode?: string | null
  account_hint_hash?: string | null
  source_device?: string | null
  credential_version?: number | null
  status?: string
  failure_count?: number
  last_backup_at?: string | null
  last_lease_at?: string | null
  last_failure_at?: string | null
  last_error?: string | null
  updated_at?: string | null
}

export interface CodexVaultStatusResponse {
  ok?: boolean
  local?: CodexVaultLocalStatus
  cloud?: {
    ok?: boolean
    vault?: CodexVaultCloudStatus
    sharing?: CodexVaultEmergencyStatus
    emergency?: CodexVaultEmergencyStatus
    error?: string
  }
  message?: string
  error?: string
}

export interface CodexVaultEmergencyStatus {
  grants?: CodexVaultEmergencyGrant[]
  leases?: CodexVaultEmergencyLease[]
  health?: CodexVaultSharingHealth
}

export interface CodexVaultSharingHealth {
  status?: 'ok' | 'warning' | 'critical' | string
  alert_count?: number
  active_lease_count?: number
  expired_uncleared_count?: number
  accounting_anomaly_count?: number
  unavailable_grant_count?: number
  recent_failed_event_count?: number
  alerts?: CodexVaultSharingAlert[]
}

export interface CodexVaultSharingAlert {
  code?: string
  severity?: 'warning' | 'critical' | string
  message?: string
  count?: number
}

export interface CodexVaultEmergencyGrant {
  id?: string
  provider_user_id?: string
  provider_account?: string
  provider_nickname?: string | null
  provider_avatar_data_url?: string | null
  consumer_user_id?: string
  consumer_account?: string
  consumer_nickname?: string | null
  consumer_avatar_data_url?: string | null
  status?: string
  label?: string | null
  purpose?: string | null
  max_lease_seconds?: number
  expires_at?: string | null
  created_at?: string
  updated_at?: string
  revoked_at?: string | null
  reciprocal_active?: boolean
  provider_vault_available?: boolean
}

export interface CodexVaultEmergencyLease {
  id?: string
  grant_id?: string
  provider_user_id?: string
  provider_account?: string
  provider_nickname?: string | null
  provider_avatar_data_url?: string | null
  consumer_user_id?: string
  consumer_account?: string
  consumer_nickname?: string | null
  consumer_avatar_data_url?: string | null
  consumer_node_id?: string
  provider_slot_id?: string
  account_hint_hash?: string | null
  purpose?: string | null
  billing_source?: string
  status?: string
  leased_at?: string
  expires_at?: string
  token_usage_event_id?: string | null
  billing_event_id?: string | null
  node_transaction_id?: string | null
  input_tokens?: number
  output_tokens?: number
  total_tokens?: number
  billed_cost_rmb_fen?: number
  provider_earned_fen?: number
  accounting_status?: string | null
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

export interface AiProviderLoginAttempt {
  schema_version?: number
  login_id: string
  provider_id: string
  flow: string
  state: 'starting' | 'waiting_for_user' | 'completed' | 'failed' | 'canceled' | 'expired' | string
  request_id?: string | null
  verification_url?: string | null
  user_code?: string | null
  auth_url?: string | null
  remote_compatible?: boolean
  recovered?: boolean
  error?: string | null
  error_code?: string | null
  started_at_ms?: number
  updated_at_ms?: number
}

export interface AiProviderAccount {
  id: string
  vendor: string
  label: string
  surface: 'cli_agent' | 'web_chat' | string
  protocol: string
  implementation_state: 'available' | 'reserved' | string
  official_login?: boolean
  login_flows?: string[]
  remote_login_supported?: boolean
  logout_supported?: boolean
  credential_owner?: string
  credential_storage?: string
  reason?: string | null
  enabled?: boolean
  blocked_reason_code?: string | null
  capabilities?: {
    login?: boolean
    logout?: boolean
    remote_login?: boolean
    idempotent_start?: boolean
    recoverable_status?: boolean
    credential_export?: boolean
    web_chat?: boolean
  }
  cli?: LocalCliToolStatus | null
  active_login?: AiProviderLoginAttempt | null
}

export interface AiProviderAccountsResponse {
  ok?: boolean
  schema?: string
  schema_version?: number
  transport?: { local?: string; ownerRelay?: string }
  providers: AiProviderAccount[]
}

export interface AiProviderLoginResponse {
  ok?: boolean
  schema?: string
  attempt: AiProviderLoginAttempt
  message?: string
}

export interface AutostartStatus {
  ok?: boolean
  supported?: boolean
  enabled?: boolean
  source?: string
  strategy?: string
  task_name?: string
  startup_shortcut_name?: string
  run_value_name?: string
  legacy_detected?: boolean
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

export interface NodeBalanceResponse {
  balance?: number
  balance_fen?: number
  lifetime_earned?: number
  lifetime_earned_fen?: number
  pending_payouts?: number
  pending_payout_fen?: number
  payout_min_fen?: number
  provider_revenue_share_x1000?: number
  provider_revenue_share_percent?: number
}

export interface NodeComputeRun {
  id?: string
  compute_call_id?: string
  consumer_user_id?: string
  provider_user_id?: string | null
  node_id?: string
  model_id?: string | null
  feature?: string
  usage_mode?: string
  status?: string
  started_at?: string
  finished_at?: string | null
  duration_ms?: number | null
  prompt_tokens?: number
  completion_tokens?: number
  reserved_token_budget?: number
  billed_cost_rmb_fen?: number
  provider_earned_fen?: number
  settlement_status?: string | null
  route_reason?: string | null
  error_message?: string | null
}

export interface NodeUsageResponse {
  consuming?: NodeComputeRun[]
  providing?: NodeComputeRun[]
}
