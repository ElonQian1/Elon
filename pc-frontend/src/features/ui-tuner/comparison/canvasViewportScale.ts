import type { PixelSize } from './types'

const MIN_SCALE = 0.08
const MAX_SCALE = 2

export type CanvasFitMode = 'stage' | 'width'

export function normalizeCanvasScale(value: number) {
  if (!Number.isFinite(value)) return 1
  return Math.round(Math.min(Math.max(value, MIN_SCALE), MAX_SCALE) * 100) / 100
}

export function calculateCanvasFitScale(
  viewport: PixelSize,
  canvas: PixelSize,
  mode: CanvasFitMode,
) {
  if (viewport.width <= 0 || viewport.height <= 0 || canvas.width <= 0 || canvas.height <= 0) {
    return null
  }
  const widthScale = viewport.width / canvas.width
  const scale = mode === 'width'
    ? Math.min(widthScale, 1)
    : Math.min(widthScale, viewport.height / canvas.height, 1)
  return normalizeCanvasScale(scale)
}
