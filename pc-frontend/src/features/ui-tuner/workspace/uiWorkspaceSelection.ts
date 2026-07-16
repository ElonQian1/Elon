import type { SourcePreviewNode } from '../source-preview/types'
import type { UiTunerElement } from '../types'

export interface UiWorkspaceSelectionHint {
  resourceId?: string
  text?: string
  name?: string
  sourceFile?: string
  className?: string
}

function cleanValue(value?: string) {
  return value?.trim().toLocaleLowerCase() ?? ''
}

function resourceName(value?: string) {
  const cleaned = cleanValue(value)
  const slash = cleaned.lastIndexOf('/')
  return slash >= 0 ? cleaned.slice(slash + 1) : cleaned
}

function sameFile(left?: string, right?: string) {
  const a = cleanValue(left).replace(/\\/g, '/')
  const b = cleanValue(right).replace(/\\/g, '/')
  return Boolean(a && b && (a === b || a.endsWith(`/${b}`) || b.endsWith(`/${a}`)))
}

export function evidenceSelectionHint(element: UiTunerElement | null): UiWorkspaceSelectionHint | null {
  if (!element) return null
  return {
    resourceId: element.runtime?.resourceId,
    text: element.text,
    name: element.name,
    sourceFile: element.source?.file,
    className: element.runtime?.className,
  }
}

export function sourceSelectionHint(node: SourcePreviewNode): UiWorkspaceSelectionHint {
  return {
    resourceId: node.resourceId,
    text: node.style.text,
    name: node.name,
    sourceFile: node.source.layoutFile,
    className: node.tag,
  }
}

function hintScore(candidate: UiWorkspaceSelectionHint, hint: UiWorkspaceSelectionHint) {
  let score = 0
  const resource = resourceName(candidate.resourceId)
  if (resource && resource === resourceName(hint.resourceId)) score += 100
  if (sameFile(candidate.sourceFile, hint.sourceFile)) score += 40
  const text = cleanValue(candidate.text)
  if (text && text === cleanValue(hint.text)) score += 30
  const name = cleanValue(candidate.name)
  if (name && name === cleanValue(hint.name)) score += 20
  const className = cleanValue(candidate.className)
  if (className && className === cleanValue(hint.className)) score += 10
  return score
}

export function findSourceSelection(root: SourcePreviewNode | null, hint: UiWorkspaceSelectionHint | null) {
  if (!root || !hint) return null
  let bestKey: string | null = null
  let bestScore = 0
  const visit = (node: SourcePreviewNode) => {
    const score = hintScore(sourceSelectionHint(node), hint)
    if (score > bestScore) {
      bestKey = node.key
      bestScore = score
    }
    node.children.forEach(visit)
  }
  visit(root)
  return bestKey
}

export function findEvidenceSelection(elements: UiTunerElement[], hint: UiWorkspaceSelectionHint) {
  let best: { id: string; score: number } | null = null
  for (const element of elements) {
    const score = hintScore(evidenceSelectionHint(element) ?? {}, hint)
    if (score > 0 && (!best || score > best.score)) best = { id: element.id, score }
  }
  return best?.id ?? null
}

export function getSelectedId(elements: UiTunerElement[], preferredId: string | null) {
  if (preferredId && elements.some((element) => element.id === preferredId)) return preferredId
  return elements[0]?.id ?? null
}
