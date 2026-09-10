import { parseResearchResult } from './browserResearchModel'
import type { ResearchAction } from './types'
import type { ResearchClaim, ResearchReceipt } from './browserResearchApi'
import { nativeResearchErrorCode } from './browserResearchErrors'
import { nativeResearchCommand } from './browserResearchHost'

interface ExecutorDependencies {
  heartbeat: (owner: string) => Promise<string>
  pending: (instance: string) => Promise<ResearchAction[]>
  claim: (id: string, instance: string) => Promise<ResearchClaim>
  receipt: (id: string, receipt: ResearchReceipt) => Promise<unknown>
  invoke: (projectKey: string, ownerKey: string, command: ResearchAction['command']) => Promise<unknown>
  owner: () => string
  now: () => number
}

/** One flight per bridge. Claims are the replay boundary; no automatic native-command retry. */
export function createResearchExecutor(deps: ExecutorDependencies) {
  let running = false
  let disposed = false
  const processed = new Set<string>()
  const receipts = new Map<string, { value: ResearchReceipt; expires: number; owner: string }>()

  async function flush() {
    for (const [id, receipt] of receipts) {
      if (disposed) return
      if (receipt.expires <= deps.now()) { receipts.delete(id); continue }
      if (receipt.owner !== deps.owner()) {
        // Permanently drop the old owner's result, including during logout.
        receipt.value = { claim_token: receipt.value.claim_token, status: 'host_unavailable', error_code: 'host_unavailable' }
        receipt.owner = deps.owner()
      }
      try { await deps.receipt(id, receipt.value); receipts.delete(id) } catch { return }
    }
  }
  async function poll() {
    if (running || disposed) return
    running = true
    try {
      await flush()
      // A failed delivery must not accumulate arbitrary private content in memory.
      if (receipts.size > 0 || disposed || !deps.owner()) return
      const pollingOwner = deps.owner()
      const instance = await deps.heartbeat(pollingOwner)
      if (disposed || deps.owner() !== pollingOwner) return
      const pending = await deps.pending(instance)
      for (const candidate of pending) {
        if (disposed || deps.owner() !== pollingOwner) return
        if (candidate.instance_id !== instance) continue
        if (processed.has(candidate.action_id) || candidate.expires_at_ms <= deps.now()) continue
        let claim: ResearchClaim
        try { claim = await deps.claim(candidate.action_id, instance) } catch { continue }
        const { action, claim_token } = claim
        if (action.action_id !== candidate.action_id) continue
        processed.add(action.action_id)
        if (processed.size > 256) processed.delete(processed.values().next().value as string)
        const ownerKey = deps.owner()
        let receipt: ResearchReceipt
        if (disposed || !ownerKey || ownerKey !== pollingOwner || action.instance_id !== instance || action.expires_at_ms <= deps.now()) {
          receipt = { claim_token, status: 'host_unavailable', error_code: 'host_unavailable' }
        } else {
          try {
            const value = await deps.invoke(action.project_key, ownerKey, nativeResearchCommand(action.command))
            if (disposed || deps.owner() !== ownerKey) {
              receipt = { claim_token, status: 'host_unavailable', error_code: 'host_unavailable' }
            } else {
              receipt = { claim_token, status: 'succeeded', result: parseResearchResult(value, action.command) }
            }
          } catch (error) {
            const code = nativeResearchErrorCode(error)
            receipt = { claim_token, status: code === 'host_unavailable' ? 'host_unavailable' : 'failed', error_code: code }
          }
        }
        // The separate research receipt never enters general Win diagnostics.
        if (disposed) return
        receipts.set(action.action_id, { value: receipt, expires: action.expires_at_ms, owner: ownerKey })
        await flush()
        if (receipts.size > 0) return
      }
    } catch { /* Poll failure is retried on the next interval without logging private data. */ }
    finally { running = false }
  }
  return { poll, dispose: () => { disposed = true; receipts.clear() } }
}
