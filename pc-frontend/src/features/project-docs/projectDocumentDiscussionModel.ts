export type DiscussionNodeKind =
  | 'topic' | 'question' | 'claim' | 'hypothesis' | 'option' | 'objection'
  | 'evidence' | 'risk' | 'decision' | 'requirement' | 'feature' | 'task' | 'result'

export type DiscussionNodeStatus =
  | 'open' | 'exploring' | 'accepted' | 'rejected' | 'superseded' | 'implemented'

export interface DiscussionSource {
  id: string
  title: string
  kind: string
  reference: string
  imported_at: string
}

export interface DiscussionNode {
  id: string
  root_id: string
  parent_id: string
  kind: DiscussionNodeKind
  title: string
  summary: string
  status: DiscussionNodeStatus
  authority: string
  section_id: string
  order: number
  color: string
  source_refs: string[]
  conversation_refs: string[]
  document_paths: string[]
  feature_node_ids: string[]
  tags: string[]
}

export interface DiscussionEdge {
  id: string
  source: string
  target: string
  relation: string
  label: string
}

export interface DiscussionGraph {
  version: 1
  sources: DiscussionSource[]
  nodes: DiscussionNode[]
  edges: DiscussionEdge[]
  evolution: DiscussionGraphEvolution
}

export interface DiscussionGraphEvolution {
  kind: string
  summary: string
  actor: string
  changed_at: string
  previous_revision: string
}

export const EMPTY_DISCUSSION_GRAPH: DiscussionGraph = {
  version: 1,
  sources: [],
  nodes: [],
  edges: [],
  evolution: { kind: '', summary: '', actor: '', changed_at: '', previous_revision: '' },
}

const kinds = new Set<DiscussionNodeKind>([
  'topic', 'question', 'claim', 'hypothesis', 'option', 'objection', 'evidence',
  'risk', 'decision', 'requirement', 'feature', 'task', 'result',
])
const statuses = new Set<DiscussionNodeStatus>([
  'open', 'exploring', 'accepted', 'rejected', 'superseded', 'implemented',
])

export function parseDiscussionGraph(content: string): DiscussionGraph {
  try {
    return sanitizeDiscussionGraph(JSON.parse(content))
  } catch {
    return EMPTY_DISCUSSION_GRAPH
  }
}

export function sanitizeDiscussionGraph(value: unknown): DiscussionGraph {
  if (!value || typeof value !== 'object') return EMPTY_DISCUSSION_GRAPH
  const source = value as Partial<DiscussionGraph>
  const sources = Array.isArray(source.sources)
    ? source.sources.slice(0, 512).flatMap((raw) => {
      if (!raw || typeof raw !== 'object') return []
      const item = raw as Partial<DiscussionSource>
      const id = stableId(item.id, 100)
      const title = text(item.title, 160)
      if (!id || !title) return []
      return [{
        id,
        title,
        kind: stableId(item.kind, 40),
        reference: text(item.reference, 1_000),
        imported_at: text(item.imported_at, 64),
      }]
    })
    : []
  const nodes = Array.isArray(source.nodes)
    ? source.nodes.slice(0, 4_096).flatMap((raw) => {
      if (!raw || typeof raw !== 'object') return []
      const item = raw as Partial<DiscussionNode>
      const id = stableId(item.id, 100)
      const title = text(item.title, 120)
      if (!id || !title) return []
      const kind = kinds.has(item.kind as DiscussionNodeKind) ? item.kind as DiscussionNodeKind : 'topic'
      const status = statuses.has(item.status as DiscussionNodeStatus) ? item.status as DiscussionNodeStatus : 'open'
      return [{
        id,
        root_id: stableId(item.root_id, 100),
        parent_id: stableId(item.parent_id, 100),
        kind,
        title,
        summary: text(item.summary, 1_200),
        status,
        authority: stableId(item.authority, 40) || 'source',
        section_id: stableId(item.section_id, 100),
        order: number(item.order),
        color: /^#[0-9a-f]{6}$/i.test(String(item.color ?? '')) ? String(item.color) : colorForKind(kind),
        source_refs: strings(item.source_refs, 48, 300),
        conversation_refs: strings(item.conversation_refs, 24, 300),
        document_paths: strings(item.document_paths, 48, 500).map((path) => path.replace(/\\/g, '/')),
        feature_node_ids: strings(item.feature_node_ids, 48, 100),
        tags: strings(item.tags, 24, 80),
      }]
    })
    : []
  const nodeIds = new Set(nodes.map((node) => node.id))
  const edges = Array.isArray(source.edges)
    ? source.edges.slice(0, 8_192).flatMap((raw) => {
      if (!raw || typeof raw !== 'object') return []
      const item = raw as Partial<DiscussionEdge>
      const id = stableId(item.id, 120)
      const edgeSource = stableId(item.source, 100)
      const target = stableId(item.target, 100)
      if (!id || edgeSource === target || !nodeIds.has(edgeSource) || !nodeIds.has(target)) return []
      return [{
        id,
        source: edgeSource,
        target,
        relation: stableId(item.relation, 40) || 'related_to',
        label: text(item.label, 100),
      }]
    })
    : []
  return {
    version: 1,
    sources: unique(sources, (item) => item.id),
    nodes: unique(nodes, (item) => item.id),
    edges: unique(edges, (item) => item.id),
    evolution: sanitizeEvolution(source.evolution),
  }
}

export function discussionRoots(graph: DiscussionGraph) {
  return graph.nodes
    .filter((node) => !node.parent_id)
    .sort((left, right) => left.order - right.order || left.title.localeCompare(right.title, 'zh-CN'))
}

export function selectDiscussionSubgraph(
  graph: DiscussionGraph,
  rootId: string,
  query: string,
  limit = 400,
) {
  const allowed = new Set<string>()
  if (rootId) {
    allowed.add(rootId)
    for (let pass = 0; pass < 25; pass += 1) {
      const before = allowed.size
      graph.nodes.forEach((node) => { if (allowed.has(node.parent_id)) allowed.add(node.id) })
      if (before === allowed.size) break
    }
  }
  const needle = query.trim().toLowerCase()
  const matches = new Set(graph.nodes.filter((node) => (
    (!allowed.size || allowed.has(node.id))
    && (!needle || `${node.title} ${node.summary} ${node.tags.join(' ')}`.toLowerCase().includes(needle))
  )).map((node) => node.id))
  if (needle) {
    const parents = new Map(graph.nodes.map((node) => [node.id, node.parent_id]))
    for (const id of [...matches]) {
      let parent = parents.get(id)
      while (parent) {
        matches.add(parent)
        parent = parents.get(parent)
      }
    }
  }
  const nodes = graph.nodes
    .filter((node) => matches.has(node.id))
    .sort((left, right) => left.order - right.order || left.title.localeCompare(right.title, 'zh-CN'))
    .slice(0, limit)
  const visible = new Set(nodes.map((node) => node.id))
  return {
    nodes,
    edges: graph.edges.filter((edge) => visible.has(edge.source) && visible.has(edge.target)),
    truncated: matches.size > nodes.length,
  }
}

export function layoutDiscussionNodes(nodes: DiscussionNode[]) {
  const byParent = new Map<string, DiscussionNode[]>()
  nodes.forEach((node) => byParent.set(node.parent_id, [...(byParent.get(node.parent_id) ?? []), node]))
  byParent.forEach((children) => children.sort((a, b) => a.order - b.order || a.title.localeCompare(b.title, 'zh-CN')))
  const positions = new Map<string, { x: number; y: number }>()
  let row = 0
  function visit(node: DiscussionNode, depth: number): number {
    const children = byParent.get(node.id) ?? []
    if (!children.length) {
      const y = row++ * 150
      positions.set(node.id, { x: depth * 310, y })
      return y
    }
    const childRows = children.map((child) => visit(child, depth + 1))
    const y = (childRows[0] + childRows[childRows.length - 1]) / 2
    positions.set(node.id, { x: depth * 310, y })
    return y
  }
  (byParent.get('') ?? []).forEach((node) => visit(node, 0))
  nodes.filter((node) => !positions.has(node.id)).forEach((node) => {
    positions.set(node.id, { x: 0, y: row++ * 150 })
  })
  return positions
}

export function discussionKindLabel(kind: DiscussionNodeKind) {
  return {
    topic: '主题', question: '问题', claim: '主张', hypothesis: '假设', option: '方案',
    objection: '反对意见', evidence: '证据', risk: '风险', decision: '决策',
    requirement: '需求', feature: '功能', task: '任务', result: '结果',
  }[kind]
}

export function discussionStatusLabel(status: DiscussionNodeStatus) {
  return {
    open: '待讨论', exploring: '讨论中', accepted: '已确认', rejected: '已否决',
    superseded: '已替代', implemented: '已实现',
  }[status]
}

function colorForKind(kind: DiscussionNodeKind) {
  if (kind === 'decision' || kind === 'result') return '#55b989'
  if (kind === 'risk' || kind === 'objection') return '#d66f78'
  if (kind === 'hypothesis' || kind === 'question') return '#d8a950'
  if (kind === 'requirement' || kind === 'feature' || kind === 'task') return '#5f91dc'
  if (kind === 'evidence') return '#50aaa7'
  return '#9a73dc'
}

function stableId(value: unknown, limit: number) {
  return String(value ?? '').trim().toLowerCase().replace(/[^a-z0-9._-]+/g, '').slice(0, limit)
}

function text(value: unknown, limit: number) {
  return String(value ?? '').trim().slice(0, limit)
}

function strings(value: unknown, count: number, chars: number) {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === 'string').slice(0, count).map((item) => text(item, chars))
    : []
}

function number(value: unknown) {
  const result = Number(value)
  return Number.isFinite(result) ? Math.max(0, Math.floor(result)) : 0
}

function unique<T>(items: T[], key: (item: T) => string) {
  return [...new Map(items.map((item) => [key(item), item])).values()]
}

function sanitizeEvolution(value: unknown): DiscussionGraphEvolution {
  const item = value && typeof value === 'object' ? value as Partial<DiscussionGraphEvolution> : {}
  return {
    kind: stableId(item.kind, 40),
    summary: text(item.summary, 1_000),
    actor: text(item.actor, 160),
    changed_at: text(item.changed_at, 64),
    previous_revision: text(item.previous_revision, 128),
  }
}
