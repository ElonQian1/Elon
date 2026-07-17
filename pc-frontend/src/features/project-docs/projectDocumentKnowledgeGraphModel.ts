export type ProjectKnowledgeMapView = 'capabilities' | 'architecture' | 'topics'
export type DocumentationStatus = 'documented' | 'partial' | 'undocumented'
export type ImplementationStatus = 'verified' | 'declared' | 'missing' | 'not_applicable'

export interface ProjectKnowledgeNodeConfig {
  id: string
  view: 'capabilities' | 'architecture'
  kind: string
  label: string
  detail: string
  parent_id: string
  order: number
  color: string
  entrypoint: string
  document_paths: string[]
  implementation_refs: string[]
  tags: string[]
}

export interface ProjectKnowledgeEdgeConfig {
  id: string
  source: string
  target: string
  relation: string
  label: string
}

export interface ProjectKnowledgeGraphConfig {
  nodes: ProjectKnowledgeNodeConfig[]
  edges: ProjectKnowledgeEdgeConfig[]
}

export interface ProjectKnowledgeMapEvidence {
  reference: string
  verification: 'exists' | 'missing' | 'declared'
}

export interface ProjectKnowledgeMapCoverage {
  key: 'overview' | 'requirements' | 'architecture' | 'reference' | 'operations' | 'evidence'
  label: string
  covered: boolean
  count: number
}

export interface ProjectKnowledgeMapNode {
  id: string
  view: ProjectKnowledgeMapView
  kind: string
  label: string
  detail: string
  color: string
  parent_id: string
  section_id: string
  depth: number
  child_count: number
  order: number
  document_count: number
  document_paths: string[]
  entrypoint: string
  entrypoint_source: 'configured' | 'inferred' | 'missing'
  coverage: ProjectKnowledgeMapCoverage[]
  missing_coverage: string[]
  documentation_status: DocumentationStatus
  implementation_refs: ProjectKnowledgeMapEvidence[]
  implementation_status: ImplementationStatus
  source: 'manifest' | 'profile_template'
  tags: string[]
}

export interface ProjectKnowledgeMapEdge {
  id: string
  source: string
  target: string
  relation: string
  label: string
  configured: boolean
}

export interface ProjectKnowledgeMapFinding {
  code: string
  severity: 'error' | 'warning' | 'info'
  node_id: string
  message: string
  suggested_action: string
}

export interface ProjectKnowledgeMap {
  version: 1
  view: ProjectKnowledgeMapView
  title: string
  source: 'manifest' | 'profile_template'
  root_id: string
  nodes: ProjectKnowledgeMapNode[]
  edges: ProjectKnowledgeMapEdge[]
  stats: {
    nodes: number
    configured_nodes: number
    documented: number
    partial: number
    undocumented: number
    implementation_verified: number
    implementation_declared: number
    implementation_missing: number
  }
  diagnostics: {
    structural_score: number
    status: 'healthy' | 'review' | 'needs_structure'
    findings: ProjectKnowledgeMapFinding[]
  }
  budget: {
    classification_model_tokens: 0
    markdown_bodies_read: 0
    metadata_only: true
  }
}

export interface ProjectKnowledgeMaps {
  capabilities: ProjectKnowledgeMap
  architecture: ProjectKnowledgeMap
  topics: ProjectKnowledgeMap
}

export const EMPTY_KNOWLEDGE_GRAPH: ProjectKnowledgeGraphConfig = { nodes: [], edges: [] }

export function sanitizeKnowledgeGraphConfig(value: unknown): ProjectKnowledgeGraphConfig {
  if (!value || typeof value !== 'object') return { nodes: [], edges: [] }
  const candidate = value as Partial<ProjectKnowledgeGraphConfig>
  const nodes = Array.isArray(candidate.nodes) ? candidate.nodes.slice(0, 256).flatMap((raw) => {
    if (!raw || typeof raw !== 'object') return []
    const node = raw as Partial<ProjectKnowledgeNodeConfig>
    const id = stableId(node.id, 80)
    const view = node.view === 'capabilities' || node.view === 'architecture' ? node.view : null
    const label = String(node.label ?? '').trim().slice(0, 60)
    if (!id || !view || !label) return []
    return [{
      id,
      view,
      kind: stableId(node.kind, 40) || (view === 'capabilities' ? 'capability' : 'component'),
      label,
      detail: String(node.detail ?? '').trim().slice(0, 240),
      parent_id: stableId(node.parent_id, 80),
      order: boundedNumber(node.order, 9_999),
      color: /^#[0-9a-f]{6}$/i.test(String(node.color ?? '')) ? String(node.color) : '#7f8fb3',
      entrypoint: normalizedPath(node.entrypoint),
      document_paths: stringArray(node.document_paths, 48).map(normalizedPath).filter(Boolean),
      implementation_refs: stringArray(node.implementation_refs, 48).map((item) => item.slice(0, 500)),
      tags: stringArray(node.tags, 24).map((item) => item.slice(0, 80)),
    }]
  }) : []
  const nodeIds = new Set(nodes.map((node) => node.id))
  const edges = Array.isArray(candidate.edges) ? candidate.edges.slice(0, 512).flatMap((raw) => {
    if (!raw || typeof raw !== 'object') return []
    const edge = raw as Partial<ProjectKnowledgeEdgeConfig>
    const id = stableId(edge.id, 100)
    const source = stableId(edge.source, 80)
    const target = stableId(edge.target, 80)
    if (!id || source === target || !nodeIds.has(source) || !nodeIds.has(target)) return []
    return [{
      id,
      source,
      target,
      relation: stableId(edge.relation, 40) || 'related_to',
      label: String(edge.label ?? '').trim().slice(0, 80),
    }]
  }) : []
  return {
    nodes: [...new Map(nodes.map((node) => [node.id, node])).values()],
    edges: [...new Map(edges.map((edge) => [edge.id, edge])).values()],
  }
}

function stableId(value: unknown, limit: number) {
  return String(value ?? '').trim().toLowerCase().replace(/[^a-z0-9._-]+/g, '').slice(0, limit)
}

function normalizedPath(value: unknown) {
  return String(value ?? '').trim().replace(/\\/g, '/')
}

function stringArray(value: unknown, limit: number) {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === 'string').slice(0, limit)
    : []
}

function boundedNumber(value: unknown, max: number) {
  const number = Number(value)
  return Number.isFinite(number) ? Math.min(max, Math.max(0, Math.floor(number))) : 0
}
