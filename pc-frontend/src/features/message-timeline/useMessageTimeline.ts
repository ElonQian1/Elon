import { useCallback, useEffect, useRef, useState } from 'react'
import { api, type ApiError } from '../../api/client'
import { useAuthStore } from '../../store/auth'
import { startSocialRefresh } from '../friends/socialRefreshLoop'
import { createTimeline, type Direction, type TimelinePage, type TimelineScope, type TimelineMessage } from './timeline'

/** One serial request chain; account/scope cancellation fences both pages and checkpoints. */
export function useMessageTimeline<T extends TimelineMessage>(scope: TimelineScope | null, onMessages: (rows: T[]) => void) {
  const owner = useAuthStore(s => s.user?.id)
  const token = useAuthStore(s => s.token)
  const key = JSON.stringify([owner, token, scope])
  const identity = useRef(key); identity.current = key
  const callback = useRef(onMessages); callback.current = onMessages
  const action = useRef<(direction: Direction) => Promise<void>>(async () => {})
  const followRef = useRef<(value: boolean) => void>(() => {})
  const [status, setStatus] = useState({ loading: false, error: '', hasOlder: false, hasNewer: false, unread: 0 })
  useEffect(() => {
    if (!scope || !owner || !token) return
    const timeline = createTimeline<T>()
    let stopped = false, busy = false, pending: Direction | null = null
    let controller: AbortController | null = null
    let receipt: AbortController | null = null, frame = 0
    let readId = ''
    const current = () => !stopped && identity.current === key && useAuthStore.getState().token === token && useAuthStore.getState().user?.id === owner
    function acknowledgeAfterPaint() {
      cancelAnimationFrame(frame)
      frame = requestAnimationFrame(() => { frame = requestAnimationFrame(async () => {
        const next = timeline.snapshot(), id = next.messages[next.messages.length - 1]?.id
        if (!current() || !next.following || next.hasNewer || document.hidden || !document.hasFocus() || !id || id === readId) return
        receipt?.abort(); receipt = new AbortController()
        const ack = receipt, deadline = setTimeout(() => ack.abort(), 12000)
        try {
          await api.post('/api/me/message-timeline/read', { ...scope, message_id: id }, { signal: ack.signal })
          if (current()) readId = id
        } catch { /* A failed receipt must not discard successfully delivered messages. */ }
        finally { clearTimeout(deadline) }
      }) })
    }
    async function read(direction: Direction) {
      if (!current() || document.hidden) return
      if (busy) { if (direction !== 'sync') pending = direction; return }
      if (direction === 'older' && !timeline.snapshot().hasOlder) return
      busy = true
      controller = new AbortController()
      const timeout = setTimeout(() => controller?.abort(), 12000)
      setStatus(s => ({ ...s, loading: true, error: '' }))
      try {
        let mode = direction === 'sync' && !timeline.snapshot().sync ? 'latest' as const : direction
        for (let pages = 0; pages < 4; pages++) {
          const page = mode === 'window'
            ? await api.post<TimelinePage<T>>('/api/me/message-timeline/window', { ...scope, message_ids: timeline.snapshot().messages.map(m => m.id) }, { signal: controller.signal })
            : await api.get<TimelinePage<T>>(timeline.query(scope!, mode), { signal: controller.signal, cache: 'no-store' })
          if (!current() || controller.signal.aborted) return
          if (page.reset) { const s = timeline.snapshot(); mode = s.following && !s.hasNewer ? 'latest' : 'window'; continue }
          const next = timeline.apply(page, mode)
          if (mode !== 'sync' || page.messages.length || page.removed_ids.length) callback.current(next.messages)
          setStatus({ loading: false, error: '', hasOlder: next.hasOlder, hasNewer: next.hasNewer, unread: next.unread })
          acknowledgeAfterPaint()
          if (mode !== 'sync' || !page.has_more) break
          if (pages === 3) pending = 'sync'
        }
      } catch (error) {
        if (!current()) return
        const denied = [401, 403, 404].includes((error as ApiError).status)
        if (denied) { timeline.reset(); callback.current([]) }
        setStatus(s => ({ ...s, loading: false, error: denied ? '无法访问此会话，请检查账号或成员权限' : '消息同步失败，已保留现有内容；点击重试' }))
      } finally {
        clearTimeout(timeout); busy = false
        if (current() && pending) { const next = pending; pending = null; void read(next) }
      }
    }
    action.current = read
    followRef.current = value => { const was = timeline.snapshot().following; timeline.follow(value); if (value && !was) { acknowledgeAfterPaint(); void read('sync') } }
    const loop = startSocialRefresh(async () => { await read('sync') }, { interval: 15000 })
    return () => { stopped = true; controller?.abort(); receipt?.abort(); cancelAnimationFrame(frame); loop.stop(); action.current = async () => {}; followRef.current = () => {} }
  // key includes the complete immutable scope and session identity.
  }, [key])
  return { ...status, older: useCallback(() => action.current('older'), []), latest: useCallback(() => action.current('latest'), []),
    refresh: useCallback(() => action.current('sync'), []), follow: useCallback((value: boolean) => followRef.current(value), []) }
}
