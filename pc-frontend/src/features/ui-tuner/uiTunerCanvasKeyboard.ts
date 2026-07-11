import type { KeyboardEvent } from 'react'
import type { UiTunerDocument, UiTunerElement } from './types'
import { clamp } from './uiTunerGeometry'

export function handleCanvasArrowKey(
  event: KeyboardEvent<HTMLDivElement>,
  selected: UiTunerElement | null,
  canvas: UiTunerDocument['canvas'],
  updateElement: (id: string, patch: Partial<UiTunerElement>) => void,
) {
  if (!selected) return
  const step = event.shiftKey ? 8 : 1
  const bounds = {
    ArrowLeft: { x: clamp(selected.x - step, 0, canvas.width - selected.width) },
    ArrowRight: { x: clamp(selected.x + step, 0, canvas.width - selected.width) },
    ArrowUp: { y: clamp(selected.y - step, 0, canvas.height - selected.height) },
    ArrowDown: { y: clamp(selected.y + step, 0, canvas.height - selected.height) },
  }[event.key]
  if (!bounds) return
  event.preventDefault()
  updateElement(selected.id, bounds)
}
