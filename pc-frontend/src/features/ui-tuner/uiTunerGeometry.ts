import type { UiTunerDocument, UiTunerElement, UiTunerElementKind } from './types'

export interface MetricItem {
  label: string
  value: string
}

export function clamp(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) return min
  if (max < min) return min
  return Math.min(Math.max(Math.round(value), min), max)
}

export function touch(document: UiTunerDocument): UiTunerDocument {
  return { ...document, updatedAt: new Date().toISOString() }
}

export function kindLabel(kind: UiTunerElementKind) {
  if (kind === 'text') return '文字'
  if (kind === 'button') return '按钮'
  if (kind === 'media') return '图片'
  return '卡片'
}

export function getMetrics(
  selected: UiTunerElement,
  elements: UiTunerElement[],
  canvas: UiTunerDocument['canvas'],
): MetricItem[] {
  const above = elements
    .filter((element) => element.id !== selected.id && element.y + element.height <= selected.y)
    .map((element) => selected.y - (element.y + element.height))
    .sort((a, b) => a - b)[0]
  const left = elements
    .filter((element) => element.id !== selected.id && element.x + element.width <= selected.x)
    .map((element) => selected.x - (element.x + element.width))
    .sort((a, b) => a - b)[0]

  return [
    { label: '距画布左', value: `${selected.x}px` },
    { label: '距画布上', value: `${selected.y}px` },
    { label: '距右边', value: `${canvas.width - selected.x - selected.width}px` },
    { label: '距底部', value: `${canvas.height - selected.y - selected.height}px` },
    { label: '上方最近', value: above === undefined ? '-' : `${above}px` },
    { label: '左侧最近', value: left === undefined ? '-' : `${left}px` },
  ]
}
