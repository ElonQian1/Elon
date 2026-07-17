import type { DocumentCatalog, ProjectDocumentEntry } from './projectDocumentModel'
import type {
  ImplementationStatus,
  ProjectKnowledgeMapFinding,
  ProjectKnowledgeMapView,
} from './projectDocumentKnowledgeGraphModel'

export type CapabilityStatus = 'healthy' | 'partial' | 'gap'

export interface ProjectCapabilityNode {
  id: string
  view: ProjectKnowledgeMapView
  kind: string
  label: string
  detail: string
  color: string
  parentId: string
  sectionId: string
  depth: number
  childCount: number
  documentCount: number
  documents: ProjectDocumentEntry[]
  documentPaths: string[]
  entrypoint: string
  entrypointSource: 'configured' | 'inferred' | 'missing'
  coverage: Array<{ key: string; label: string; covered: boolean; count: number }>
  missingCoverage: string[]
  status: CapabilityStatus
  implementationStatus: ImplementationStatus
  implementationRefs: Array<{ reference: string; verification: 'exists' | 'missing' | 'declared' }>
  source: 'manifest' | 'profile_template'
  isRoot: boolean
}

export interface ProjectCapabilityEdge {
  id: string
  source: string
  target: string
  relation: string
  label: string
  configured: boolean
}

export interface ProjectCapabilityGraph {
  view: ProjectKnowledgeMapView
  title: string
  source: 'manifest' | 'profile_template' | 'unavailable'
  rootId: string
  nodes: ProjectCapabilityNode[]
  edges: ProjectCapabilityEdge[]
  stats: Record<CapabilityStatus, number>
  structuralScore: number
  diagnosticStatus: 'healthy' | 'review' | 'needs_structure' | 'unavailable'
  findings: ProjectKnowledgeMapFinding[]
}

export interface CapabilityGraphSelection {
  nodes: ProjectCapabilityNode[]
  edges: ProjectCapabilityEdge[]
}

export function buildCapabilityGraph(
  projectName: string,
  catalog: DocumentCatalog | null,
  view: ProjectKnowledgeMapView,
): ProjectCapabilityGraph {
  const map = catalog?.analysis?.knowledge_maps?.[view]
  if (!map) return unavailableGraph(projectName, view)
  const documents = new Map((catalog?.documents ?? []).map((document) => [normalizePath(document.path), document]))
  const nodes = map.nodes.map((node) => ({
    id: node.id,
    view: node.view,
    kind: node.kind,
    label: node.id === map.root_id ? projectName : node.label,
    detail: node.detail,
    color: node.color,
    parentId: node.parent_id,
    sectionId: node.section_id,
    depth: node.depth,
    childCount: node.child_count,
    documentCount: node.document_count,
    documents: node.document_paths.map((path) => documents.get(normalizePath(path))).filter((item): item is ProjectDocumentEntry => !!item),
    documentPaths: node.document_paths,
    entrypoint: node.entrypoint,
    entrypointSource: node.entrypoint_source,
    coverage: node.coverage,
    missingCoverage: node.missing_coverage,
    status: documentStatus(node.documentation_status),
    implementationStatus: node.implementation_status,
    implementationRefs: node.implementation_refs,
    source: node.source,
    isRoot: node.id === map.root_id,
  }))
  return {
    view,
    title: map.title,
    source: map.source,
    rootId: map.root_id,
    nodes,
    edges: map.edges,
    stats: { healthy: map.stats.documented, partial: map.stats.partial, gap: map.stats.undocumented },
    structuralScore: map.diagnostics.structural_score,
    diagnosticStatus: map.diagnostics.status,
    findings: map.diagnostics.findings,
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
  const children = childrenByParent(nodes)
  const positions = new Map<string, { x: number; y: number }>()
  let leafIndex = 0
  const visit = (node: ProjectCapabilityNode): number => {
    const descendants = children.get(node.id) ?? []
    const y = descendants.length
      ? descendants.map(visit).reduce((total, value) => total + value, 0) / descendants.length
      : leafIndex++ * 172
    positions.set(node.id, { x: node.depth * 328, y })
    return y
  }
  const root = nodes.find((node) => node.isRoot)
  if (root) visit(root)
  const height = Math.max(0, (leafIndex - 1) * 172)
  positions.forEach((position, id) => positions.set(id, { ...position, y: position.y - height / 2 }))
  return positions
}

export function capabilityStatusLabel(status: CapabilityStatus) {
  return status === 'healthy' ? '文档充分' : status === 'partial' ? '文档待补' : '尚无文档'
}

export function implementationStatusLabel(status: ImplementationStatus) {
  const labels: Record<ImplementationStatus, string> = {
    verified: '实现已定位', declared: '实现待核对', missing: '缺少实现证据', not_applicable: '不适用',
  }
  return labels[status]
}

export function knowledgeMapViewLabel(view: ProjectKnowledgeMapView) {
  return view === 'architecture' ? '技术架构' : view === 'topics' ? '文档主题' : '产品功能'
}

function unavailableGraph(projectName: string, view: ProjectKnowledgeMapView): ProjectCapabilityGraph {
  const rootId = `map-${view}-root`
  const root: ProjectCapabilityNode = {
    id: rootId, view, kind: 'project', label: projectName, detail: '当前节点尚未返回统一知识图谱，请刷新或升级 Windows 节点。',
    color: '#7f8fb3', parentId: '', sectionId: '', depth: 0, childCount: 0, documentCount: 0,
    documents: [], documentPaths: [], entrypoint: '', entrypointSource: 'missing', coverage: [], missingCoverage: [],
    status: 'gap', implementationStatus: 'not_applicable', implementationRefs: [], source: 'profile_template', isRoot: true,
  }
  return { view, title: knowledgeMapViewLabel(view), source: 'unavailable', rootId, nodes: [root], edges: [], stats: { healthy: 0, partial: 0, gap: 0 }, structuralScore: 0, diagnosticStatus: 'unavailable', findings: [] }
}

function documentStatus(value: string): CapabilityStatus {
  return value === 'documented' ? 'healthy' : value === 'partial' ? 'partial' : 'gap'
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
  return `${node.label} ${node.detail} ${node.kind} ${node.documentPaths.join(' ')} ${node.implementationRefs.map((item) => item.reference).join(' ')}`.toLowerCase()
}

function normalizePath(value: string) {
  return String(value ?? '').trim().replace(/\\/g, '/').toLowerCase()
}
