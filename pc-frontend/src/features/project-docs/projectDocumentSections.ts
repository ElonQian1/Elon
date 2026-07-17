import type { DocumentCatalog, ProjectDocumentEntry } from './projectDocumentModel'

export const SECTION_CONFIG_PATH = '.elon/document-sections.json'
export const ORGANIZATION_SUGGESTIONS_PATH = '.elon/document-organization-suggestions.json'

export interface CustomDocumentSection {
  id: string
  label: string
  detail: string
  color: string
  parent_id: string
  order: number
  icon: string
  entrypoint: string
}

export interface DocumentKnowledgeHome {
  title: string
  summary: string
  entrypoint: string
  start_here: string[]
}

export interface DocumentKnowledgeMetadata {
  id: string
  doc_type: string
  audience: string[]
  owner: string
  owners: string[]
  reviewed_at: string
  review_interval_days: number
  implementation_refs: string[]
  version: string
  related: string[]
  supersedes: string[]
  order: number
  pinned: boolean
}

export interface DocumentOrganizationAuditEntry {
  id: string
  action: string
  target: string
  summary: string
  at: string
}

export interface DocumentSectionManifest {
  version: 1
  profile: string
  home: DocumentKnowledgeHome
  sections: CustomDocumentSection[]
  assignments: Record<string, string>
  governance_overrides: Record<string, string>
  document_metadata: Record<string, DocumentKnowledgeMetadata>
  audit_log: DocumentOrganizationAuditEntry[]
}

export interface DocumentSection {
  key: string
  label: string
  detail: string
  color: string
  custom?: boolean
  virtual?: boolean
  template?: boolean
  parentId?: string
  order?: number
  icon?: string
  entrypoint?: string
  depth?: number
}

export interface SuggestedAssignment {
  path: string
  section_id: string
  reason: string
}

export interface SuggestedFileOperation {
  id: string
  kind: 'rename' | 'move'
  source_path: string
  target_path: string
  source_revision: string
  reason: string
  status: 'proposed' | 'applied'
}

export type DocumentAutomationMode = 'git_backed_full' | 'trusted_reversible' | 'review_all' | 'suggestions_only'

export interface DocumentOrganizationSuggestions {
  version: 1
  status: 'requested' | 'ready' | 'applied'
  summary: string
  proposed_profile: string
  proposed_home: DocumentKnowledgeHome | null
  proposed_sections: CustomDocumentSection[]
  assignments: SuggestedAssignment[]
  conflicts: string[]
  move_suggestions: string[]
  architecture_findings: string[]
  missing_document_types: string[]
  document_metadata: Record<string, DocumentKnowledgeMetadata>
  file_operations: SuggestedFileOperation[]
  documents_read: number
  estimated_tokens_used: number
}

export const EMPTY_SECTION_MANIFEST: DocumentSectionManifest = {
  version: 1,
  profile: 'auto',
  home: { title: '', summary: '', entrypoint: '', start_here: [] },
  sections: [],
  assignments: {},
  governance_overrides: {},
  document_metadata: {},
  audit_log: [],
}

export const SYSTEM_DOCUMENT_SECTIONS: DocumentSection[] = [
  { key: 'required', label: '必须文档', detail: '跨供应商入口与共享硬规则', color: '#b889ff' },
  { key: 'on-demand', label: '按需文档', detail: '领域指令、项目导航和说明', color: '#6ca7ff' },
  { key: 'current', label: '当前知识', detail: '规范、架构、需求和操作手册', color: '#62c891' },
  { key: 'customizations', label: 'AI 定制资产', detail: 'Agent、Prompt 和 Skill，按任务加载', color: '#53a9c9' },
  { key: 'drafts', label: '笔记与讨论', detail: '想法、草稿和未批准需求', color: '#e6aa58' },
  { key: 'evidence', label: '状态与证据', detail: '报告只能证明结果，不能定义需求', color: '#60c5ca' },
  { key: 'decisions', label: '决策记录', detail: '已接受的架构决策和原因', color: '#cf83e5' },
  { key: 'archive', label: '历史归档', detail: '默认不进入当前实现检索', color: '#747984' },
  { key: 'unclassified', label: '等待整理', detail: '程序无法确认权威性的文档', color: '#df6f77' },
]

export const SUGGESTIONS_SECTION: DocumentSection = {
  key: 'suggestions',
  label: 'AI 整理建议',
  detail: '先审核建议，再应用虚拟分区',
  color: '#9b73ed',
  virtual: true,
}

export function buildDocumentSections(manifest: DocumentSectionManifest): DocumentSection[] {
  const custom = [...manifest.sections].sort((left, right) => left.order - right.order || left.label.localeCompare(right.label, 'zh-CN')).map((section) => ({
    key: customSectionKey(section.id),
    label: section.label,
    detail: section.detail || '用户自定义项目分区',
    color: section.color || '#7b8aa5',
    custom: true,
    parentId: section.parent_id ? customSectionKey(section.parent_id) : undefined,
    order: section.order,
    icon: section.icon,
    entrypoint: section.entrypoint,
  }))
  return [...SYSTEM_DOCUMENT_SECTIONS, ...custom, SUGGESTIONS_SECTION]
}

export function governanceSectionForDocument(
  document: ProjectDocumentEntry,
  manifest: DocumentSectionManifest,
): string {
  const assigned = manifest.governance_overrides[normalizedPath(document.path)]
  if (assigned && SYSTEM_DOCUMENT_SECTIONS.some((section) => section.key === assigned)) return assigned
  return automaticGovernanceSection(document)
}

export function sectionForDocument(
  document: ProjectDocumentEntry,
  manifest: DocumentSectionManifest,
): string {
  const assigned = manifest.assignments[normalizedPath(document.path)]
  if (assigned?.startsWith('custom:') && manifest.sections.some((section) => customSectionKey(section.id) === assigned)) return assigned
  return governanceSectionForDocument(document, manifest)
}

function automaticGovernanceSection(document: ProjectDocumentEntry): string {
  const { role, lifecycle, ambiguous } = document.metadata
  if (lifecycle === 'archived' || role === 'archive') return 'archive'
  if (['policy', 'router'].includes(role)) return 'required'
  if (['agent_definition', 'prompt_template', 'skill'].includes(role)) return 'customizations'
  if (['instruction', 'project_guide', 'provider_adapter', 'guide'].includes(role)) return 'on-demand'
  if (['spec', 'architecture', 'requirement', 'runbook'].includes(role) && lifecycle === 'active') return 'current'
  if (role === 'decision') return 'decisions'
  if (['status', 'report'].includes(role)) return 'evidence'
  if (ambiguous || lifecycle === 'unclassified') return 'unclassified'
  if (lifecycle === 'draft' || ['discussion', 'note'].includes(role)) return 'drafts'
  return 'unclassified'
}

export function parseSectionManifest(content: string): DocumentSectionManifest {
  if (!content.trim()) return cloneEmptyManifest()
  try {
    const value = JSON.parse(content) as Partial<DocumentSectionManifest>
    const sections = Array.isArray(value.sections)
      ? value.sections.map(sanitizeCustomSection).filter((section): section is CustomDocumentSection => !!section)
      : []
    const customSectionKeys = new Set(sections.map((section) => customSectionKey(section.id)))
    const governanceSectionKeys = new Set(SYSTEM_DOCUMENT_SECTIONS.map((section) => section.key))
    const assignments: Record<string, string> = {}
    const governanceOverrides: Record<string, string> = {}
    if (value.assignments && typeof value.assignments === 'object') {
      for (const [path, section] of Object.entries(value.assignments)) {
        const normalized = normalizedPath(path)
        if (typeof section !== 'string' || !normalized) continue
        if (customSectionKeys.has(section)) assignments[normalized] = section
        else if (governanceSectionKeys.has(section)) governanceOverrides[normalized] = section
      }
    }
    if (value.governance_overrides && typeof value.governance_overrides === 'object') {
      for (const [path, section] of Object.entries(value.governance_overrides)) {
        const normalized = normalizedPath(path)
        if (typeof section === 'string' && governanceSectionKeys.has(section) && normalized) {
          governanceOverrides[normalized] = section
        }
      }
    }
    const documentMetadata: Record<string, DocumentKnowledgeMetadata> = {}
    if (value.document_metadata && typeof value.document_metadata === 'object') {
      for (const [path, metadata] of Object.entries(value.document_metadata)) {
        const normalized = normalizedPath(path)
        const sanitized = sanitizeKnowledgeMetadata(metadata)
        if (normalized && sanitized) documentMetadata[normalized] = sanitized
      }
    }
    return {
      version: 1,
      profile: sanitizeProfile(value.profile),
      home: sanitizeKnowledgeHome(value.home),
      sections,
      assignments,
      governance_overrides: governanceOverrides,
      document_metadata: documentMetadata,
      audit_log: sanitizeAuditLog(value.audit_log),
    }
  } catch {
    return cloneEmptyManifest()
  }
}

export function parseOrganizationSuggestions(content: string): DocumentOrganizationSuggestions | null {
  if (!content.trim()) return null
  try {
    const value = JSON.parse(content) as Partial<DocumentOrganizationSuggestions>
    const status = ['requested', 'ready', 'applied'].includes(value.status ?? '')
      ? value.status as DocumentOrganizationSuggestions['status']
      : 'requested'
    const proposedSections = uniqueSections(Array.isArray(value.proposed_sections)
      ? value.proposed_sections.map(sanitizeCustomSection).filter((section): section is CustomDocumentSection => !!section)
      : []).slice(0, 16)
    const assignments = Array.isArray(value.assignments)
      ? value.assignments.slice(0, 500).flatMap((assignment) => {
        if (!assignment || typeof assignment !== 'object') return []
        const candidate = assignment as Partial<SuggestedAssignment>
        const path = normalizedPath(candidate.path ?? '')
        const sectionId = String(candidate.section_id ?? '').trim()
        if (!path || !sectionId) return []
        return [{ path, section_id: sectionId, reason: String(candidate.reason ?? '').slice(0, 500) }]
      })
      : []
    const fileOperations = Array.isArray(value.file_operations)
      ? value.file_operations.slice(0, 100).flatMap((operation) => {
        if (!operation || typeof operation !== 'object') return []
        const candidate = operation as Partial<SuggestedFileOperation>
        const id = String(candidate.id ?? '').trim().slice(0, 80)
        const kind = candidate.kind === 'rename' || candidate.kind === 'move' ? candidate.kind : null
        const sourcePath = normalizedPath(candidate.source_path ?? '')
        const targetPath = normalizedPath(candidate.target_path ?? '')
        if (!id || !kind || !sourcePath || !targetPath || sourcePath.toLowerCase() === targetPath.toLowerCase()) return []
        return [{
          id,
          kind,
          source_path: sourcePath,
          target_path: targetPath,
          source_revision: String(candidate.source_revision ?? '').trim().slice(0, 128),
          reason: String(candidate.reason ?? '').trim().slice(0, 500),
          status: candidate.status === 'applied' ? 'applied' as const : 'proposed' as const,
        }]
      })
      : []
    return {
      version: 1,
      status,
      summary: String(value.summary ?? '').slice(0, 4000),
      proposed_profile: sanitizeProfile(value.proposed_profile),
      proposed_home: value.proposed_home ? sanitizeKnowledgeHome(value.proposed_home) : null,
      proposed_sections: proposedSections,
      assignments,
      conflicts: stringArray(value.conflicts, 100),
      move_suggestions: stringArray(value.move_suggestions, 100),
      architecture_findings: stringArray(value.architecture_findings, 100),
      missing_document_types: stringArray(value.missing_document_types, 100).map((entry) => entry.slice(0, 120)),
      document_metadata: sanitizeKnowledgeMetadataMap(value.document_metadata),
      file_operations: fileOperations,
      documents_read: safeNonNegativeNumber(value.documents_read),
      estimated_tokens_used: safeNonNegativeNumber(value.estimated_tokens_used),
    }
  } catch {
    return null
  }
}

export function createCustomSection(
  label: string,
  existing: CustomDocumentSection[],
  parentId = '',
): CustomDocumentSection {
  const base = sanitizeSectionId(label) || `section-${Date.now()}`
  let id = base
  let suffix = 2
  while (existing.some((section) => section.id === id)) id = `${base}-${suffix++}`
  return {
    id, label: label.trim().slice(0, 40), detail: '用户自定义项目分区', color: '#7f8fb3',
    parent_id: existing.some((section) => section.id === parentId) ? parentId : '',
    order: Math.max(0, ...existing.filter((section) => section.parent_id === parentId).map((section) => section.order || 0)) + 10,
    icon: '', entrypoint: '',
  }
}

export function customSectionKey(id: string) {
  return `custom:${sanitizeSectionId(id)}`
}

export function buildOrganizationPrompt(
  projectName: string,
  catalog: DocumentCatalog,
  manifest: DocumentSectionManifest,
  operationId?: string,
  authorizationMode: DocumentAutomationMode = 'git_backed_full',
) {
  const customSections = manifest.sections.length
    ? manifest.sections.map((section) => `${customSectionKey(section.id)}=${section.label}`).join(', ')
    : '无'
  const ambiguous = catalog.documents.filter((document) => document.metadata.ambiguous).length
  const authorizationInstruction = authorizationMode === 'git_backed_full' || authorizationMode === 'trusted_reversible'
    ? `⑤ 使用 authorization_mode=${authorizationMode} 调用 project_docs_apply_suggestions；⑥ 若有 proposed file_operations，选择全部整理项调用 project_docs_apply_file_operations，不需要 reviewed/allow 标志；git_backed_full 必须确认响应同时包含整理前 git_baseline_commit 和整理后 git_result_commit；⑦ project_docs_get_status 核对最终状态。`
    : authorizationMode === 'review_all'
      ? '⑤ 调用 project_docs_get_status 后停在等待审核；没有用户确认，不得调用任何 apply 工具。'
      : '⑤ 调用 project_docs_get_status 后结束；当前是 suggestions_only，禁止调用任何 apply 工具。'
  return `<elon-project-docs-task version="1">\n请为项目“${projectName}”执行低 token 文档治理实验。\n\n` +
    `运行 ID：${operationId || '由 MCP 会话生成'}。权限模式：${authorizationMode}。目录 revision：${catalog.revision}；文档 ${catalog.documents.length} 份；歧义 ${ambiguous} 份；当前项目类型：${manifest.profile}；现有主题分区：${customSections}。\n` +
    '如果提供 project_docs_* MCP 工具，必须按以下顺序直接调用，不要用页面点击代替：' +
    '① project_docs_analyze 获取 classification_model_tokens=0 的紧凑目录和 document_health；大型仓库先按 federation 选择 scope_id；' +
    '② project_docs_get_issues 获取链接、孤立文档、owner、复查周期和实现引用的程序证据；' +
    '③ 仅对仍需语义判断的少量文档调用 project_docs_read；' +
    '④ project_docs_save_suggestions 携带当前 authorization_mode 保存 ready 建议；' +
    `④ 保存后按当前权限继续；${authorizationInstruction}` +
    '先根据 analyze 返回的 document_health 判断项目类型、质量问题、联邦节点和缺失基础文档；建议必须同时考虑面向人的主题知识树与面向 AI 的治理属性。' +
    '可提出 proposed_profile、层级 proposed_sections、proposed_home、document_metadata（类型、owner、复查日期、实现引用、关系、替代关系）；不要把 required/on-demand 等治理状态当成主题目录。' +
    '如发现命名含糊或路径放错，可在 file_operations 中提出结构化 rename/move；source_revision 必须使用 analyze 返回的 content_hash。' +
    `建议只能落到 ${ORGANIZATION_SUGGESTIONS_PATH}；不得删除、覆盖或改写 Markdown，也不得直接改分区配置。` +
    'git_backed_full 会自动完成整理前和整理后两次仅文档 Git 提交；任何模式都不得越界、操作非 Markdown、修改代码或自动 push。' +
    '虚拟分区不改变真实路径的 role、lifecycle、authority 或 default_retrieval；不能借虚拟 current 提升权威性。' +
    '只为确有改进价值的文档生成 assignments，层级主题最多 16 个，并在 architecture_findings 与 missing_document_types 中记录结构缺口；如实记录实际正文读取数和 token。' +
    '如果当前供应商确实没有 MCP，才使用同一顺序做本地元数据扫描并写建议 JSON；不要全文扫描 docs。'
}

export function serializeProjectDocumentJson(value: unknown) {
  return `${JSON.stringify(value, null, 2)}\n`
}

function sanitizeCustomSection(value: unknown): CustomDocumentSection | null {
  if (!value || typeof value !== 'object') return null
  const candidate = value as Partial<CustomDocumentSection>
  const id = sanitizeSectionId(candidate.id ?? '')
  const label = String(candidate.label ?? '').trim().slice(0, 40)
  if (!id || !label) return null
  const color = /^#[0-9a-f]{6}$/i.test(candidate.color ?? '') ? String(candidate.color) : '#7f8fb3'
  return {
    id,
    label,
    detail: String(candidate.detail ?? '').trim().slice(0, 120),
    color,
    parent_id: sanitizeSectionId(candidate.parent_id ?? ''),
    order: Math.min(9999, Math.max(0, Math.floor(Number(candidate.order) || 0))),
    icon: String(candidate.icon ?? '').trim().slice(0, 32),
    entrypoint: normalizedPath(candidate.entrypoint ?? ''),
  }
}

function sanitizeProfile(value: unknown) {
  const profile = String(value ?? '').trim().toLowerCase()
  return ['software-platform', 'software-api', 'product', 'research', 'operations', 'personal-knowledge'].includes(profile)
    ? profile
    : 'auto'
}

function sanitizeKnowledgeHome(value: unknown): DocumentKnowledgeHome {
  const candidate = value && typeof value === 'object' ? value as Partial<DocumentKnowledgeHome> : {}
  return {
    title: String(candidate.title ?? '').trim().slice(0, 80),
    summary: String(candidate.summary ?? '').trim().slice(0, 1000),
    entrypoint: normalizedPath(candidate.entrypoint ?? ''),
    start_here: stringArray(candidate.start_here, 12).map(normalizedPath).filter(Boolean),
  }
}

function sanitizeKnowledgeMetadata(value: unknown): DocumentKnowledgeMetadata | null {
  if (!value || typeof value !== 'object') return null
  const candidate = value as Partial<DocumentKnowledgeMetadata>
  return {
    id: String(candidate.id ?? '').trim().slice(0, 120),
    doc_type: String(candidate.doc_type ?? '').trim().slice(0, 64),
    audience: stringArray(candidate.audience, 12).map((entry) => entry.slice(0, 80)),
    owner: String(candidate.owner ?? '').trim().slice(0, 80),
    owners: stringArray(candidate.owners, 12).map((entry) => entry.slice(0, 80)),
    reviewed_at: /^\d{4}-\d{2}-\d{2}$/.test(String(candidate.reviewed_at ?? ''))
      ? String(candidate.reviewed_at) : '',
    review_interval_days: Math.min(3650, Math.max(1, Math.floor(Number(candidate.review_interval_days) || 180))),
    implementation_refs: stringArray(candidate.implementation_refs, 32).map((entry) => entry.slice(0, 500)),
    version: String(candidate.version ?? '').trim().slice(0, 40),
    related: stringArray(candidate.related, 24).map(normalizedPath).filter(Boolean),
    supersedes: stringArray(candidate.supersedes, 24).map(normalizedPath).filter(Boolean),
    order: Math.min(999999, Math.max(0, Math.floor(Number(candidate.order) || 0))),
    pinned: candidate.pinned === true,
  }
}

function sanitizeAuditLog(value: unknown): DocumentOrganizationAuditEntry[] {
  if (!Array.isArray(value)) return []
  return value.slice(-100).flatMap((entry) => {
    if (!entry || typeof entry !== 'object') return []
    const candidate = entry as Partial<DocumentOrganizationAuditEntry>
    const id = String(candidate.id ?? '').trim().slice(0, 80)
    const action = String(candidate.action ?? '').trim().slice(0, 64)
    if (!id || !action) return []
    return [{
      id,
      action,
      target: String(candidate.target ?? '').trim().slice(0, 500),
      summary: String(candidate.summary ?? '').trim().slice(0, 500),
      at: String(candidate.at ?? '').trim().slice(0, 40),
    }]
  })
}

function sanitizeKnowledgeMetadataMap(value: unknown) {
  const output: Record<string, DocumentKnowledgeMetadata> = {}
  if (!value || typeof value !== 'object') return output
  for (const [path, metadata] of Object.entries(value)) {
    const normalized = normalizedPath(path)
    const sanitized = sanitizeKnowledgeMetadata(metadata)
    if (normalized && sanitized) output[normalized] = sanitized
  }
  return output
}

function sanitizeSectionId(value: string) {
  return String(value).trim().toLowerCase()
    .replace(/[^a-z0-9\u4e00-\u9fff_-]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 48)
}

function normalizedPath(value: string) {
  return String(value).trim().replace(/\\/g, '/')
}

function stringArray(value: unknown, limit: number) {
  return Array.isArray(value)
    ? value.filter((entry): entry is string => typeof entry === 'string').slice(0, limit).map((entry) => entry.slice(0, 1000))
    : []
}

function safeNonNegativeNumber(value: unknown) {
  const number = Number(value)
  return Number.isFinite(number) && number >= 0 ? Math.floor(number) : 0
}

function uniqueSections(sections: CustomDocumentSection[]) {
  return [...new Map(sections.map((section) => [section.id, section])).values()]
}

function cloneEmptyManifest(): DocumentSectionManifest {
  return {
    version: 1,
    profile: 'auto',
    home: { title: '', summary: '', entrypoint: '', start_here: [] },
    sections: [],
    assignments: {},
    governance_overrides: {},
    document_metadata: {},
    audit_log: [],
  }
}
