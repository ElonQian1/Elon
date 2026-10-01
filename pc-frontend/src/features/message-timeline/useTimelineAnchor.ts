import { useLayoutEffect, useRef, type RefObject } from 'react'

export function useTimelineAnchor(feed: RefObject<HTMLElement>, revision: unknown) {
  const anchor = useRef<{ id: string; offset: number } | null>(null)
  useLayoutEffect(() => {
    const node = feed.current, saved = anchor.current
    if (!node || !saved) return
    const element = node.querySelector<HTMLElement>(`[data-message-id="${CSS.escape(saved.id)}"]`)
    if (element) node.scrollTop += element.getBoundingClientRect().top - node.getBoundingClientRect().top - saved.offset
    anchor.current = null
  }, [feed, revision])
  return () => {
    const node = feed.current
    if (!node) return
    const top = node.getBoundingClientRect().top
    const visible = Array.from(node.querySelectorAll<HTMLElement>('[data-message-id]')).find(element => element.getBoundingClientRect().bottom > top)
    if (visible?.dataset.messageId) anchor.current = { id: visible.dataset.messageId, offset: visible.getBoundingClientRect().top - top }
  }
}
