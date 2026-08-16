import {
  deriveNodeCacheHealth,
  fetchNodeCacheHealth,
  type NodeCacheHealthResponse,
  type NodeCacheHealthView,
} from './nodeCacheHealth'
import { nodeId, nodeName } from './nodeHelpers'
import type { NodeSummary } from './types'

const DEFAULT_CONCURRENCY = 4
const MAX_CONCURRENCY = 8

interface FleetItemBase {
  node: NodeSummary
  nodeId: string
  name: string
}

export type NodeCacheFleetItem = FleetItemBase & (
  | { status: 'ready'; report: NodeCacheHealthResponse; health: NodeCacheHealthView }
  | { status: 'missing' }
  | { status: 'error' }
)

export interface NodeCacheFleetSummary {
  total: number
  healthy: number
  needsAttention: number
  missing: number
  failed: number
}

type ReportFetcher = (nodeId: string) => Promise<NodeCacheHealthResponse | null>

export async function fetchNodeCacheFleet(
  nodes: NodeSummary[],
  concurrency = DEFAULT_CONCURRENCY,
  fetcher: ReportFetcher = fetchNodeCacheHealth,
): Promise<NodeCacheFleetItem[]> {
  const uniqueNodes = deduplicateNodes(nodes)
  if (uniqueNodes.length === 0) return []

  const results = new Array<NodeCacheFleetItem>(uniqueNodes.length)
  const workerCount = Math.min(
    uniqueNodes.length,
    Math.max(1, Math.min(MAX_CONCURRENCY, Math.trunc(concurrency) || DEFAULT_CONCURRENCY)),
  )
  let cursor = 0

  async function worker() {
    while (cursor < uniqueNodes.length) {
      const index = cursor
      cursor += 1
      const node = uniqueNodes[index]
      const id = nodeId(node)
      const base: FleetItemBase = { node, nodeId: id, name: nodeName(node) }
      try {
        const report = await fetcher(id)
        results[index] = report
          ? { ...base, status: 'ready', report, health: deriveNodeCacheHealth(report) }
          : { ...base, status: 'missing' }
      } catch {
        results[index] = { ...base, status: 'error' }
      }
    }
  }

  await Promise.all(Array.from({ length: workerCount }, () => worker()))
  return results.sort(compareFleetItems)
}

export function summarizeNodeCacheFleet(items: NodeCacheFleetItem[]): NodeCacheFleetSummary {
  const summary: NodeCacheFleetSummary = {
    total: items.length,
    healthy: 0,
    needsAttention: 0,
    missing: 0,
    failed: 0,
  }
  for (const item of items) {
    if (item.status === 'missing') summary.missing += 1
    else if (item.status === 'error') summary.failed += 1
    else if (item.health.tone === 'healthy') summary.healthy += 1
    else summary.needsAttention += 1
  }
  return summary
}

function deduplicateNodes(nodes: NodeSummary[]): NodeSummary[] {
  const seen = new Set<string>()
  return nodes.filter((node) => {
    const id = nodeId(node)
    if (!id || seen.has(id)) return false
    seen.add(id)
    return true
  })
}

function compareFleetItems(left: NodeCacheFleetItem, right: NodeCacheFleetItem): number {
  const risk = fleetRisk(left) - fleetRisk(right)
  if (risk !== 0) return risk
  const name = left.name.localeCompare(right.name, 'zh-CN')
  return name || left.nodeId.localeCompare(right.nodeId)
}

function fleetRisk(item: NodeCacheFleetItem): number {
  if (item.status === 'error') return 0
  if (item.status === 'ready' && item.health.tone === 'critical') return 1
  if (item.status === 'ready' && item.health.tone === 'attention') return 2
  if (item.status === 'missing') return 3
  return 4
}
