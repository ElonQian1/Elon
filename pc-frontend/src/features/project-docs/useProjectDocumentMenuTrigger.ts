import { useCallback, useEffect, useRef, type KeyboardEvent as ReactKeyboardEvent, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent } from 'react'

export interface ProjectDocumentMenuPoint {
  x: number
  y: number
}

export function useProjectDocumentMenuTrigger<T>(
  onOpen: (target: T, point: ProjectDocumentMenuPoint) => void,
) {
  const timerRef = useRef<number | null>(null)
  const startPointRef = useRef<ProjectDocumentMenuPoint | null>(null)

  const cancelLongPress = useCallback(() => {
    if (timerRef.current !== null) window.clearTimeout(timerRef.current)
    timerRef.current = null
    startPointRef.current = null
  }, [])

  useEffect(() => cancelLongPress, [cancelLongPress])

  return useCallback((target: T) => ({
    onContextMenu: (event: ReactMouseEvent<HTMLElement>) => {
      event.preventDefault()
      event.stopPropagation()
      onOpen(target, { x: event.clientX, y: event.clientY })
    },
    onKeyDown: (event: ReactKeyboardEvent<HTMLElement>) => {
      if (event.key !== 'ContextMenu' && !(event.shiftKey && event.key === 'F10')) return
      event.preventDefault()
      const rect = event.currentTarget.getBoundingClientRect()
      onOpen(target, { x: rect.left + 24, y: rect.top + Math.min(rect.height, 36) })
    },
    onPointerDown: (event: ReactPointerEvent<HTMLElement>) => {
      if (event.pointerType !== 'touch') return
      cancelLongPress()
      const point = { x: event.clientX, y: event.clientY }
      startPointRef.current = point
      timerRef.current = window.setTimeout(() => {
        onOpen(target, point)
        cancelLongPress()
      }, 550)
    },
    onPointerMove: (event: ReactPointerEvent<HTMLElement>) => {
      const start = startPointRef.current
      if (start && Math.hypot(event.clientX - start.x, event.clientY - start.y) > 8) cancelLongPress()
    },
    onPointerUp: cancelLongPress,
    onPointerCancel: cancelLongPress,
  }), [cancelLongPress, onOpen])
}

export function menuPointForButton(button: HTMLElement): ProjectDocumentMenuPoint {
  const rect = button.getBoundingClientRect()
  return { x: Math.min(rect.right, window.innerWidth - 8), y: rect.bottom + 4 }
}
