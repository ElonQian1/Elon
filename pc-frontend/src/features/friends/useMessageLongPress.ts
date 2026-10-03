import { useEffect, useRef, type MouseEvent, type PointerEvent } from 'react'

export type MessageLongPress = (target: HTMLElement, x: number, y: number) => void

/** Preserve tap and scrolling; only a stationary touch/pen opens the message menu. */
export function useMessageLongPress(identity: string, open?: MessageLongPress) {
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  const start = useRef<{ x: number; y: number; id: number } | null>(null)
  const suppressClick = useRef(false)
  const current = useRef(open); current.current = open
  function cancel() { clearTimeout(timer.current); start.current = null }
  useEffect(() => { suppressClick.current = false; return cancel }, [identity, !!open])
  return {
    onPointerDown: (event: PointerEvent<HTMLElement>) => {
      cancel(); suppressClick.current = false
      if (!current.current || event.pointerType === 'mouse' || !event.isPrimary || event.button !== 0) return
      const target = event.currentTarget, x = event.clientX, y = event.clientY
      start.current = { x, y, id: event.pointerId }
      timer.current = setTimeout(() => {
        if (!target.isConnected || !start.current) return
        suppressClick.current = true; start.current = null; current.current?.(target, x, y)
      }, 550)
    },
    onPointerMove: (event: PointerEvent<HTMLElement>) => {
      const point = start.current
      if (point && (point.id !== event.pointerId || Math.hypot(event.clientX - point.x, event.clientY - point.y) > 8)) cancel()
    },
    onPointerUp: cancel, onPointerCancel: cancel, onPointerLeave: cancel,
    onContextMenu: cancel,
    onClickCapture: (event: MouseEvent<HTMLElement>) => {
      if (suppressClick.current) { event.preventDefault(); event.stopPropagation(); suppressClick.current = false }
    },
  }
}
