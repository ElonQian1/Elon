import {
  buildKnowledgeSections,
  CAPABILITY_MAP_SECTION,
  KNOWLEDGE_HOME_SECTION,
  topicSectionForDocument,
} from './projectDocumentArchitecture'
import type { DocumentCatalog, ProjectDocumentEntry } from './projectDocumentModel'
import { SUGGESTIONS_SECTION, type DocumentSection, type DocumentSectionManifest } from './projectDocumentSections'

export type CapabilityStatus = 'healthy' | 'partial' | 'gap'
export type CapabilityCoverageKey = 'overview' | 'requirements' | 'architecture' | 'reference' | 'operations' | 'evidence'

export interface CapabilityCoverageItem {
  key: CapabilityCoverageKey
  label: string
  covered: boolean
  count: number
}

export interface ProjectCapabilityNode {
  id: string
  label: string
  detail: string
  color: string
  parentId: string
  depth: number
  childCount: number
  documentCount: number
  documents: ProjectDocumentEntry[]
  entrypoint: string
  entrypointSource: 'configured' | 'inferred' | 'missing'
  coverage: CapabilityCoverageItem[]
  missingCoverage: string[]
  status: CapabilityStatus
  isRoot: boolean
  isCustom: boolean
}

export interface ProjectCapabilityEdge {
  id: string
  source: string
  target: string
}

export interface ProjectCapabilityGraph {
  rootId: string
  nodes: ProjectCapabilityNode[]
  edges: ProjectCapabilityEdge[]
  stats: Record<CapabilityStatus, number>
}

export interface CapabilityGraphSelection {
  nodes: ProjectCapabilityNode[]
  edges: ProjectCapabilityEdge[]
}

const ROOT_ID = 'capability:root'
const COVERAGE_DEFINITIONS: Array<Omit<CapabilityCoverageItem, 'covered' | 'count'>> = [
  { key: 'overview', label: '入口' },
  { key: 'requirements', label: '需求' },
  { key: 'architecture', label: '设计' },
  { key: 'reference', label: '参考' },
  { key: 'operations', label: '操作' },
  { key: 'evidence', label: '证据' },
]

export function buildCapabilityGraph(
  projectName: string,
  catalog: DocumentCatalog | null,
  manifest: DocumentSectionManifest,
): ProjectCapabilityGraph {
  const documents = catalog?.documents ?? []
  const sections = buildKnowledgeSections(catalog, manifest).filter(isCapabilitySection)
  const sectionIds = new Set(sections.map((section) => section.key))
  const documentsBySection = new Map<string, ProjectDocumentEntry[]>()
  for (const document of documents) {
    const sectionId = topicSectionForDocument(document, catalog, manifest)
    documentsBySection.set(sectionId, [...(documentsBySection.get(sectionId) ?? []), document])
  }

  const nodes = sections.map((section) => buildSectionNode(
    section,
    documentsBySection.get(section.key) ?? [],
    section.parentId && sectionIds.has(section.parentId) ? section.parentId : ROOT_ID,
  ))
  const childCounts = new Map<string, number>()
  nodes.forEach((node) => childCounts.set(node.parentId, (childCounts.get(node.parentId) ?? 0) + 1))
  const depths = resolveDepths(nodes)
  const hydrated = nodes.map((node) => ({
    ...node,
    depth: depths.get(node.id) ?? 1,
    childCount: childCounts.get(node.id) ?? 0,
  }))
  const rootDocuments = prioritizeDocuments(documents)
  const rootConfigured = rootDocuments.find((document) => normalizePath(document.path) === normalizePath(manifest.home.entrypoint))
  const rootEntrypoint = rootConfigured?.path ?? rootDocuments[0]?.path ?? ''
  const rootCoverage = buildCoverage(documents, !!rootEntrypoint)
  const root: ProjectCapabilityNode = {
    id: ROOT_ID,
    label: projectName,
    detail: '项目功能与知识覆盖总览',
    color: '#9b73ed',
    parentId: '',
    depth: 0,
    childCount: childCounts.get(ROOT_ID) ?? 0,
    documentCount: documents.length,
    documents: rootDocuments,
    entrypoint: rootEntrypoint,
    entrypointSource: rootConfigured ? 'configured' : rootEntrypoint ? 'inferred' : 'missing',
    coverage: rootCoverage,
    missingCoverage: rootCoverage.filter((item) => !item.covered).map((item) => item.label),
    status: documents.length && rootCoverage.filter((item) => item.covered).length >= 4 ? 'healthy' : documents.length ? 'partial' : 'gap',
    isRoot: true,
    isCustom: false,
  }
  const allNodes = [root, ...hydrated]
  const edges = hydrated.map((node) => ({ id: `${node.parentId}->${node.id}`, source: node.parentId, target: node.id }))
  return {
    rootId: ROOT_ID,
    nodes: allNodes,
    edges,
    stats: hydrated.reduce<Record<CapabilityStatus, number>>((stats, node) => {
      stats[node.status] += 1
      return stats
    }, { healthy: 0, partial: 0, gap: 0 }),
  }
}

export function selectCapabilityGraph(
  graph: ProjectCapabilityGraph,
  collapsedIds: ReadonlySet<string>,
  query = '',
  status: CapabilityStatus | 'all' = 'all',
): CapabilityGraphSelection {
  const normalizedQuery = query.trim().toLowerCase()
  const filtered = !!normalizedQuery || status !== 'all'
  const byId = new Map(graph.nodes.map((node) => [node.id, node]))
  const visibleIds = new Set<string>([graph.rootId])

  if (filtered) {
    graph.nodes.filter((node) => !node.isRoot
      && (status === 'all' || node.status === status)
      && (!normalizedQuery || nodeSearchText(node).includes(normalizedQuery)))
      .forEach((node) => addWithAncestors(node.id, byId, visibleIds))
  } else {
    const children = childrenByParent(graph.nodes)
    const visit = (nodeId: string) => {
      visibleIds.add(nodeId)
      if (collapsedIds.has(nodeId)) return
      ;(children.get(nodeId) ?? []).forEach((child) => visit(child.id))
    }
    visit(graph.rootId)
  }

  return {
    nodes: graph.nodes.filter((node) => visibleIds.has(node.id)),
    edges: graph.edges.filter((edge) => visibleIds.has(edge.source) && visibleIds.has(edge.target)),
  }
}

export function layoutCapabilityGraph(nodes: ProjectCapabilityNode[]) {
  const nodeIds = new Set(nodes.map((node) => node.id))
  const children = childrenByParent(nodes.filter((node) => node.isRoot || nodeIds.has(node.parentId)))
  const positions = new Map<string, { x: number; y: number }>()
  let leafIndex = 0
  const visit = (node: ProjectCapabilityNode): number => {
    const descendants = children.get(node.id) ?? []
    const y = descendants.length
      ? descendants.map(visit).reduce((total, value) => total + value, 0) / descendants.length
      : leafIndex++ * 164
    positions.set(node.id, { x: node.depth * 318, y })
    return y
  }
  const root = nodes.find((node) => node.isRoot)
  if (root) visit(root)
  const height = Math.max(0, (leafIndex - 1) * 164)
  positions.forEach((position, id) => positions.set(id, { ...position, y: position.y - height / 2 }))
  return positions
}

export function capabilityStatusLabel(status: CapabilityStatus) {
  return status === 'healthy' ? '覆盖良好' : status === 'partial' ? '待补齐' : '文档空白'
}

function buildSectionNode(section: DocumentSection, documents: ProjectDocumentEntry[], parentId: string): ProjectCapabilityNode {
  const prioritized = prioritizeDocuments(documents)
  const configured = prioritized.find((document) => normalizePath(document.path) === normalizePath(section.entrypoint ?? ''))
  const entrypoint = configured?.path ?? prioritized[0]?.path ?? ''
  const coverage = buildCoverage(documents, !!entrypoint)
  const coveredCount = coverage.filter((item) => item.covered).length
  return {
    id: section.key,
    label: section.label,
    detail: section.detail,
    color: section.color,
    parentId,
    depth: 1,
    childCount: 0,
    documentCount: documents.length,
    documents: prioritized,
    entrypoint,
    entrypointSource: configured ? 'configured' : entrypoint ? 'inferred' : 'missing',
    coverage,
    missingCoverage: coverage.filter((item) => !item.covered).map((item) => item.label),
    status: !documents.length ? 'gap' : configured || coveredCount >= 3 ? 'healthy' : 'partial',
    isRoot: false,
    isCustom: !!section.custom,
  }
}

function buildCoverage(documents: ProjectDocumentEntry[], hasEntrypoint = false) {
  const counts = Object.fromEntries(COVERAGE_DEFINITIONS.map((item) => [item.key, 0])) as Record<CapabilityCoverageKey, number>
  documents.forEach((document) => coverageKeysForDocument(document).forEach((key) => { counts[key] += 1 }))
  if (hasEntrypoint && !counts.overview) counts.overview = 1
  return COVERAGE_DEFINITIONS.map((item) => ({ ...item, count: counts[item.key], covered: counts[item.key] > 0 }))
}

function coverageKeysForDocument(document: ProjectDocumentEntry): CapabilityCoverageKey[] {
  const role = document.metadata.role
  const searchable = `${document.path} ${document.title} ${document.metadata.headings.join(' ')}`.toLowerCase()
  const keys = new Set<CapabilityCoverageKey>()
  if (['policy', 'router', 'project_guide'].includes(role) || /readme|overview|总览|入口/.test(searchable)) keys.add('overview')
  if (role === 'requirement' || /requirement|需求|验收/.test(searchable)) keys.add('requirements')
  if (role === 'architecture' || /architecture|设计|架构|data-flow/.test(searchable)) keys.add('architecture')
  if (['spec', 'instruction', 'provider_adapter'].includes(role) || /api|reference|schema|接口|规范/.test(searchable)) keys.add('reference')
  if (['runbook', 'guide'].includes(role) || /runbook|deploy|release|setup|操作|发布|运维/.test(searchable)) keys.add('operations')
  if (['status', 'report', 'decision'].includes(role) || /test|report|evidence|status|测试|证据|决策/.test(searchable)) keys.add('evidence')
  return [...keys]
}

function prioritizeDocuments(documents: ProjectDocumentEntry[]) {
  return [...documents].sort((left, right) => documentPriority(right) - documentPriority(left)
    || left.title.localeCompare(right.title, 'zh-CN'))
}

function documentPriority(document: ProjectDocumentEntry) {
  const rolePriority: Record<string, number> = {
    router: 9, project_guide: 8, requirement: 7, architecture: 7, spec: 6, runbook: 5, guide: 4,
  }
  return (document.metadata.default_retrieval ? 20 : 0)
    + (document.metadata.lifecycle === 'active' ? 4 : 0)
    + (document.metadata.ambiguous ? -8 : 0)
    + (rolePriority[document.metadata.role] ?? 0)
}

function resolveDepths(nodes: ProjectCapabilityNode[]) {
  const byId = new Map(nodes.map((node) => [node.id, node]))
  const depths = new Map<string, number>()
  const depthOf = (node: ProjectCapabilityNode, seen = new Set<string>()): number => {
    if (depths.has(node.id)) return depths.get(node.id)!
    if (node.parentId === ROOT_ID || seen.has(node.id)) return 1
    const parent = byId.get(node.parentId)
    if (!parent) return 1
    const depth = Math.min(4, depthOf(parent, new Set([...seen, node.id])) + 1)
    depths.set(node.id, depth)
    return depth
  }
  nodes.forEach((node) => depths.set(node.id, depthOf(node)))
  return depths
}

function childrenByParent(nodes: ProjectCapabilityNode[]) {
  const children = new Map<string, ProjectCapabilityNode[]>()
  nodes.filter((node) => !node.isRoot).forEach((node) => {
    children.set(node.parentId, [...(children.get(node.parentId) ?? []), node])
  })
  return children
}

function addWithAncestors(id: string, byId: Map<string, ProjectCapabilityNode>, output: Set<string>) {
  let current = byId.get(id)
  while (current) {
    output.add(current.id)
    current = current.parentId ? byId.get(current.parentId) : undefined
  }
}

function nodeSearchText(node: ProjectCapabilityNode) {
  return `${node.label} ${node.detail} ${node.documents.map((document) => `${document.title} ${document.path}`).join(' ')}`.toLowerCase()
}

function isCapabilitySection(section: DocumentSection) {
  return ![KNOWLEDGE_HOME_SECTION, CAPABILITY_MAP_SECTION, SUGGESTIONS_SECTION.key].includes(section.key)
}

function normalizePath(value: string) {
  return String(value ?? '').trim().replace(/\\/g, '/')
}
