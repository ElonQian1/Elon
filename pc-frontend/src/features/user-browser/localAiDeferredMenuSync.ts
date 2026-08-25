import {
  getLocalAiWebSessionState,
  runLocalAiWebAdapterCommand,
  waitForLocalAiAdapterResults,
  type LocalAiAdapterAction,
  type LocalAiWebSessionState,
} from './localAiBrowserApi'
import { findMatchingLocalAiCommandReceipt } from './localAiCommandReceipt'

const MENU_OPEN_SETTLE_MS = 180
const inFlightMenuSyncs = new Map<string, Promise<LocalAiWebSessionState | null>>()

interface LocalAiDeferredMenuSyncRequest {
  providerId: string
  ownerKey: string
  sessionIdentity: string
  listAction: LocalAiAdapterAction
  collectAction: LocalAiAdapterAction
}

export function syncLocalAiDeferredMenu(
  request: LocalAiDeferredMenuSyncRequest,
): Promise<LocalAiWebSessionState | null> {
  const key = `${request.sessionIdentity}:${request.listAction}`
  const existing = inFlightMenuSyncs.get(key)
  if (existing) return existing
  const task = performLocalAiDeferredMenuSync(request)
    .finally(() => { inFlightMenuSyncs.delete(key) })
  inFlightMenuSyncs.set(key, task)
  return task
}

async function performLocalAiDeferredMenuSync({
  providerId,
  ownerKey,
  listAction,
  collectAction,
}: LocalAiDeferredMenuSyncRequest): Promise<LocalAiWebSessionState | null> {
  const listRequestId = await runLocalAiWebAdapterCommand(providerId, ownerKey, listAction)
  await new Promise((resolve) => window.setTimeout(resolve, MENU_OPEN_SETTLE_MS))
  let next = await getLocalAiWebSessionState(providerId, ownerKey)
  const listReceipt = findMatchingLocalAiCommandReceipt(
    next.commandResult,
    next.commandResults,
    listAction,
    listRequestId,
  )
  if (listReceipt) {
    return next.commandResult === listReceipt ? next : { ...next, commandResult: listReceipt }
  }

  const collectRequestId = await runLocalAiWebAdapterCommand(providerId, ownerKey, collectAction)
  next = await waitForLocalAiAdapterResults(providerId, ownerKey, [
    { action: listAction, requestId: listRequestId },
    { action: collectAction, requestId: collectRequestId },
  ]) ?? next
  return next
}
