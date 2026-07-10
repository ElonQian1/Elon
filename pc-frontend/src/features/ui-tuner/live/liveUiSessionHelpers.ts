import type { UiTunerElement } from '../types'
import type { LiveUiNode } from './liveUiApi'

export function matchLiveNode(
  selected: UiTunerElement | null,
  nodes: LiveUiNode[],
): LiveUiNode | null {
  if (!selected?.runtime) return null
  const resourceId = comparableResourceId(selected.runtime.resourceId)
  if (resourceId) {
    const exact = nodes.find((node) => comparableResourceId(node.resourceId) === resourceId)
    if (exact) return exact
  }
  const original = selected.runtime.originalBounds
  let best: { node: LiveUiNode; score: number } | null = null
  for (const node of nodes) {
    if (!node.geometry.visible) continue
    const score = overlapScore(original, node.geometry.boundsInDisplayPx)
    if (score > (best?.score ?? 0)) best = { node, score }
  }
  return best && best.score >= 0.45 ? best.node : null
}

export function mergeEffectiveValues(
  node: LiveUiNode,
  values: Record<string, LiveUiNode['properties'][string]['effective']>,
): LiveUiNode {
  const properties = { ...node.properties }
  for (const [name, value] of Object.entries(values)) {
    if (!value || !properties[name]) continue
    properties[name] = { ...properties[name], effective: value }
  }
  return { ...node, properties }
}

export function messageOf(error: unknown, fallback: string) {
  return error instanceof Error && error.message.trim() ? error.message : fallback
}

function comparableResourceId(value?: string) {
  return value?.trim().replace(/^.*:id\//, '').replace(/^.*\/id\//, '') || ''
}

function overlapScore(
  left: NonNullable<UiTunerElement['runtime']>['originalBounds'],
  right: LiveUiNode['geometry']['boundsInDisplayPx'],
) {
  const intersectionWidth = Math.max(0, Math.min(left.right, right.right) - Math.max(left.left, right.left))
  const intersectionHeight = Math.max(0, Math.min(left.bottom, right.bottom) - Math.max(left.top, right.top))
  const intersection = intersectionWidth * intersectionHeight
  const leftArea = Math.max(1, left.width * left.height)
  const rightArea = Math.max(1, right.width * right.height)
  return intersection / Math.max(leftArea, rightArea)
}
