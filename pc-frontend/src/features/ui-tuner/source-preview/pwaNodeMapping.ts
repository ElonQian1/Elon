import { flattenSourceTree } from './sourcePreviewTree'
import type { SourcePreviewNode } from './types'

export interface PwaIdentity {
  key: string
  uiNode: string
  id: string
  ariaLabel: string
  role: string
  text: string
  tag: string
  classNames: string[]
}

function normalize(value: string | undefined): string {
  return String(value || '')
    .replace(/^@\+?id\//, '')
    .replace(/[^a-z0-9\u4e00-\u9fff]+/gi, '')
    .toLowerCase()
}

function isStableAffixMatch(left: string, right: string): boolean {
  return Math.min(left.length, right.length) >= 8
    && (left.endsWith(right) || right.endsWith(left))
}

function scoreNode(node: SourcePreviewNode, identity: PwaIdentity): number {
  const resourceId = normalize(node.resourceId)
  const name = normalize(node.name)
  const text = normalize(node.style.text)
  const description = normalize(node.style.contentDescription)
  const identities = [identity.uiNode, identity.id, identity.ariaLabel, identity.key].map(normalize).filter(Boolean)
  let score = 0
  if (resourceId && identities.includes(resourceId)) score = Math.max(score, 120)
  if (name && identities.includes(name)) score = Math.max(score, 105)
  if (resourceId && identities.some((value) => isStableAffixMatch(resourceId, value))) score = Math.max(score, 92)
  if (name && identities.some((value) => isStableAffixMatch(name, value))) score = Math.max(score, 88)
  if (description && normalize(identity.ariaLabel) === description) score = Math.max(score, 100)
  if (text && normalize(identity.text) === text) score = Math.max(score, 95)
  if (text.length >= 2 && normalize(identity.text).includes(text)) score = Math.max(score, 70)
  if (identity.classNames.some((className) => normalize(className) === resourceId || normalize(className) === name)) score = Math.max(score, 65)
  return score
}

export function matchPwaSourceNode(root: SourcePreviewNode, identity: PwaIdentity): SourcePreviewNode | null {
  const ranked = flattenSourceTree(root)
    .map((node) => ({ node, score: scoreNode(node, identity) }))
    .sort((left, right) => right.score - left.score)
  return ranked[0]?.score >= 65 ? ranked[0].node : null
}
