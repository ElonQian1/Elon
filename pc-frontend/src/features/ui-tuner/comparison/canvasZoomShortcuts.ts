export type CanvasZoomCommand = 'zoom-in' | 'zoom-out' | 'actual-size'

interface CanvasZoomKeyboardInput {
  altKey: boolean
  ctrlKey: boolean
  key: string
  metaKey: boolean
}

export function canvasZoomCommand(input: CanvasZoomKeyboardInput): CanvasZoomCommand | null {
  if (input.altKey || (!input.ctrlKey && !input.metaKey)) return null
  if (input.key === '+' || input.key === '=' || input.key === 'Add') return 'zoom-in'
  if (input.key === '-' || input.key === '_' || input.key === 'Subtract') return 'zoom-out'
  if (input.key === '0') return 'actual-size'
  return null
}
