export interface DocumentMetadata {
  role: string
  lifecycle: string
  authority: string
  scope: string
  default_retrieval: boolean
  ambiguous: boolean
  confidence: string
  reason: string
  token_estimate: number
  content_hash: string
  headings: string[]
}

export interface ProjectDocumentEntry {
  title: string
  path: string
  size_bytes: number
  source: string
  metadata: DocumentMetadata
}

export interface DocumentBudget {
  classification_model_tokens: number
  estimated_full_read_tokens: number
  estimated_default_retrieval_tokens: number
  estimated_tokens_avoided: number
  ambiguous_documents: number
  excluded_by_default: number
}

export interface DocumentCatalog {
  project_id: string
  workspace: string
  revision: string
  source: string
  documents: ProjectDocumentEntry[]
  warnings: string[]
  can_edit: boolean
  access?: {
    mode: 'pc_node' | 'server_workspace' | 'server_fallback_read_only'
    degraded: boolean
    body_readable: boolean
    writable: boolean
  }
  budget: DocumentBudget
  analysis?: DocumentHealthAnalysis
}

export interface DocumentHealthIssue {
  fingerprint: string
  type: string
  severity: 'error' | 'warning' | 'info'
  path: string
  message: string
  evidence: string
  suggested_action: string
  confidence: number
  workflow?: {
    status: 'open' | 'assigned' | 'snoozed' | 'ignored' | 'resolved'
    owner: string
    due_at: string
    reason: string
    snoozed_until: string
    updated_at_ms: number
  }
  context?: { primary_topic: string; secondary_topics: string[]; scope_id: string }
}

export interface DocumentHealthAnalysis {
  version: number
  source: 'server'
  overall: { score: number; status: 'healthy' | 'review' | 'needs_attention' }
  architecture: {
    profile: string
    profile_label: string
    profile_source: 'manifest' | 'metadata'
    score: number
    status: 'healthy' | 'needs_attention' | 'needs_structure'
    topic_assigned_documents: number
    topic_unassigned_documents: number
    ambiguous_documents: number
    outdated_documents: number
    duplicate_titles: number
    home_configured: boolean
    foundation_coverage: Array<{ doc_type: string; label: string; covered: boolean }>
    missing_document_types: string[]
    findings: string[]
  }
  knowledge_maps?: ProjectKnowledgeMaps
  quality: {
    summary: {
      score: number
      status: string
      total_issues: number
      errors: number
      warnings: number
      info: number
      broken_links: number
      orphan_documents: number
      missing_owners: number
      missing_review_dates: number
      overdue_reviews: number
      implementation_conflicts: number
      external_links_checked: number
      external_links_pending: number
    }
    issues: DocumentHealthIssue[]
    returned_issues: number
    total_issues: number
    issue_types: string[]
  }
  governance_workflow?: {
    version: number
    summary: { open: number; assigned: number; snoozed: number; ignored: number; resolved: number; actionable: number; overdue: number }
    issues: DocumentHealthIssue[]
    returned_issues: number
    total_issues: number
    filters: { types: string[]; severities: string[]; owners: string[]; topics: string[]; scopes: string[]; statuses: string[] }
    trend: Array<{ created_at_ms: number; overall_score: number; architecture_score: number; quality_score: number; federation_score: number; issue_count: number; actionable_count: number }>
    score_explanation: {
      formula: string
      overall: number
      components: Array<{ key: string; label: string; score: number; weight: number; contribution: number }>
    }
  }
  maintenance: {
    index_version: number
    durable_queue: boolean
    poll_interval_seconds: number
    changed_documents: number
    pending_events: number
    processed_events: number
    last_indexed_at_ms: number
  }
  federation: {
    enabled: boolean
    source: 'manifest' | 'metadata'
    root_id: string
    node_count: number
    aggregated_score: number
    unhealthy_nodes: number
    max_depth: number
    nodes: Array<{
      id: string
      label: string
      parent_id: string
      scope_path: string
      profile: string
      owner: string
      include_globs?: string[]
      exclude_globs?: string[]
      document_count: number
      direct_children: number
      score: number
      status: string
      home_configured: boolean
    }>
  }
}

export interface DocumentFile {
  path: string
  content: string
  revision: string
  byte_len: number
  can_edit: boolean
  source?: 'pc_node' | 'server_workspace' | 'server_fallback'
  warnings?: string[]
}

export function roleLabel(role: string) {
  const labels: Record<string, string> = {
    policy: '共享规则', router: '入口路由', instruction: '按需指令', provider_adapter: '供应商桥接',
    project_guide: '项目导航', spec: '规范', architecture: '架构', requirement: '需求', runbook: '手册',
    agent_definition: 'Agent 定义', prompt_template: 'Prompt 模板', skill: 'Skill',
    decision: '决策', status: '状态', report: '证据', discussion: '讨论', archive: '归档', note: '笔记',
  }
  return labels[role] ?? role ?? '未分类'
}

export function lifecycleLabel(lifecycle: string) {
  const labels: Record<string, string> = {
    active: '当前有效', accepted: '已接受', draft: '草稿', deprecated: '已弃用',
    superseded: '已替代', archived: '历史', unclassified: '待整理',
  }
  return labels[lifecycle] ?? lifecycle
}

export function formatNumber(value: number) {
  return new Intl.NumberFormat('zh-CN').format(Number(value) || 0)
}
import type { ProjectKnowledgeMaps } from './projectDocumentKnowledgeGraphModel'
