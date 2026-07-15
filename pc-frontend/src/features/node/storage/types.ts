export interface NodeDataRootMigrationItem {
  kind?: 'workspace' | 'storage' | string
  source_path?: string
  target_path?: string | null
  exists?: boolean
  has_data?: boolean
  read_only_compatibility?: boolean
  strategy?: string
}

export interface NodeDataRootDiskStatus {
  total_bytes?: number
  free_bytes?: number
  available_bytes?: number
}

export interface NodeBuildCacheStatus {
  root?: string
  cache_bytes?: number
  temp_bytes?: number
  largest_project_rust_bytes?: number
  disk_free_bytes?: number
  disk_total_bytes?: number
  min_free_bytes?: number
  build_headroom_bytes?: number
  max_total_cache_bytes?: number
  max_project_rust_bytes?: number
  pressure?: boolean
  active_leases?: number
  last_cleanup_at_unix_secs?: number | null
  last_cleanup_reclaimed_bytes?: number
  captured_at_unix_secs?: number
}

export interface NodeCacheAdvisorCandidate {
  kind?: string
  label?: string
  path?: string
  source?: string
  scope?: string
  exists?: boolean
  managed_by_platform?: boolean
  automatic_action?: 'none' | string
  estimated_bytes?: number | null
  recommendation?: string
}

export interface NodeCacheAdvisorReport {
  schema_version?: number
  mode?: 'advisory' | string
  summary?: string
  candidates?: NodeCacheAdvisorCandidate[]
  suggestions?: string[]
}

export interface NodeDataRootStatus {
  configured?: boolean
  configuration_required?: boolean
  configuration_recommended?: boolean
  governance_mode?: 'advisory' | string
  source?: 'environment' | 'persisted' | 'unconfigured' | string
  root_path?: string | null
  workspace_root?: string | null
  storage_root?: string | null
  cache_root?: string | null
  temp_root?: string | null
  invalid_reason?: string | null
  migration_required?: boolean
  migration_plan?: NodeDataRootMigrationItem[]
  legacy_policy?: string
  warnings?: string[]
  capacity_warnings?: string[]
  active_task_count?: number
  active_tasks?: number | unknown[]
  disk?: NodeDataRootDiskStatus
  disk_total_bytes?: number
  disk_free_bytes?: number
  cache_bytes?: number
  temp_bytes?: number
  build_cache?: NodeBuildCacheStatus
  cache_advisor?: NodeCacheAdvisorReport
}

export interface NodeCacheAdvisorResponse {
  ok?: boolean
  cache_advisor?: NodeCacheAdvisorReport
  message?: string
}

export interface NodeDataRootSetResponse {
  ok?: boolean
  data_root?: NodeDataRootStatus
  restart_recommended?: boolean
  message?: string
}

export interface NodeDataRootCleanupEntry {
  kind?: 'cache' | 'temp' | string
  path?: string
  existed?: boolean
  estimated_bytes?: number
  removed?: boolean
}

export interface NodeDataRootCleanupResult {
  apply?: boolean
  estimated_bytes?: number
  entries?: NodeDataRootCleanupEntry[]
}

export interface NodeDataRootCleanupResponse {
  ok?: boolean
  cleanup?: NodeDataRootCleanupResult
  data_root?: NodeDataRootStatus
  message?: string
}
