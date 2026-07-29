import {
  sanitizeDiscussionGraph,
  type DiscussionGraph,
  type DiscussionNode,
  type DiscussionSource,
} from './projectDocumentDiscussionModel'

export interface DiscussionPromotionPreview {
  id: string
  nodeId: string
  path: string
  title: string
  documentType: string
  sectionId: string
}

export interface DiscussionGraphProposalView {
  status: string
  summary: string
  changeKind: string
  actor: string
  graph: DiscussionGraph
  promotions: DiscussionPromotionPreview[]
  documentsRead: number
  estimatedTokensUsed: number
}

export interface DiscussionProposalDiff {
  newNodes: DiscussionNode[]
  changedNodes: DiscussionNode[]
  newSources: DiscussionSource[]
  changedSources: DiscussionSource[]
  newEdges: number
}

export function parseDiscussionProposal(content: string): DiscussionGraphProposalView | null {
  try {
    const raw = JSON.parse(content) as Record<string, unknown>
    const status = text(raw.status, 40)
    if (status !== 'ready') return null
    return {
      status,
      summary: text(raw.summary, 1_000),
      changeKind: text(raw.change_kind, 40),
      actor: text(raw.actor, 160),
      graph: sanitizeDiscussionGraph(raw.graph),
      promotions: promotions(raw.promotions),
      documentsRead: count(raw.documents_read),
      estimatedTokensUsed: count(raw.estimated_tokens_used),
    }
  } catch {
    return null
  }
}

export function discussionProposalDiff(
  current: DiscussionGraph,
  proposal: DiscussionGraphProposalView,
): DiscussionProposalDiff {
  const currentNodes = new Map(current.nodes.map((node) => [node.id, node]))
  const currentSources = new Map(current.sources.map((source) => [source.id, source]))
  const currentEdges = new Set(current.edges.map((edge) => edge.id))
  return {
    newNodes: proposal.graph.nodes.filter((node) => !currentNodes.has(node.id)),
    changedNodes: proposal.graph.nodes.filter((node) => {
      const before = currentNodes.get(node.id)
      return !!before && JSON.stringify(before) !== JSON.stringify(node)
    }),
    newSources: proposal.graph.sources.filter((source) => !currentSources.has(source.id)),
    changedSources: proposal.graph.sources.filter((source) => {
      const before = currentSources.get(source.id)
      return !!before && JSON.stringify(before) !== JSON.stringify(source)
    }),
    newEdges: proposal.graph.edges.filter((edge) => !currentEdges.has(edge.id)).length,
  }
}

function promotions(value: unknown): DiscussionPromotionPreview[] {
  if (!Array.isArray(value)) return []
  return value.slice(0, 512).flatMap((raw) => {
    if (!raw || typeof raw !== 'object') return []
    const item = raw as Record<string, unknown>
    const id = text(item.id, 120)
    const nodeId = text(item.node_id, 100)
    const path = text(item.path, 500).replace(/\\/g, '/')
    const title = text(item.title, 160)
    if (!id || !nodeId || !path || !title) return []
    return [{
      id,
      nodeId,
      path,
      title,
      documentType: text(item.document_type, 40),
      sectionId: text(item.section_id, 100),
    }]
  })
}

function text(value: unknown, limit: number) {
  return String(value ?? '').trim().slice(0, limit)
}

function count(value: unknown) {
  const result = Number(value)
  return Number.isFinite(result) ? Math.max(0, Math.floor(result)) : 0
}
