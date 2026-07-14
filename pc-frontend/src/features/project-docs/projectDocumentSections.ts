import type { DocumentCatalog, ProjectDocumentEntry } from './projectDocumentModel'

export const SECTION_CONFIG_PATH = '.elon/document-sections.json'
export const ORGANIZATION_SUGGESTIONS_PATH = '.elon/document-organization-suggestions.json'

export interface CustomDocumentSection {
  id: string
  label: string
  detail: string
  color: string
}

export interface DocumentSectionManifest {
  version: 1
  sections: CustomDocumentSection[]
  assignments: Record<string, string>
}

export interface DocumentSection {
  key: string
  label: string
  detail: string
  color: string
  custom?: boolean
  virtual?: boolean
}

export interface SuggestedAssignment {
  path: string
  section_id: string
  reason: string
}

export interface DocumentOrganizationSuggestions {
  version: 1
  status: 'requested' | 'ready' | 'applied'
  summary: string
  proposed_sections: CustomDocumentSection[]
  assignments: SuggestedAssignment[]
  conflicts: string[]
  move_suggestions: string[]
  documents_read: number
  estimated_tokens_used: number
}

export const EMPTY_SECTION_MANIFEST: DocumentSectionManifest = {
  version: 1,
  sections: [],
  assignments: {},
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
  const custom = manifest.sections.map((section) => ({
    key: customSectionKey(section.id),
    label: section.label,
    detail: section.detail || '用户自定义项目分区',
    color: section.color || '#7b8aa5',
    custom: true,
  }))
  return [...SYSTEM_DOCUMENT_SECTIONS, ...custom, SUGGESTIONS_SECTION]
}

export function sectionForDocument(
  document: ProjectDocumentEntry,
  manifest: DocumentSectionManifest,
): string {
  const assigned = manifest.assignments[normalizedPath(document.path)]
  const validKeys = new Set(buildDocumentSections(manifest).filter((section) => !section.virtual).map((section) => section.key))
  if (assigned && validKeys.has(assigned)) return assigned

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
    const validSectionKeys = new Set([
      ...SYSTEM_DOCUMENT_SECTIONS.map((section) => section.key),
      ...sections.map((section) => customSectionKey(section.id)),
    ])
    const assignments: Record<string, string> = {}
    if (value.assignments && typeof value.assignments === 'object') {
      for (const [path, section] of Object.entries(value.assignments)) {
        if (typeof section === 'string' && validSectionKeys.has(section) && normalizedPath(path)) {
          assignments[normalizedPath(path)] = section
        }
      }
    }
    return { version: 1, sections, assignments }
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
      : []).slice(0, 8)
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
    return {
      version: 1,
      status,
      summary: String(value.summary ?? '').slice(0, 4000),
      proposed_sections: proposedSections,
      assignments,
      conflicts: stringArray(value.conflicts, 100),
      move_suggestions: stringArray(value.move_suggestions, 100),
      documents_read: safeNonNegativeNumber(value.documents_read),
      estimated_tokens_used: safeNonNegativeNumber(value.estimated_tokens_used),
    }
  } catch {
    return null
  }
}

export function createCustomSection(label: string, existing: CustomDocumentSection[]): CustomDocumentSection {
  const base = sanitizeSectionId(label) || `section-${Date.now()}`
  let id = base
  let suffix = 2
  while (existing.some((section) => section.id === id)) id = `${base}-${suffix++}`
  return { id, label: label.trim().slice(0, 40), detail: '用户自定义项目分区', color: '#7f8fb3' }
}

export function customSectionKey(id: string) {
  return `custom:${sanitizeSectionId(id)}`
}

export function buildOrganizationPrompt(
  projectName: string,
  catalog: DocumentCatalog,
  manifest: DocumentSectionManifest,
) {
  const lines = catalog.documents.map((document) => [
    document.path,
    document.metadata.role,
    document.metadata.lifecycle,
    document.metadata.authority,
    sectionForDocument(document, manifest),
    document.metadata.ambiguous ? 'ambiguous' : document.metadata.confidence,
  ].join(' | '))
  let compactCatalog = lines.join('\n')
  const maxChars = 12_000
  if (compactCatalog.length > maxChars) {
    compactCatalog = `${compactCatalog.slice(0, maxChars)}\n…目录因预算截断，请在结果中明确标记未覆盖范围。`
  }
  const customSections = manifest.sections.length
    ? manifest.sections.map((section) => `${customSectionKey(section.id)}=${section.label}`).join(', ')
    : '无'
  return `请为项目“${projectName}”执行低 token 文档治理实验。\n\n` +
    '程序已仅用路径、标题、元数据和哈希预分类，classification_model_tokens=0；不要全文扫描 Markdown。' +
    '优先检查 ambiguous；需要内容时先读标题和目录，仍无法判断才读单篇正文。' +
    `现有自定义分区：${customSections}。` +
    `你只能新建或覆盖 ${ORGANIZATION_SUGGESTIONS_PATH}，不得修改其他文件，不得移动或删除文档。` +
    '该文件必须是纯 JSON（不要 Markdown 代码栏），结构为：' +
    '{"version":1,"status":"ready","summary":"...","proposed_sections":[{"id":"slug","label":"名称","detail":"用途","color":"#RRGGBB"}],"assignments":[{"path":"docs/x.md","section_id":"current 或 custom:slug","reason":"..."}],"conflicts":["..."],"move_suggestions":["..."],"documents_read":0,"estimated_tokens_used":0}。' +
    '只为确有改进价值的文档生成 assignments；新分区最多 8 个。保持 AGENTS.md + .github/copilot-instructions.md + .github/instructions/ 的跨供应商结构。' +
    '如实填写实际读取文档数和估算 token。\n\n' +
    `紧凑目录（path | role | lifecycle | authority | section | confidence）：\n${compactCatalog}`
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
  return { id, label, detail: String(candidate.detail ?? '').trim().slice(0, 120), color }
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
  return { version: 1, sections: [], assignments: {} }
}
