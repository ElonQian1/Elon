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
