import { sha256 } from '@noble/hashes/sha2.js'
import { bytesToHex, utf8ToBytes } from '@noble/hashes/utils.js'
import { resolveApiUrl } from '../../../api/runtime'
import { useAuthStore } from '../../../store/auth'
import { recordPath, type RecordCard, type RecordView } from './recordApi'
import { RecordMediaCache } from './recordMediaCache'

export const RECORD_DOCUMENT_LEASE_MS = 5 * 60_000
type Identity = Pick<RecordCard, 'group_id' | 'record_id'>
export interface CachedRecordDocument { validatedAt: number; view: RecordView }
const cache = new RecordMediaCache({ database: 'elon-record-documents-v1', memoryBytes: 4 * 1024 * 1024,
  diskBytes: 8 * 1024 * 1024, maxEntries: 16, maxAge: RECORD_DOCUMENT_LEASE_MS })
let generation = 0
export const recordDocumentGeneration = () => generation
function prefix(card: Identity, owner: string) {
  return `${encodeURIComponent(owner)}|${new URL(resolveApiUrl(recordPath(card)), location.href).href}|`
}
function key(card: Identity, owner: string, token: string) {
  // A session fingerprint isolates disk entries without persisting credentials.
  return prefix(card, owner) + bytesToHex(sha256(utf8ToBytes(token)))
}
export function documentLeaseRemaining(entry: CachedRecordDocument, now = Date.now()) {
  const age = now - entry.validatedAt
  return Number.isFinite(age) && age >= 0 ? Math.max(0, RECORD_DOCUMENT_LEASE_MS - age) : 0
}
export async function readRecordDocument(card: Identity, owner: string, token: string): Promise<CachedRecordDocument | undefined> {
  if (!owner || !token) return
  const blob = await cache.get(key(card, owner, token))
  if (!blob) return
  try {
    const entry = JSON.parse(await blob.text()) as CachedRecordDocument
    const view = entry?.view
    if (documentLeaseRemaining(entry) && view?.card?.group_id === card.group_id && view.card.record_id === card.record_id
      && typeof view.owner_id === 'string' && Array.isArray(view.document?.messages) && Array.isArray(view.document.warnings)
      && typeof view.document.title === 'string' && typeof view.document.raw_text === 'string') return entry
  } catch { /* Invalid or obsolete local content requires a normal server read. */ }
}
export function saveRecordDocument(card: Identity, owner: string, token: string, entry: CachedRecordDocument, current: () => boolean) {
  if (!owner || !token || !current()) return Promise.resolve()
  return cache.put(key(card, owner, token), new Blob([JSON.stringify(entry)], { type: 'application/json' }), current)
}
export function clearRecordDocument(card: Identity, owner: string) {
  generation++
  return cache.clear(prefix(card, owner))
}
useAuthStore.subscribe((state, previous) => {
  if (state.token === previous.token && state.user?.id === previous.user?.id) return
  generation++
  if (previous.user?.id) void cache.clear(`${encodeURIComponent(previous.user.id)}|`)
})
