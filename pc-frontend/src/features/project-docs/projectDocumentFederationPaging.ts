import type { DocumentHealthAnalysis } from './projectDocumentModel'

export type FederationNode = NonNullable<DocumentHealthAnalysis['federation']['nodes']>[number]

export interface FederationPage {
  nodes: FederationNode[]
  pagination: {
    returned: number
    total_matching: number
    has_more: boolean
    next_cursor?: string | null
  }
}

export interface FederationBranch {
  nodes: FederationNode[]
  nextCursor: string | null
  total: number
  hasMore: boolean
  loading: boolean
  error: string
  requestId: number
}

export type FederationPagingState = Record<string, FederationBranch>

export function beginFederationPage(
  state: FederationPagingState,
  parentId: string,
  requestId: number,
  append: boolean,
): FederationPagingState {
  const current = state[parentId]
  return { ...state, [parentId]: {
    nodes: append ? current?.nodes ?? [] : [], nextCursor: current?.nextCursor ?? null,
    total: current?.total ?? 0, hasMore: current?.hasMore ?? false,
    loading: true, error: '', requestId,
  } }
}

export function acceptFederationPage(
  state: FederationPagingState,
  parentId: string,
  requestId: number,
  page: FederationPage,
  append: boolean,
): FederationPagingState {
  const current = state[parentId]
  if (!current || current.requestId !== requestId) return state
  const source = append ? [...current.nodes, ...page.nodes] : page.nodes
  const nodes = [...new Map(source.map((node) => [node.id, node])).values()]
  return { ...state, [parentId]: { ...current, nodes,
    nextCursor: page.pagination.next_cursor ?? null,
    total: page.pagination.total_matching, hasMore: page.pagination.has_more,
    loading: false, error: '',
  } }
}

export function rejectFederationPage(
  state: FederationPagingState,
  parentId: string,
  requestId: number,
  error: string,
): FederationPagingState {
  const current = state[parentId]
  if (!current || current.requestId !== requestId) return state
  return { ...state, [parentId]: { ...current, loading: false, error } }
}
