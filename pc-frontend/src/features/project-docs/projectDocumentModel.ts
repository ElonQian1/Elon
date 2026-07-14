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

interface DocumentBudget {
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
  budget: DocumentBudget
}

export interface DocumentFile {
  path: string
  content: string
  revision: string
  byte_len: number
  can_edit: boolean
}

interface DocumentSection {
  key: string
  label: string
  detail: string
  test: (document: ProjectDocumentEntry) => boolean
}

export const DOCUMENT_SECTIONS: DocumentSection[] = [
  {
    key: 'required',
    label: '必须文档',
    detail: '跨供应商入口与共享硬规则',
    test: (document) => ['policy', 'router'].includes(document.metadata.role),
  },
  {
    key: 'on-demand',
    label: '按需文档',
    detail: '领域指令、项目导航和供应商桥接',
    test: (document) => ['instruction', 'project_guide', 'provider_adapter'].includes(document.metadata.role),
  },
  {
    key: 'current',
    label: '当前知识',
    detail: '规范、架构、需求和操作手册',
    test: (document) => ['spec', 'architecture', 'requirement', 'runbook'].includes(document.metadata.role)
      && document.metadata.lifecycle === 'active',
  },
  {
    key: 'drafts',
    label: '笔记与讨论',
    detail: '想法、草稿和未批准需求',
    test: (document) => ['discussion', 'note'].includes(document.metadata.role)
      && document.metadata.lifecycle !== 'archived',
  },
  {
    key: 'evidence',
    label: '状态与证据',
    detail: '报告只能证明结果，不能定义需求',
    test: (document) => ['status', 'report'].includes(document.metadata.role),
  },
  {
    key: 'decisions',
    label: '决策记录',
    detail: '已接受的架构决策和原因',
    test: (document) => document.metadata.role === 'decision',
  },
  {
    key: 'archive',
    label: '历史归档',
    detail: '默认不进入当前实现检索',
    test: (document) => document.metadata.lifecycle === 'archived' || document.metadata.role === 'archive',
  },
  {
    key: 'unclassified',
    label: '等待整理',
    detail: '程序无法确认权威性的文档',
    test: (document) => document.metadata.ambiguous,
  },
]

export function buildOrganizationPrompt(projectName: string, catalog: DocumentCatalog) {
  const lines = catalog.documents.map((document) => [
    document.path,
    document.metadata.role,
    document.metadata.lifecycle,
    document.metadata.authority,
    retrievalMode(document),
    document.metadata.ambiguous ? 'ambiguous' : document.metadata.confidence,
  ].join(' | '))
  let compactCatalog = lines.join('\n')
  const maxChars = 12_000
  if (compactCatalog.length > maxChars) {
    compactCatalog = `${compactCatalog.slice(0, maxChars)}\n…目录因预算截断，请先处理已列出的歧义文档。`
  }
  return `请为项目“${projectName}”执行一次低 token 文档治理实验。\n\n` +
    '程序已经仅用路径、标题、元数据和哈希完成预分类，classification_model_tokens=0；不要重新全文扫描所有 Markdown。' +
    '本轮只生成分类、权威冲突、建议目录和迁移顺序，不移动、删除或改写文档。' +
    '优先检查标记为 ambiguous 的文档；需要内容时先读标题和目录，仍无法判断才读取正文。' +
    '必须保持 AGENTS.md + .github/copilot-instructions.md + .github/instructions/ 的跨供应商必读/按需结构，' +
    '不得为 Codex、Claude、Gemini、Copilot 复制多套规则。最终报告实际读取的文档数、估算 token、冲突和建议。\n\n' +
    `紧凑目录（path | role | lifecycle | authority | retrieval | confidence）：\n${compactCatalog}`
}

function retrievalMode(document: ProjectDocumentEntry) {
  if (document.metadata.default_retrieval) return 'required'
  if (
    ['archived', 'draft', 'deprecated', 'superseded', 'unclassified'].includes(document.metadata.lifecycle)
    || ['archive', 'discussion', 'report', 'status', 'note'].includes(document.metadata.role)
  ) return 'excluded'
  return 'on_demand'
}

export function roleLabel(role: string) {
  const labels: Record<string, string> = {
    policy: '共享规则', router: '入口路由', instruction: '按需指令', provider_adapter: '供应商桥接',
    project_guide: '项目导航', spec: '规范', architecture: '架构', requirement: '需求', runbook: '手册',
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
