export type AiResourceClass =
  | 'own_codex'
  | 'remote_node'
  | 'shared_codex'
  | 'platform_model'

export interface AiResourcePolicy {
  project_id: string
  enabled_classes: AiResourceClass[]
  priority: AiResourceClass[]
  allow_fallback: boolean
  privacy_mode: 'prefer_local' | 'balanced' | 'prefer_available'
  max_estimated_unit_cost_micros?: number
  updated_by_user_id: string
  created_at: string
  updated_at: string
}

export interface AiResourceSummary {
  resource_id: string
  resource_class: AiResourceClass
  label: string
  provider: string
  model?: string
  availability: string
  execution_scope: string
  cost_basis: string
  quota_state: string
  task_kinds: string[]
  estimated_unit_cost_micros?: number
  evidence: string[]
}

export interface AiResourceOverview {
  schema: string
  project_id: string
  policy: AiResourcePolicy
  resources: AiResourceSummary[]
  cautions: string[]
}

export interface AiRoutePreview {
  schema: string
  project_id: string
  task_kind: string
  selected?: AiResourceSummary
  fallbacks: AiResourceSummary[]
  reasons: string[]
  execution_started: false
  quota_verified: false
}
