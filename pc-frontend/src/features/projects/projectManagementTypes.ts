export interface WorkspaceHealth {
  workspace_exists?: boolean
  git_initialized?: boolean
  git_remote?: string
  node_online?: boolean
  node_id?: string
  disk_free_bytes?: number
  issues?: string[]
  cli_ready?: boolean
  can_run_on_pc?: boolean
  verified_can_run_on_pc?: boolean | null
  warnings?: string[]
  project?: {
    workspace_path?: string | null
    node_id?: string | null
  }
  node?: {
    node_id?: string
    online?: boolean
    cli_connected?: boolean
    cli_project_ready?: boolean
  } | null
  live_inspect?: {
    is_git_worktree?: boolean
    git_remote_origin?: string | null
    disk_free_bytes?: number | null
  } | null
}

export interface ProjectGitStatus {
  project_id?: string
  workspace?: string
  git?: {
    has_git?: boolean
    origin?: string | null
    branch?: string | null
    remote_check?: string | null
  }
  deploy_key?: {
    exists?: boolean
    public_key?: string | null
    github_deploy_keys_url?: string
  }
  workflow?: unknown
}

export interface ProjectRelease {
  id: string
  project_id: string
  task_id?: string | null
  uploaded_by?: string | null
  version_name?: string | null
  package_name?: string | null
  version_code?: number | null
  channel?: string | null
  status?: string | null
  apk_url?: string | null
  file_name?: string | null
  sha256?: string | null
  size_bytes?: number | null
  changelog?: string | null
  build_started_at?: string | null
  source_git_sha?: string | null
  source_worktree?: string | null
  created_at?: string
  updated_at?: string
}
