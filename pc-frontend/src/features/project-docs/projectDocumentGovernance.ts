import type { ProjectDocumentEntry } from './projectDocumentModel'

export type RetrievalPolicy = 'required' | 'on_demand' | 'excluded'
export type GovernanceLifecycle = 'active' | 'accepted' | 'source_material' | 'draft' | 'deprecated' | 'superseded' | 'archived' | 'unclassified'
export type AuthorityLevel = 'binding' | 'authoritative' | 'guidance' | 'evidence' | 'proposal' | 'non_authoritative' | 'none' | 'unknown'

export interface DocumentGovernanceFacets {
  retrieval: RetrievalPolicy | ''
  lifecycle: GovernanceLifecycle | ''
  authority: AuthorityLevel | ''
  document_type: string
}

export interface DocumentRelation {
  relation: 'related' | 'supports' | 'depends_on' | 'implements' | 'evidence_for' | 'supersedes' | 'replaced_by' | 'see_also'
  target: string
}

export interface GovernanceFacetOption {
  key: string
  label: string
  detail: string
}

export const RETRIEVAL_OPTIONS: GovernanceFacetOption[] = [
  { key: 'required', label: '必须读取', detail: '每个 AI 供应商的共享入口或硬规则' },
  { key: 'on_demand', label: '按需读取', detail: '任务命中领域或主题时才加载' },
  { key: 'excluded', label: '默认排除', detail: '草稿、证据、历史或未分类材料' },
]

export const LIFECYCLE_OPTIONS: GovernanceFacetOption[] = [
  { key: 'active', label: '当前有效', detail: '仍适用于当前实现' },
  { key: 'accepted', label: '已接受', detail: '已接受决策或基线' },
  { key: 'source_material', label: '原始来源', detail: '聊天或导入材料，只用于追溯和再整理' },
  { key: 'draft', label: '草稿', detail: '尚未批准' },
  { key: 'deprecated', label: '已弃用', detail: '仍可追溯但不再推荐' },
  { key: 'superseded', label: '已替代', detail: '存在新的权威来源' },
  { key: 'archived', label: '历史归档', detail: '仅用于历史追溯' },
  { key: 'unclassified', label: '待确认', detail: '程序无法确认当前状态' },
]

export const AUTHORITY_OPTIONS: GovernanceFacetOption[] = [
  { key: 'binding', label: '约束性', detail: '共享规则和路由，必须遵守' },
  { key: 'authoritative', label: '权威事实', detail: '当前规范、架构、需求或运行基线' },
  { key: 'guidance', label: '指导性', detail: '指南和说明，不覆盖约束' },
  { key: 'evidence', label: '证据性', detail: '证明结果但不定义需求' },
  { key: 'proposal', label: '提案', detail: '未批准方案或需求' },
  { key: 'non_authoritative', label: '非权威', detail: '工具笔记、定制资产或历史材料' },
  { key: 'none', label: '无权威性', detail: '明确仅为原始来源，不能定义当前事实' },
  { key: 'unknown', label: '未知', detail: '需要用户或 AI 复核' },
]

export const DOCUMENT_TYPE_OPTIONS: GovernanceFacetOption[] = [
  { key: 'policy', label: '规则', detail: '仓库或领域约束' },
  { key: 'architecture', label: '架构', detail: '模块、边界与数据流' },
  { key: 'spec', label: '规范', detail: '可验证的技术定义' },
  { key: 'requirement', label: '需求', detail: '用户或业务目标' },
  { key: 'runbook', label: '操作手册', detail: '部署、运行和恢复步骤' },
  { key: 'guide', label: '指南', detail: '面向任务的说明' },
  { key: 'decision', label: '决策', detail: '已接受取舍及原因' },
  { key: 'report', label: '报告/证据', detail: '状态、测试或交付证据' },
  { key: 'discussion', label: '讨论/笔记', detail: '尚未成为项目事实' },
  { key: 'customization', label: 'AI 定制资产', detail: 'Agent、Prompt 或 Skill' },
]

const retrievalValues = new Set(RETRIEVAL_OPTIONS.map((option) => option.key))
const lifecycleValues = new Set(LIFECYCLE_OPTIONS.map((option) => option.key))
const authorityValues = new Set(AUTHORITY_OPTIONS.map((option) => option.key))
const relationValues = new Set(['related', 'supports', 'depends_on', 'implements', 'evidence_for', 'supersedes', 'replaced_by', 'see_also'])

export function sanitizeGovernanceFacetsMap(value: unknown) {
  const output: Record<string, DocumentGovernanceFacets> = {}
  if (!value || typeof value !== 'object') return output
  for (const [path, raw] of Object.entries(value)) {
    const normalized = normalizePath(path)
    if (!normalized || !raw || typeof raw !== 'object') continue
    const candidate = raw as Partial<DocumentGovernanceFacets>
    output[normalized] = {
      retrieval: retrievalValues.has(String(candidate.retrieval)) ? candidate.retrieval as RetrievalPolicy : '',
      lifecycle: lifecycleValues.has(String(candidate.lifecycle)) ? candidate.lifecycle as GovernanceLifecycle : '',
      authority: authorityValues.has(String(candidate.authority)) ? candidate.authority as AuthorityLevel : '',
      document_type: identifier(candidate.document_type, 64),
    }
  }
  return output
}

export function sanitizeSecondaryAssignments(value: unknown, knownTopics: Set<string>) {
  const output: Record<string, string[]> = {}
  if (!value || typeof value !== 'object') return output
  for (const [path, raw] of Object.entries(value)) {
    const normalized = normalizePath(path)
    if (!normalized || !Array.isArray(raw)) continue
    const topics = [...new Set(raw.filter((item): item is string => typeof item === 'string')
      .map((item) => item.trim()).filter((item) => knownTopics.has(item)))].slice(0, 12)
    if (topics.length) output[normalized] = topics
  }
  return output
}

export function sanitizeDocumentRelations(value: unknown): DocumentRelation[] {
  if (!Array.isArray(value)) return []
  const output: DocumentRelation[] = []
  for (const raw of value.slice(0, 48)) {
    if (!raw || typeof raw !== 'object') continue
    const candidate = raw as Partial<DocumentRelation>
    const relation = String(candidate.relation ?? '').trim().toLowerCase()
    const target = normalizePath(candidate.target ?? '')
    if (!relationValues.has(relation) || !target) continue
    if (!output.some((item) => item.relation === relation && item.target.toLowerCase() === target.toLowerCase())) {
      output.push({ relation: relation as DocumentRelation['relation'], target })
    }
  }
  return output
}

export function effectiveGovernanceFacets(
  document: ProjectDocumentEntry,
  configured?: Partial<DocumentGovernanceFacets>,
): DocumentGovernanceFacets {
  const base = inferredFacets(document)
  if (!configured) return base
  return {
    retrieval: clampRetrieval(base.retrieval, configured.retrieval),
    lifecycle: clampLifecycle(base.lifecycle, configured.lifecycle),
    authority: clampAuthority(base.authority, configured.authority),
    document_type: identifier(configured.document_type, 64) || base.document_type,
  }
}

export function governanceQuickView(facets: DocumentGovernanceFacets) {
  if (facets.retrieval === 'required') return 'required'
  if (facets.retrieval === 'on_demand' && ['instruction', 'project_guide', 'provider_adapter', 'guide'].includes(facets.document_type)) return 'on-demand'
  if (['agent_definition', 'prompt_template', 'skill', 'customization'].includes(facets.document_type)) return 'customizations'
  if (facets.document_type === 'decision') return 'decisions'
  if (['status', 'report'].includes(facets.document_type)) return 'evidence'
  if (facets.document_type === 'archive' || facets.lifecycle === 'archived') return 'archive'
  if (['discussion', 'note'].includes(facets.document_type)) return 'drafts'
  if (['draft', 'unclassified'].includes(facets.lifecycle)) return 'drafts'
  if (facets.authority === 'unknown') return 'unclassified'
  if (['active', 'accepted'].includes(facets.lifecycle)) return 'current'
  return 'unclassified'
}

export function facetLabel(options: GovernanceFacetOption[], key: string) {
  return options.find((option) => option.key === key)?.label ?? (key || '未设置')
}

function inferredFacets(document: ProjectDocumentEntry): DocumentGovernanceFacets {
  const { role, lifecycle, authority } = document.metadata
  const excluded = ['draft', 'deprecated', 'superseded', 'archived', 'unclassified'].includes(lifecycle)
    || ['archive', 'discussion', 'note', 'status', 'report'].includes(role)
  return {
    retrieval: ['policy', 'router'].includes(role) ? 'required' : excluded ? 'excluded' : 'on_demand',
    lifecycle: lifecycleValues.has(lifecycle) ? lifecycle as GovernanceLifecycle : 'unclassified',
    authority: authorityLevel(authority),
    document_type: identifier(role, 64),
  }
}

function authorityLevel(value: string): AuthorityLevel {
  if (['repository_policy', 'repository_routing', 'domain_policy'].includes(value)) return 'binding'
  if (['normative', 'approved', 'operational', 'decision_record'].includes(value)) return 'authoritative'
  if (value === 'evidence') return 'evidence'
  if (value === 'proposal') return 'proposal'
  if (value === 'none') return 'none'
  if (['historical', 'customization'].includes(value)) return 'non_authoritative'
  if (['provider_routing', 'project_guidance', 'informative'].includes(value)) return 'guidance'
  return 'unknown'
}

function clampRetrieval(base: RetrievalPolicy | '', requested?: string) {
  if (!requested || !retrievalValues.has(requested)) return base
  if (base === 'excluded' || requested === 'required' && base !== 'required') return base
  return requested as RetrievalPolicy
}

function clampLifecycle(base: GovernanceLifecycle | '', requested?: string) {
  if (!requested || !lifecycleValues.has(requested)) return base
  if (['draft', 'deprecated', 'superseded', 'archived', 'unclassified'].includes(base)
    && ['active', 'accepted'].includes(requested)) return base
  return requested as GovernanceLifecycle
}

function clampAuthority(base: AuthorityLevel | '', requested?: string) {
  if (!requested || !authorityValues.has(requested)) return base
  const rank = ['none', 'unknown', 'non_authoritative', 'proposal', 'evidence', 'guidance', 'authoritative', 'binding']
  return rank.indexOf(requested) > rank.indexOf(base) ? base : requested as AuthorityLevel
}

function identifier(value: unknown, limit: number) {
  return String(value ?? '').trim().toLowerCase().replace(/-/g, '_').replace(/[^a-z0-9_]/g, '').slice(0, limit)
}

function normalizePath(value: unknown) { return String(value ?? '').trim().replace(/\\/g, '/') }
