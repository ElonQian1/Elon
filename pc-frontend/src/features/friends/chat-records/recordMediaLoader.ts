import { getAuthToken } from '../../../api/client'
import { resolveApiUrl } from '../../../api/runtime'
import { useAuthStore } from '../../../store/auth'
import { recordPath, recordRequest, type RecordCard } from './recordApi'
import { RECORD_MEDIA_LIMIT, recordMediaCache } from './recordMediaCache'

type RecordIdentity = Pick<RecordCard, 'group_id' | 'record_id'>
interface Flight { controller: AbortController; promise: Promise<Blob>; users: number }
const flights = new Map<string, Flight>()
let generation = 0
const cancelled = () => new DOMException('记录已关闭或账号已变化', 'AbortError')
function ownerPrefix(owner: string) { return `${encodeURIComponent(owner)}|` }
function scope(card: RecordIdentity, owner: string) {
  return `${ownerPrefix(owner)}${new URL(resolveApiUrl(recordPath(card)), location.href).href}|`
}
export function clearRecordMedia(card: RecordIdentity, owner = useAuthStore.getState().user?.id || '') {
  generation++
  const prefix = scope(card, owner)
  for (const [key, flight] of flights) if (key.startsWith(prefix)) { flight.controller.abort(); flights.delete(key) }
  return recordMediaCache.clear(prefix)
}
useAuthStore.subscribe((state, previous) => {
  if (state.token === previous.token && state.user?.id === previous.user?.id) return
  generation++
  for (const flight of flights.values()) flight.controller.abort()
  flights.clear()
  if (previous.user?.id) void recordMediaCache.clear(ownerPrefix(previous.user.id))
})

/** Called only for attachments in a successfully authorized reader document. */
export async function loadRecordMedia(card: RecordIdentity, asset: string, signal: AbortSignal): Promise<Blob> {
  const token = getAuthToken(), owner = useAuthStore.getState().user?.id || '', epoch = generation
  const current = () => !signal.aborted && token === getAuthToken() && epoch === generation && owner === (useAuthStore.getState().user?.id || '')
  if (!token || !current()) throw cancelled()
  const key = `${scope(card, owner)}${encodeURIComponent(asset)}`
  // A missing owner must not create a shared anonymous disk namespace.
  const cached = owner ? await recordMediaCache.get(key) : undefined
  if (!current()) throw cancelled()
  if (cached) return cached
  let flight = flights.get(key)
  if (!flight || flight.controller.signal.aborted) {
    const controller = new AbortController()
    const live = () => !controller.signal.aborted && epoch === generation && token === getAuthToken()
    const request = async () => {
      const timeout = setTimeout(() => controller.abort(), 30_000)
      try {
        const value = await recordRequest(`${recordPath(card)}/assets/${encodeURIComponent(asset)}`, controller.signal, true) as Blob
        if (!live()) throw cancelled()
        if (!value.size || value.size > RECORD_MEDIA_LIMIT) throw Error('附件过大或内容为空')
        if (owner) void recordMediaCache.put(key, value, live)
        return value
      } catch (error) {
        // A denied asset must not leave other local bytes of that record reusable.
        if (error instanceof Error && error.message.includes('记录已撤回')) await clearRecordMedia(card, owner)
        throw error
      } finally { clearTimeout(timeout) }
    }
    flight = { controller, promise: request(), users: 0 }
    flights.set(key, flight)
    const active = flight
    void flight.promise.finally(() => { if (flights.get(key) === active) flights.delete(key) }).catch(() => {})
  }
  const active = flight
  active.users++
  return new Promise((resolve, reject) => {
    let settled = false
    const finish = (blob?: Blob, error?: unknown) => {
      if (settled) return
      settled = true; signal.removeEventListener('abort', abort); active.users--
      if (!active.users && error) active.controller.abort()
      if (error || !current()) reject(error || cancelled()); else resolve(blob!)
    }
    const abort = () => finish(undefined, cancelled())
    signal.addEventListener('abort', abort, { once: true })
    if (signal.aborted) abort()
    active.promise.then(blob => finish(blob), error => finish(undefined, error))
  })
}
