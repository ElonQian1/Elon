import { flattenSourceTree } from './sourcePreviewTree'
import {
  normalizePwaExplicitStyleBinding,
  type PwaElementIdentity,
  type PwaExplicitStyleBinding,
  type PwaSourceBinding,
  type PwaSourceCandidate,
} from './pwaDesignDraft'
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
  return rankPwaSourceNodes(root, identity)[0]?.node ?? null
}

export function pwaSourceBinding(
  identity: PwaElementIdentity,
  root: SourcePreviewNode | null,
  explicitStyleBinding?: PwaExplicitStyleBinding,
): PwaSourceBinding {
  const pwaStyle = normalizePwaExplicitStyleBinding(explicitStyleBinding)
  const pwaCandidates: PwaSourceCandidate[] = identity.key.startsWith('selector-evidence:') ? [] : [{
    platform: 'pwa',
    stableKey: identity.key,
    file: pwaStyle?.sourceFile,
    symbol: identity.sourceSymbol || undefined,
    componentPath: identity.componentPath || undefined,
    resourceId: identity.resourceId || identity.id || undefined,
    confidence: identity.confidenceScore,
    reason: pwaStyle ? 'PWA 节点显式提供可校验样式源码绑定' : '真实 DOM 稳定身份，只能用于 PWA 源码符号反查',
  }]
  const androidCandidates = root ? rankPwaSourceNodes(root, identity).slice(0, 3).map(({ node, score }) => ({
    platform: 'android' as const,
    stableKey: node.key,
    file: node.source?.layoutFile,
    symbol: node.name,
    resourceId: node.resourceId,
    confidence: Math.min(1, score / 120),
    reason: score >= 100 ? '稳定 resourceId/语义精确匹配' : '节点名称、文字或组件标识候选匹配',
  })) : []
  const androidBest = androidCandidates[0]
  const needsBinding = !pwaStyle || !androidBest?.file
  const bestScore = Math.min(identity.confidenceScore, androidBest?.confidence ?? 0)
  return {
    status: needsBinding ? (pwaCandidates.length || androidCandidates.length ? 'CANDIDATE' : 'NEEDS_AI') : 'BOUND',
    bindingConfidence: bestScore >= .85 ? 'high' : bestScore >= .6 ? 'medium' : 'low',
    needsBinding,
    pwaCandidates,
    androidCandidates,
    pwaStyle: pwaStyle ?? undefined,
  }
}

function rankPwaSourceNodes(root: SourcePreviewNode, identity: PwaIdentity) {
  return flattenSourceTree(root)
    .map((node) => ({ node, score: scoreNode(node, identity) }))
    .filter((candidate) => candidate.score >= 65)
    .sort((left, right) => right.score - left.score)
}
