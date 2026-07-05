import { api } from '../../api/client'
import { formatBytes } from '../projects/nodeHelpers'
import type { NodeAgentVersion, NodeBalanceResponse, NodeSummary, NodeUsageResponse } from './types'

export { formatBytes }

export function nodeId(node: NodeSummary): string {
  return String(node.node_id ?? node.agent_id ?? '').trim()
}

export function nodeName(node: NodeSummary): string {
  return String(node.display_name ?? node.device_name ?? node.label ?? node.short_id ?? node.node_id ?? 'PC 节点').trim()
}

export function nodeSummaryLine(node: NodeSummary): string {
  const status = node.online ? '在线' : '离线'
  const cap = String(node.capacity_label ?? '').trim()
  return cap ? `${status} · ${cap}` : status
}

export function nodeCanAcceptProject(node: NodeSummary): boolean {
  const value = node.can_accept_project
  if (typeof value === 'boolean') return value
  if (typeof value === 'number') return value !== 0
  if (typeof value === 'string') return value !== 'false' && value !== '0'
  return !!node.online
}

export function capacityText(node: NodeSummary): string {
  const count = Number(node.project_count ?? 0)
  const limit = Number(node.project_limit ?? 0)
  return limit > 0 ? `${count}/${limit} 个项目` : `${count} 个项目`
}

export function formatDateTime(value: unknown): string {
  if (!value) return ''
  const date = new Date(String(value))
  if (isNaN(date.getTime())) return String(value).slice(0, 16)
  return date.toLocaleString('zh-CN', { hour12: false })
}

export function formatUnixTime(value: unknown): string {
  const ts = Number(value ?? 0)
  if (!ts) return ''
  return formatDateTime(new Date(ts * 1000).toISOString())
}

export async function fetchMyNodes(): Promise<NodeSummary[]> {
  const data = await api.get<{ nodes?: NodeSummary[] }>('/api/me/nodes')
  return data.nodes ?? []
}

export async function fetchMarketNodes(): Promise<NodeSummary[]> {
  const data = await api.get<{ nodes?: NodeSummary[] }>('/api/nodes')
  return data.nodes ?? []
}

export async function fetchNodeBalance(): Promise<NodeBalanceResponse> {
  return api.get('/api/me/node-balance')
}

export async function fetchNodeUsage(): Promise<NodeUsageResponse> {
  return api.get('/api/me/node-usage')
}

export async function fetchNodeAgentVersion(): Promise<NodeAgentVersion> {
  return api.get('/api/node-agent/version')
}
