import { safeNodeAdminUrl } from '../../lib/utils'
import { nodeApi } from '../node/localNodeApi'
import { parseResearchAction, parseResearchResult, record, ResearchError } from './browserResearchModel'
import type { ResearchAction, ResearchCommand, ResearchResult } from './types'
import { receiptErrorCode, type ResearchFailureCode } from './browserResearchErrors'
import type { ResearchHost } from './browserResearchHost'

const BASE = '/api/browser-research/actions'
export interface ResearchClaim { action: ResearchAction; claim_token: string }
export interface ResearchReceipt {
  claim_token: string
  status: 'succeeded' | 'failed' | 'host_unavailable'
  result?: ResearchResult
  error_code?: ResearchFailureCode
}
async function call(path: string, options?: RequestInit): Promise<Record<string, unknown>> {
  let value: unknown
  try { value = await nodeApi<unknown>(safeNodeAdminUrl(), path, options) }
  catch (error) { throw new ResearchError(receiptErrorCode(error instanceof Error ? error.message : null)) }
  if (!record(value) || value.ok !== true) throw new ResearchError(record(value) ? receiptErrorCode(value.error) : 'operation_failed')
  return value
}
export async function heartbeatResearchHost(host: ResearchHost): Promise<void> {
  await call('/api/browser-research/hosts/heartbeat', { method: 'POST', body: JSON.stringify(host) })
}
export async function pendingResearchActions(instance: string): Promise<ResearchAction[]> {
  const value = await call(`${BASE}/pending?limit=8&instance_id=${encodeURIComponent(instance)}`)
  if (!Array.isArray(value.actions) || value.actions.length > 8) throw new ResearchError('invalid_response')
  return value.actions.map(parseResearchAction)
}
export async function claimResearchAction(id: string, instance: string): Promise<ResearchClaim> {
  const value = await call(`${BASE}/${encodeURIComponent(id)}/claim`, { method: 'POST', body: JSON.stringify({ instance_id: instance }) })
  if (typeof value.claim_token !== 'string' || !value.claim_token || value.claim_token.length > 256) {
    throw new ResearchError('invalid_response')
  }
  const action = parseResearchAction(value.action)
  if (action.action_id !== id) throw new ResearchError('invalid_response')
  return { action, claim_token: value.claim_token }
}
export function postResearchReceipt(id: string, receipt: ResearchReceipt): Promise<unknown> {
  return call(`${BASE}/${encodeURIComponent(id)}/receipt`, { method: 'POST', body: JSON.stringify(receipt) })
}
export async function submitResearchCommand(projectRoot: string, command: ResearchCommand): Promise<ResearchAction> {
  const value = await call(BASE, { method: 'POST', body: JSON.stringify({ project_root: projectRoot, command }) })
  return parseResearchAction(value.action)
}
export async function getResearchAction(id: string): Promise<{ action: ResearchAction; terminal: boolean }> {
  const value = await call(`${BASE}/${encodeURIComponent(id)}`)
  if (typeof value.terminal !== 'boolean') throw new ResearchError('invalid_response')
  const action = parseResearchAction(value.action)
  if (action.action_id !== id) throw new ResearchError('invalid_response')
  return { action, terminal: value.terminal }
}
export async function runResearchCommand(
  projectRoot: string, command: ResearchCommand, signal: AbortSignal,
): Promise<ResearchResult> {
  if (signal.aborted) throw new ResearchError('cancelled')
  const submitted = await submitResearchCommand(projectRoot, command)
  const deadline = Date.now() + 45_000
  while (!signal.aborted && Date.now() < deadline) {
    const { action, terminal } = await getResearchAction(submitted.action_id)
    if (signal.aborted) throw new ResearchError('cancelled')
    if (action.project_key !== submitted.project_key) throw new ResearchError('invalid_response')
    if (terminal) {
      if (action.status !== 'succeeded' || !record(action.receipt) || action.receipt.status !== 'succeeded') {
        const code = record(action.receipt) ? receiptErrorCode(action.receipt.error_code) : 'operation_failed'
        throw new ResearchError(action.status === 'host_unavailable' ? 'host_unavailable' : code)
      }
      return parseResearchResult(action.receipt.result, command)
    }
    await new Promise<void>((resolve) => {
      const done = () => { clearTimeout(timer); signal.removeEventListener('abort', done); resolve() }
      const timer = setTimeout(done, 500)
      signal.addEventListener('abort', done, { once: true })
    })
  }
  throw new ResearchError(signal.aborted ? 'cancelled' : 'timeout')
}
