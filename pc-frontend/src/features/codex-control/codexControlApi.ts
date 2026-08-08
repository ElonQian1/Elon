import { safeNodeAdminUrl } from '../../lib/utils'
import { nodeApi } from '../node/localNodeApi'
import type { WinActionKind, WinControlAction, WinControlCapabilities, WinLogSource, WinTimelineResponse } from './types'

const BASE = '/api/codex-control'

export async function fetchWinControlCapabilities(): Promise<WinControlCapabilities> {
  const response = await nodeApi<{ capabilities: WinControlCapabilities }>(safeNodeAdminUrl(), `${BASE}/capabilities`)
  return response.capabilities
}

export function fetchWinTimeline(sources: WinLogSource[], limit = 300): Promise<WinTimelineResponse> {
  const query = new URLSearchParams({ limit: String(limit) })
  if (sources.length) query.set('sources', sources.join(','))
  return nodeApi(safeNodeAdminUrl(), `${BASE}/events?${query}`)
}

export function postWinEvent(input: {
  trace_id?: string
  source: 'frontend' | 'network' | 'tauri'
  level?: string
  kind: string
  summary?: string
  fields?: Record<string, unknown>
}): Promise<unknown> {
  return nodeApi(safeNodeAdminUrl(), `${BASE}/events`, {
    method: 'POST',
    body: JSON.stringify(input),
  })
}

export async function queueWinAction(kind: WinActionKind, route?: string): Promise<WinControlAction> {
  const response = await nodeApi<{ action: WinControlAction }>(safeNodeAdminUrl(), `${BASE}/actions`, {
    method: 'POST',
    body: JSON.stringify({ kind, route, requested_by: 'pc_ui', trace_id: `pc_ui:${Date.now()}` }),
  })
  return response.action
}

export async function fetchPendingWinActions(): Promise<WinControlAction[]> {
  const response = await nodeApi<{ actions?: WinControlAction[] }>(safeNodeAdminUrl(), `${BASE}/actions/pending?limit=20`)
  return Array.isArray(response.actions) ? response.actions : []
}

export async function claimWinAction(actionId: string): Promise<WinControlAction> {
  const response = await nodeApi<{ action: WinControlAction }>(
    safeNodeAdminUrl(), `${BASE}/actions/${encodeURIComponent(actionId)}/claim`, { method: 'POST' },
  )
  return response.action
}

export function postWinActionReceipt(
  actionId: string,
  receipt: { status: string; message?: string; route?: string; at_ms?: number },
): Promise<unknown> {
  return nodeApi(safeNodeAdminUrl(), `${BASE}/actions/${encodeURIComponent(actionId)}/receipt`, {
    method: 'POST',
    body: JSON.stringify(receipt),
  })
}

export function fetchWinDiagnostics(): Promise<{ diagnostics?: Record<string, unknown> }> {
  return nodeApi(safeNodeAdminUrl(), `${BASE}/diagnostics`)
}

export function exportWinDiagnostics(): Promise<{ path?: string; message?: string }> {
  return nodeApi(safeNodeAdminUrl(), `${BASE}/export`, { method: 'POST' }, 15_000)
}
