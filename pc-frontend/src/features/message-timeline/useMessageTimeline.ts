import { useCallback, useEffect, useRef, useState } from 'react'
import { api, type ApiError } from '../../api/client'
import { useAuthStore } from '../../store/auth'
import { startSocialRefresh } from '../friends/socialRefreshLoop'
import { createTimeline, type Direction, type TimelinePage, type TimelineScope, type TimelineMessage, type ReadingTarget } from './timeline'

/** One serial request chain; account/scope cancellation fences both pages and checkpoints. */
export function useMessageTimeline<T extends TimelineMessage>(scope: TimelineScope | null, onMessages: (rows: T[]) => void) {
  const owner = useAuthStore(s => s.user?.id)
  const token = useAuthStore(s => s.token)
  const key = JSON.stringify([owner, token, scope])
  const identity = useRef(key); identity.current = key
  const callback = useRef(onMessages); callback.current = onMessages
  const action = useRef<(direction: Direction, target?: Record<string, string>) => Promise<ReadingTarget | boolean | undefined>>(async () => undefined)
  const followRef = useRef<(value: boolean) => void>(() => {})
  const [status, setStatus] = useState({ loading: false, error: '', hasOlder: false, hasNewer: false, unread: 0 })
  useEffect(() => {
    if (!scope || !owner || !token) return
    const timeline = createTimeline<T>()
    let stopped = false, busy = false, pending: Direction | null = null
    let controller: AbortController | null = null
    let receipt: AbortController | null = null, frame = 0
    let readId = ''
    let generation = 0, version = 1
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
    async function read(direction: Direction, target?: Record<string, string>): Promise<ReadingTarget | boolean | undefined> {
      if (!current() || document.hidden) return
      if (direction === 'around' || direction === 'latest') { controller?.abort(); generation++; busy = false; pending = null }
      if (busy) { if (direction !== 'sync') pending = direction; return }
      if (direction === 'older' && !timeline.snapshot().hasOlder) return
      if (direction === 'newer' && !timeline.snapshot().hasNewer) return
      const previous = timeline.snapshot()
      if (direction === 'around') { timeline.upgrade(); version = 2; timeline.follow(false) }
      const ticket = ++generation
      busy = true
      controller = new AbortController()
      const request = controller
      const timeout = setTimeout(() => request.abort(), 12000)
      setStatus(s => ({ ...s, loading: true, error: '' }))
      try {
        let mode = direction === 'sync' && !timeline.snapshot().sync ? 'latest' as const : direction
        for (let pages = 0; pages < 4; pages++) {
          const page = mode === 'window'
            ? await api.post<TimelinePage<T>>('/api/me/message-timeline' + (version === 2 ? '/v2' : '') + '/window', { ...scope, message_ids: timeline.snapshot().messages.map(m => m.id) }, { signal: request.signal })
            : await api.get<TimelinePage<T>>(timeline.query(scope!, mode) + (target ? '&' + new URLSearchParams(target) : ''), { signal: request.signal, cache: 'no-store' })
          if (!current() || request.signal.aborted || ticket !== generation) return
          if (page.reset) { const s = timeline.snapshot(); mode = s.following && !s.hasNewer ? 'latest' : 'window'; continue }
          const next = timeline.apply(page, mode)
          if (mode !== 'sync' || page.messages.length || page.removed_ids.length) callback.current(next.messages)
          setStatus({ loading: false, error: '', hasOlder: next.hasOlder, hasNewer: next.hasNewer, unread: next.unread })
          acknowledgeAfterPaint()
          if (mode !== 'sync' || !page.has_more) return page.target || true
          if (pages === 3 && !pending) pending = 'sync'
        }
      } catch (error) {
        if (!current() || ticket !== generation) return
        const status = (error as ApiError).status
        const denied = [401, 403].includes(status) || (status === 404 && direction !== 'around')
        if (denied) { timeline.reset(); callback.current([]) }
        else if (direction === 'around') { version = previous.version; timeline.upgrade(version); timeline.follow(previous.following) }
        setStatus(s => ({ ...s, loading: false, error: denied ? '无法访问此会话，请检查账号或成员权限' : status === 404 ? '书签或消息已不可用，已保留当前消息' : '消息同步失败，已保留现有内容；点击重试' }))
        return false
      } finally {
        clearTimeout(timeout)
        if (ticket === generation) { busy = false; if (current() && pending) { const next = pending; pending = null; void read(next) } }
      }
    }
    action.current = read
    followRef.current = value => { const was = timeline.snapshot().following; timeline.follow(value); if (value && !was) { acknowledgeAfterPaint(); void read('sync') } }
    const loop = startSocialRefresh(async () => { await read('sync') }, { interval: 15000 })
    return () => { stopped = true; controller?.abort(); receipt?.abort(); cancelAnimationFrame(frame); loop.stop(); action.current = async () => undefined; followRef.current = () => {} }
  // key includes the complete immutable scope and session identity.
  }, [key])
  return { ...status, older: useCallback(() => action.current('older'), []), latest: useCallback(() => action.current('latest'), []),
    newer: useCallback(() => action.current('newer'), []), navigate: useCallback((target: Record<string, string>) => action.current('around', target), []),
    refresh: useCallback(() => action.current('sync'), []), follow: useCallback((value: boolean) => followRef.current(value), []) }
}
