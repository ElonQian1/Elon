import type { DocumentCatalog, ProjectDocumentEntry } from './projectDocumentModel'
import {
  customSectionKey,
  SUGGESTIONS_SECTION,
  type DocumentSection,
  type DocumentSectionManifest,
} from './projectDocumentSections'

export type DocumentNavigationMode = 'knowledge' | 'governance'

export const KNOWLEDGE_HOME_SECTION = 'knowledge-home'
export const CAPABILITY_MAP_SECTION = 'capability-map'

export interface KnowledgeFoundation {
  id: string
  label: string
  covered: boolean
}

export interface KnowledgeArchitectureHealth {
  profile: string
  profileLabel: string
  profileSource: 'manifest' | 'metadata'
  score: number
  status: 'healthy' | 'needs_attention' | 'needs_structure'
  foundations: KnowledgeFoundation[]
  missingDocumentTypes: string[]
  topicAssigned: number
  topicUnassigned: number
  topicAutomatic: number
  ambiguous: number
  outdated: number
  duplicateTitles: number
  homeConfigured: boolean
  findings: string[]
}

interface TemplateSection {
  id: string
  label: string
  detail: string
  color: string
  parentId?: string
  aliases: string[]
}

interface ProfileTemplate {
  key: string
  label: string
  description: string
  foundations: string[]
  sections: TemplateSection[]
}

const sharedWorkspace: TemplateSection[] = [
  { id: 'decisions', label: '决策记录', detail: '已接受决定及其原因', color: '#c884dd', aliases: ['decision', 'adr', '决策'] },
  { id: 'workbench', label: '工作区', detail: '草稿、讨论、证据和待整理材料', color: '#c58c55', aliases: ['draft', 'discussion', 'report', 'inbox', '草稿', '讨论', '证据'] },
]

const profiles: ProfileTemplate[] = [
  {
    key: 'software-platform', label: '软件平台', description: '多端应用、服务端和节点共同组成的平台项目',
    foundations: ['overview', 'architecture', 'backend-api', 'clients', 'operations'],
    sections: [
      { id: 'overview', label: '项目总览', detail: '定位、范围、路线图和推荐阅读', color: '#9a74e8', aliases: ['readme', 'project', 'overview', 'landing', '总览', '路线图'] },
      { id: 'architecture', label: '平台架构', detail: '总体分层、模块边界和关键数据流', color: '#5f8fe3', aliases: ['architecture', '架构', 'data-flow'] },
      { id: 'backend-api', label: '后端与 API', detail: '服务端、接口、数据模型和权限', color: '#4fa9b8', aliases: ['server', 'backend', 'api', 'store', 'database', '接口', '后端'] },
      { id: 'clients', label: '客户端', detail: 'PC、Windows 节点和 Android 客户端', color: '#5fbc88', aliases: ['client', 'android', 'pc-', 'windows', 'node-agent', '客户端', '节点'] },
      { id: 'ai-context', label: 'AI 与上下文', detail: '代理、模型、RAG、Prompt 和上下文工程', color: '#b582dd', aliases: ['ai_', 'agent', 'prompt', 'skill', 'rag', 'context', '模型', 'ai-'] },
      { id: 'project-system', label: '用户与项目系统', detail: '用户、项目、Git 工作区和协作流程', color: '#d18b62', aliases: ['user-project', 'project-store', 'workspace', '用户', '项目系统'] },
      { id: 'operations', label: '发布与运维', detail: '构建、发布、部署、升级和故障处理', color: '#d0a34f', aliases: ['deploy', 'release', 'runbook', 'setup', 'upgrade', '发布', '运维', '故障'] },
      { id: 'security', label: '安全与权限', detail: '权限、密钥、隐私和安全边界', color: '#d06e75', aliases: ['security', 'permission', 'secret', 'vault', '安全', '权限', '密钥'] },
      ...sharedWorkspace,
    ],
  },
  {
    key: 'software-api', label: 'API / SDK', description: '以接口、SDK 或服务能力为核心的软件项目',
    foundations: ['overview', 'quickstart', 'architecture', 'api-reference', 'data-model', 'operations'],
    sections: [
      { id: 'overview', label: '项目总览', detail: '用途、范围和版本', color: '#9a74e8', aliases: ['readme', 'overview', '总览'] },
      { id: 'quickstart', label: '快速开始', detail: '安装、认证和第一个请求', color: '#5fbc88', aliases: ['quickstart', 'getting-started', 'setup', '快速开始', '安装'] },
      { id: 'concepts', label: '核心概念', detail: '领域术语、能力和约束', color: '#6e9bdc', aliases: ['concept', 'domain', '概念'] },
      { id: 'architecture', label: '系统架构', detail: '模块、边界和数据流', color: '#5f8fe3', aliases: ['architecture', '架构'] },
      { id: 'api-reference', label: 'API 参考', detail: '端点、参数、响应、错误码和示例', color: '#4fa9b8', aliases: ['api', 'endpoint', 'openapi', '接口', '错误码'] },
      { id: 'data-model', label: '数据模型', detail: '实体、字段、Schema 和关系', color: '#5aaf9a', aliases: ['data-model', 'schema', 'database', '数据模型'] },
      { id: 'guides', label: '开发指南', detail: '面向任务的使用方法', color: '#7ca86c', aliases: ['guide', 'how-to', '指南'] },
      { id: 'operations', label: '部署运维', detail: '部署、监控和故障恢复', color: '#d0a34f', aliases: ['deploy', 'runbook', 'monitor', '运维'] },
      ...sharedWorkspace,
    ],
  },
  {
    key: 'product', label: '产品与业务', description: '以用户、需求、流程和业务目标为核心的项目',
    foundations: ['overview', 'users', 'requirements', 'roadmap', 'metrics'],
    sections: [
      { id: 'overview', label: '产品总览', detail: '愿景、目标和范围', color: '#9a74e8', aliases: ['overview', 'vision', '总览', '愿景'] },
      { id: 'users', label: '用户与场景', detail: '角色、旅程和使用场景', color: '#5f8fe3', aliases: ['user', 'persona', 'journey', '用户', '场景'] },
      { id: 'requirements', label: '需求与验收', detail: '已批准需求和验收标准', color: '#5fbc88', aliases: ['requirement', 'spec', '需求', '验收'] },
      { id: 'processes', label: '业务流程', detail: '规则、流程和状态', color: '#4fa9b8', aliases: ['process', 'workflow', '流程'] },
      { id: 'roadmap', label: '路线图', detail: '阶段、优先级和交付计划', color: '#d0a34f', aliases: ['roadmap', 'plan', '路线图', '计划'] },
      { id: 'metrics', label: '指标与复盘', detail: '指标、反馈和结果', color: '#d18b62', aliases: ['metric', 'analytics', '指标', '复盘'] },
      ...sharedWorkspace,
    ],
  },
  {
    key: 'research', label: '研究项目', description: '围绕问题、文献、方法、实验和结论组织',
    foundations: ['overview', 'literature', 'methods', 'experiments', 'results'],
    sections: [
      { id: 'overview', label: '研究总览', detail: '问题、范围和研究目标', color: '#9a74e8', aliases: ['overview', 'question', '总览', '问题'] },
      { id: 'literature', label: '文献与来源', detail: '相关工作、引用和资料', color: '#5f8fe3', aliases: ['literature', 'reference', '文献', '来源'] },
      { id: 'methods', label: '方法', detail: '研究设计、方法和假设', color: '#4fa9b8', aliases: ['method', 'hypothesis', '方法', '假设'] },
      { id: 'experiments', label: '实验与数据', detail: '实验记录、数据集和分析', color: '#5fbc88', aliases: ['experiment', 'dataset', '实验', '数据'] },
      { id: 'results', label: '结果与结论', detail: '发现、限制和结论', color: '#d0a34f', aliases: ['result', 'conclusion', '结果', '结论'] },
      ...sharedWorkspace,
    ],
  },
  {
    key: 'operations', label: '运维项目', description: '围绕服务、SOP、监控、事故和恢复组织',
    foundations: ['overview', 'runbooks', 'monitoring', 'incidents', 'security'],
    sections: [
      { id: 'overview', label: '服务总览', detail: '服务目录、依赖和负责人', color: '#9a74e8', aliases: ['overview', 'service', '总览', '服务'] },
      { id: 'runbooks', label: '运行手册', detail: '部署、维护和标准操作', color: '#5fbc88', aliases: ['runbook', 'sop', 'deploy', '手册', '部署'] },
      { id: 'monitoring', label: '监控与告警', detail: '指标、仪表盘和告警规则', color: '#4fa9b8', aliases: ['monitor', 'alert', '监控', '告警'] },
      { id: 'incidents', label: '事故与恢复', detail: '事故记录、恢复和复盘', color: '#d06e75', aliases: ['incident', 'recovery', '故障', '恢复', '事故'] },
      { id: 'security', label: '安全与权限', detail: '访问、密钥和审计', color: '#d0a34f', aliases: ['security', 'permission', '安全', '权限'] },
      ...sharedWorkspace,
    ],
  },
  {
    key: 'personal-knowledge', label: '个人知识库', description: '围绕主题、方法、来源和笔记组织',
    foundations: ['overview', 'topics', 'guides', 'sources'],
    sections: [
      { id: 'overview', label: '知识库总览', detail: '范围和推荐入口', color: '#9a74e8', aliases: ['overview', 'readme', '总览'] },
      { id: 'topics', label: '核心主题', detail: '长期维护的主题知识', color: '#5f8fe3', aliases: ['topic', '主题'] },
      { id: 'guides', label: '方法与指南', detail: '可复用的方法和步骤', color: '#5fbc88', aliases: ['guide', 'method', '方法', '指南'] },
      { id: 'sources', label: '来源与参考', detail: '引用、摘录和外部资料', color: '#4fa9b8', aliases: ['source', 'reference', '来源', '参考'] },
      ...sharedWorkspace,
    ],
  },
]

export const KNOWLEDGE_PROFILE_OPTIONS = profiles.map(({ key, label, description }) => ({
  key,
  label,
  description,
}))

export function resolveKnowledgeProfile(catalog: DocumentCatalog | null, manifest: DocumentSectionManifest) {
  const configured = profiles.find((profile) => profile.key === manifest.profile)
  if (configured) return { template: configured, source: 'manifest' as const }
  const searchable = (catalog?.documents ?? []).flatMap((document) => [
    document.path, document.title, ...document.metadata.headings,
  ]).join(' ').toLowerCase()
  const score = (terms: string[]) => terms.filter((term) => searchable.includes(term)).length
  const inferred = score(['android', 'pc-frontend', 'node-agent', 'server/src']) >= 3 ? 'software-platform'
    : score(['api', 'openapi', 'endpoint', 'sdk', '接口']) >= 2 ? 'software-api'
      : score(['research', 'experiment', 'dataset', '文献', '实验']) >= 2 ? 'research'
        : score(['runbook', 'incident', 'monitor', '运维', '故障']) >= 2 ? 'operations'
          : score(['roadmap', 'requirement', 'persona', '需求', '用户']) >= 2 ? 'product'
            : 'personal-knowledge'
  return { template: profiles.find((profile) => profile.key === inferred) ?? profiles[5], source: 'metadata' as const }
}

export function buildKnowledgeSections(catalog: DocumentCatalog | null, manifest: DocumentSectionManifest) {
  const { template } = resolveKnowledgeProfile(catalog, manifest)
  const customIds = new Set(manifest.sections.map((section) => section.id))
  const customSections: DocumentSection[] = [...manifest.sections]
    .sort((left, right) => left.order - right.order || left.label.localeCompare(right.label, 'zh-CN'))
    .map((section) => ({
      key: customSectionKey(section.id), label: section.label, detail: section.detail,
      color: section.color, custom: true, parentId: section.parent_id ? customSectionKey(section.parent_id) : undefined,
      order: section.order, icon: section.icon, entrypoint: section.entrypoint,
    }))
  const templateSections: DocumentSection[] = template.sections
    .filter((section) => !customIds.has(section.id))
    .map((section, index) => ({
      key: `topic:${section.id}`, label: section.label, detail: section.detail, color: section.color,
      template: true, virtual: true, parentId: section.parentId ? `topic:${section.parentId}` : undefined, order: index * 10,
    }))
  const topics = [...templateSections, ...customSections]
  return [
    { key: KNOWLEDGE_HOME_SECTION, label: '知识首页', detail: '项目地图、推荐阅读与完整度', color: '#9b73ed', virtual: true, order: -100 },
    { key: CAPABILITY_MAP_SECTION, label: '功能地图', detail: '功能、子能力与对应文档覆盖', color: '#58a8df', virtual: true, order: -90 },
    ...flattenSectionTree(topics),
    SUGGESTIONS_SECTION,
  ]
}

export function topicSectionForDocument(
  document: ProjectDocumentEntry,
  catalog: DocumentCatalog | null,
  manifest: DocumentSectionManifest,
) {
  const assigned = manifest.assignments[normalizePath(document.path)]
  if (assigned?.startsWith('custom:') && manifest.sections.some((section) => customSectionKey(section.id) === assigned)) return assigned
  const entrypoint = manifest.sections.find((section) => normalizePath(section.entrypoint) === normalizePath(document.path))
  if (entrypoint) return customSectionKey(entrypoint.id)
  const { template } = resolveKnowledgeProfile(catalog, manifest)
  const inferred = inferTopic(document, template)
  const matchingCustom = manifest.sections.find((section) => section.id === inferred.id)
    ?? manifest.sections.find((section) => sectionMatches(section.id, section.label, inferred.aliases))
  return matchingCustom ? customSectionKey(matchingCustom.id) : `topic:${inferred.id}`
}

export function analyzeKnowledgeArchitecture(catalog: DocumentCatalog | null, manifest: DocumentSectionManifest): KnowledgeArchitectureHealth {
  const documents = catalog?.documents ?? []
  const { template, source } = resolveKnowledgeProfile(catalog, manifest)
  const counts = new Map<string, number>()
  documents.forEach((document) => {
    const section = topicSectionForDocument(document, catalog, manifest)
    counts.set(section, (counts.get(section) ?? 0) + 1)
  })
  const customAssigned = documents.filter((document) => manifest.assignments[normalizePath(document.path)]?.startsWith('custom:')).length
  const topicAutomatic = Math.max(0, documents.length - customAssigned)
  const foundations = template.foundations.map((id) => ({
    id,
    label: template.sections.find((section) => section.id === id)?.label ?? id,
    covered: foundationCovered(id, documents, manifest, template),
  }))
  const missingDocumentTypes = foundations.filter((item) => !item.covered).map((item) => item.id)
  const ambiguous = documents.filter((document) => document.metadata.ambiguous).length
  const outdated = documents.filter((document) => ['deprecated', 'superseded'].includes(document.metadata.lifecycle)).length
  const duplicateTitles = duplicateTitleCount(documents)
  const homeConfigured = !!manifest.home.title && !!manifest.home.summary && !!(manifest.home.entrypoint || manifest.home.start_here.length)
  const score = Math.min(100,
    (source === 'manifest' ? 15 : 8)
    + (homeConfigured ? 15 : 0)
    + Math.round(30 * (foundations.filter((item) => item.covered).length / Math.max(1, foundations.length)))
    + 30
    + Math.max(0, 10 - Math.round(10 * ambiguous / Math.max(1, documents.length))))
  const findings: string[] = []
  if (source === 'metadata') findings.push(`项目类型暂由路径和标题推断为“${template.label}”，建议固化模板。`)
  if (!homeConfigured) findings.push('缺少知识首页摘要和推荐阅读入口。')
  if (missingDocumentTypes.length) findings.push(`缺少 ${missingDocumentTypes.length} 类基础文档或明确入口。`)
  if (ambiguous) findings.push(`${ambiguous} 份文档仍无法从路径判断权威性。`)
  if (duplicateTitles) findings.push(`${duplicateTitles} 组文档标题重复，需要确认唯一入口。`)
  return {
    profile: template.key, profileLabel: template.label, profileSource: source, score,
    status: score >= 85 ? 'healthy' : score >= 60 ? 'needs_attention' : 'needs_structure',
    foundations, missingDocumentTypes, topicAssigned: documents.length,
    topicUnassigned: 0, topicAutomatic, ambiguous, outdated,
    duplicateTitles, homeConfigured, findings,
  }
}

export function knowledgeSectionCounts(
  catalog: DocumentCatalog | null,
  manifest: DocumentSectionManifest,
  sections: DocumentSection[],
) {
  const counts = Object.fromEntries(sections.map((section) => [section.key, 0])) as Record<string, number>
  for (const document of catalog?.documents ?? []) {
    const section = topicSectionForDocument(document, catalog, manifest)
    counts[section] = (counts[section] ?? 0) + 1
  }
  counts[CAPABILITY_MAP_SECTION] = sections.filter((section) =>
    ![KNOWLEDGE_HOME_SECTION, CAPABILITY_MAP_SECTION, SUGGESTIONS_SECTION.key].includes(section.key),
  ).length
  return counts
}

export function recommendedStartDocuments(catalog: DocumentCatalog | null, manifest: DocumentSectionManifest) {
  const documents = catalog?.documents ?? []
  const byPath = new Map(documents.map((document) => [normalizePath(document.path), document]))
  const configured = [manifest.home.entrypoint, ...manifest.home.start_here]
    .map(normalizePath).filter(Boolean).map((path) => byPath.get(path)).filter((item): item is ProjectDocumentEntry => !!item)
  if (configured.length) return [...new Map(configured.map((document) => [document.path, document])).values()].slice(0, 8)
  return documents.filter((document) => /(^|\/)(readme|ai_project|ai_architecture|system-architecture)/i.test(document.path)).slice(0, 6)
}

function inferTopic(document: ProjectDocumentEntry, template: ProfileTemplate) {
  const searchable = `${document.path} ${document.title} ${document.metadata.headings.join(' ')}`.toLowerCase()
  const scored = template.sections.map((section) => ({
    section,
    score: section.aliases.reduce((total, alias) => total + (searchable.includes(alias.toLowerCase()) ? 2 : 0), 0)
      + roleTopicScore(document.metadata.role, section.id),
  })).sort((left, right) => right.score - left.score)
  return scored[0]?.score > 0 ? scored[0].section : template.sections.find((section) => section.id === 'workbench') ?? template.sections[0]
}

function roleTopicScore(role: string, sectionId: string) {
  if (role === 'architecture' && sectionId === 'architecture') return 5
  if (role === 'requirement' && sectionId === 'requirements') return 5
  if (role === 'runbook' && ['operations', 'runbooks'].includes(sectionId)) return 5
  if (role === 'decision' && sectionId === 'decisions') return 5
  if (['discussion', 'note', 'status', 'report'].includes(role) && sectionId === 'workbench') return 3
  if (['policy', 'router', 'agent_definition', 'prompt_template', 'skill'].includes(role) && sectionId === 'ai-context') return 4
  return 0
}

function foundationCovered(id: string, documents: ProjectDocumentEntry[], manifest: DocumentSectionManifest, template: ProfileTemplate) {
  if (Object.values(manifest.document_metadata).some((metadata) => metadata.doc_type === id)) return true
  if (manifest.sections.some((section) => section.id === id && !!section.entrypoint)) return true
  const definition = template.sections.find((section) => section.id === id)
  return !!definition && documents.some((document) => {
    const searchable = `${document.path} ${document.title}`.toLowerCase()
    return definition.aliases.some((alias) => searchable.includes(alias.toLowerCase())) || roleTopicScore(document.metadata.role, id) >= 5
  })
}

function sectionMatches(id: string, label: string, aliases: string[]) {
  const searchable = `${id} ${label}`.toLowerCase()
  return aliases.some((alias) => searchable.includes(alias.toLowerCase()))
}

function flattenSectionTree(sections: DocumentSection[]) {
  const byParent = new Map<string, DocumentSection[]>()
  for (const section of sections) {
    const parent = section.parentId && sections.some((candidate) => candidate.key === section.parentId) ? section.parentId : ''
    byParent.set(parent, [...(byParent.get(parent) ?? []), section])
  }
  const output: DocumentSection[] = []
  const visit = (parent: string, depth: number) => {
    const children = (byParent.get(parent) ?? []).sort((left, right) => (left.order ?? 0) - (right.order ?? 0) || left.label.localeCompare(right.label, 'zh-CN'))
    children.forEach((section) => {
      output.push({ ...section, depth })
      visit(section.key, depth + 1)
    })
  }
  visit('', 0)
  return output
}

function duplicateTitleCount(documents: ProjectDocumentEntry[]) {
  const counts = new Map<string, number>()
  documents.forEach((document) => {
    const title = document.title.toLowerCase().replace(/[\s\-_（）()]/g, '')
    if (title) counts.set(title, (counts.get(title) ?? 0) + 1)
  })
  return [...counts.values()].filter((count) => count > 1).length
}

function normalizePath(value: string) {
  return String(value ?? '').trim().replace(/\\/g, '/')
}
