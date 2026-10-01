import { useEffect, useRef, type RefObject } from 'react'

/** Scroll intent is scoped to this feed; programmatic restoration cannot drain history. */
export function useHistoryPagination(feed: RefObject<HTMLElement>, key: string,
  options: { hasOlder: boolean; loading: boolean; load: () => void }) {
  const state = useRef(options); state.current = options
  const resume = useRef<() => void>(() => {})
  useEffect(() => {
    const node = feed.current
    if (!node) return
    let frame = 0, pending = false, requested = false, top = node.scrollTop, intent = 0, touchY = 0
    function load() {
      if (!pending || requested || !state.current.hasOlder || node!.scrollTop > 96 || document.hidden) return
      if (state.current.loading) return
      pending = false; intent = 0; requested = true; state.current.load()
    }
    function schedule() { if (requested) return; intent = Date.now() + 800; pending = true; cancelAnimationFrame(frame); frame = requestAnimationFrame(() => { frame = requestAnimationFrame(load) }) }
    const wheel = (event: WheelEvent) => { if (event.deltaY < 0) schedule(); else pending = false }
    const touchStart = (event: TouchEvent) => { touchY = event.touches[0]?.clientY || 0 }
    const touchMove = (event: TouchEvent) => { if ((event.touches[0]?.clientY || 0) > touchY + 24) schedule() }
    const keyboard = (event: KeyboardEvent) => { if (['ArrowUp', 'PageUp', 'Home'].includes(event.key)) schedule() }
    const scroll = () => { const next = node.scrollTop; if (next < top && Date.now() < intent && !requested) { pending = true; load() }; top = next; if (next > 96) pending = false }
    resume.current = () => { if (!state.current.loading) { requested = false; load() } }
    node.addEventListener('scroll', scroll, { passive: true }); node.addEventListener('wheel', wheel, { passive: true })
    node.addEventListener('touchstart', touchStart, { passive: true }); node.addEventListener('touchmove', touchMove, { passive: true }); node.addEventListener('keydown', keyboard)
    return () => {
      cancelAnimationFrame(frame); resume.current = () => {}
      node.removeEventListener('scroll', scroll); node.removeEventListener('wheel', wheel)
      node.removeEventListener('touchstart', touchStart); node.removeEventListener('touchmove', touchMove); node.removeEventListener('keydown', keyboard)
    }
  }, [feed, key])
  useEffect(() => { resume.current() }, [options.loading, options.hasOlder])
}
