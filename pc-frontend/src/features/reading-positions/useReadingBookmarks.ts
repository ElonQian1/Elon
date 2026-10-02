import { useEffect, useRef, type RefObject } from 'react'
import { api } from '../../api/client'
import { useAuthStore } from '../../store/auth'
import type { TimelineMessage, TimelineScope, ReadingTarget } from '../message-timeline/timeline'
import type { useMessageTimeline } from '../message-timeline/useMessageTimeline'
import '../../../../server/src/assets/reading_positions.js'
import '../../../../server/src/assets/reading_bookmarks_ui.js'
import '../../../../server/src/assets/reading_bookmarks.css'

interface ReadingUI {
  add(message: TimelineMessage): void
  latest(): Promise<void>
  historical(): boolean
  close(): void
}
declare global {
  var ElonReadingBookmarksUI: { mount(options: {
    owner: string; scope: TimelineScope; list: HTMLElement; toolbar: HTMLElement
    current(): boolean; messages(): TimelineMessage[]; filtered(): boolean
    request(method: string, path: string, body?: unknown): Promise<unknown>
    navigate(query: Record<string, string>): Promise<ReadingTarget | boolean | undefined>
    latest(): Promise<boolean | ReadingTarget | undefined>; history(value: boolean): void
    newer(): Promise<unknown>
  }): ReadingUI }
}
/** Shared reading protocol and accessible controls; the React conversation owns their lifetime. */
export function useReadingBookmarks(owner: string, scope: TimelineScope, list: RefObject<HTMLDivElement>, toolbar: RefObject<HTMLDivElement>,
  messages: TimelineMessage[], timeline: ReturnType<typeof useMessageTimeline>, history: (value: boolean) => void, filtered: boolean) {
  const instance = useRef<ReadingUI | null>(null), data = useRef({ messages, timeline, history, filtered })
  data.current = { messages, timeline, history, filtered }
  const token = useAuthStore(s => s.token), key = JSON.stringify([owner, token, scope])
  useEffect(() => {
    if (!list.current || !toolbar.current) return
    const ui = ElonReadingBookmarksUI.mount({ owner, scope, list: list.current, toolbar: toolbar.current,
      current: () => useAuthStore.getState().token === token && useAuthStore.getState().user?.id === owner,
      messages: () => data.current.messages, filtered: () => data.current.filtered,
      request: (method, path, body) => method === 'POST' ? api.post(path, body) : api.get(path),
      navigate: query => data.current.timeline.navigate(query), latest: () => data.current.timeline.latest(), history: value => data.current.history(value),
      newer: () => data.current.timeline.hasNewer ? data.current.timeline.newer() : Promise.resolve() })
    instance.current = ui
    return () => { ui.close(); instance.current = null }
  }, [key])
  return instance
}
