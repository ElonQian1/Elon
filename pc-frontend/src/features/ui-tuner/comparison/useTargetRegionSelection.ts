import { useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import { clientPointToNaturalPoint, hasUsableArea, rectFromPoints } from './comparisonGeometry'
import type { PixelRect, PixelSize } from './types'

export function useTargetRegionSelection(
  naturalSize: PixelSize,
  onCommit: (rect: PixelRect) => void,
) {
  const startRef = useRef<{ x: number; y: number } | null>(null)
  const [draftRect, setDraftRect] = useState<PixelRect | null>(null)

  const pointFor = (event: ReactPointerEvent<HTMLElement>) => clientPointToNaturalPoint(
    { x: event.clientX, y: event.clientY },
    event.currentTarget.getBoundingClientRect(),
    naturalSize,
  )

  const onPointerDown = (event: ReactPointerEvent<HTMLElement>) => {
    if (event.button !== 0) return
    event.preventDefault()
    event.currentTarget.setPointerCapture(event.pointerId)
    const point = pointFor(event)
    startRef.current = point
    setDraftRect(rectFromPoints(point, point, naturalSize))
  }

  const onPointerMove = (event: ReactPointerEvent<HTMLElement>) => {
    const start = startRef.current
    if (!start) return
    event.preventDefault()
    setDraftRect(rectFromPoints(start, pointFor(event), naturalSize))
  }

  const finish = (event: ReactPointerEvent<HTMLElement>) => {
    const start = startRef.current
    if (!start) return
    const rect = rectFromPoints(start, pointFor(event), naturalSize)
    startRef.current = null
    setDraftRect(null)
    if (hasUsableArea(rect)) onCommit(rect)
  }

  const cancel = () => {
    startRef.current = null
    setDraftRect(null)
  }

  return {
    draftRect,
    handlers: {
      onPointerDown,
      onPointerMove,
      onPointerUp: finish,
      onPointerCancel: cancel,
    },
  }
}
