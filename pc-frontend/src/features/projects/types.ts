export interface NodeCapacity {
  capacity_tone?: string
  capacity_label?: string
  capacity_warnings?: string[]
  can_accept_project?: boolean | number | string
  project_slots_remaining?: number
  project_count?: number
  project_limit?: number
  disk_free_bytes?: number
  hardware_summary?: string
  workspace_provision_ready?: boolean
  allowed_clis?: string[]
  storage_ready?: boolean
  storage?: { enabled?: boolean }
  storage_repo_url_configured?: boolean
}

export interface ProjectNode extends NodeCapacity {
  node_id?: string
  agent_id?: string
  id?: string
  owner_user_id?: string
  short_id?: string
  display_name?: string
  label?: string
  device_name?: string
  online?: boolean
}

export interface CreateProjectPayload {
  name: string
  description?: string | null
  template: string
  repo_url?: string | null
  branch?: string | null
  execution_target: 'pc_node' | 'server'
  node_id?: string
  storage_node_id?: string | null
  skip_storage: boolean
}

export interface CreatedProject {
  id?: string
  name?: string
}

export interface CreateProjectResult {
  project?: CreatedProject
}
