import { commitSourcePreview } from './sourcePreviewApi'
import { findSourceNode } from './sourcePreviewTree'
import type { PwaDesignDraft, PwaStyleProperty } from './pwaDesignDraft'
import type { SourcePreviewNode } from './types'

export interface PwaDesignWritebackPlan {
  strategy: 'DETERMINISTIC_THEN_CODEX' | 'CODEX_REQUIRED'
  targets: {
    pwa: 'CODEX_REQUIRED'
    android: 'DETERMINISTIC' | 'DETERMINISTIC_PARTIAL' | 'CODEX_REQUIRED'
  }
  deterministic: Array<{
    elementKey: string
    nodeKey: string
    layoutFile: string
    changes: Record<string, string>
  }>
  codexReasons: string[]
  requiresCodex: true
}

export interface PwaDeterministicWritebackResult {
  applied: number
  sourceRevision: string
  changedFiles: string[]
  error?: string
}

export function planPwaDesignWriteback(
  draft: PwaDesignDraft | null,
  root: SourcePreviewNode | null,
): PwaDesignWritebackPlan {
  const deterministic: PwaDesignWritebackPlan['deterministic'] = []
  const codexReasons = new Set<string>(['PWA 源码候选需要由现有 Codex handoff 完成 TSX/CSS 来源绑定与写回'])
  for (const [elementKey, element] of Object.entries(draft?.elements ?? {})) {
    const candidate = element.binding.androidCandidates.find((item) => item.file)
    const node = candidate && root ? findSourceNode(root, candidate.stableKey) : null
    if (!candidate?.file || !node) {
      codexReasons.add(`${elementKey} 尚未绑定 Android 源码节点`)
      continue
    }
    const changes: Record<string, string> = {}
    for (const [property, value] of Object.entries(element.styleDiff)) {
      const translated = deterministicAndroidValue(property as PwaStyleProperty, value)
      if (translated) changes[translated.property] = translated.value
      else codexReasons.add(`${elementKey} 的 ${property} 需要 Codex 翻译`)
    }
    if (Object.keys(changes).length) {
      deterministic.push({ elementKey, nodeKey: node.key, layoutFile: node.source.layoutFile, changes })
    }
  }
  const changedCount = Object.keys(draft?.elements ?? {}).length
  const android = deterministic.length === 0
    ? 'CODEX_REQUIRED'
    : deterministic.length === changedCount && codexReasons.size === 1
      ? 'DETERMINISTIC'
      : 'DETERMINISTIC_PARTIAL'
  return {
    strategy: deterministic.length ? 'DETERMINISTIC_THEN_CODEX' : 'CODEX_REQUIRED',
    targets: { pwa: 'CODEX_REQUIRED', android },
    deterministic,
    codexReasons: [...codexReasons],
    requiresCodex: true,
  }
}

export async function applyDeterministicAndroidWriteback(input: {
  draft: PwaDesignDraft
  root: SourcePreviewNode | null
  projectRoot: string
  sourceRevision: string
}): Promise<PwaDeterministicWritebackResult> {
  const plan = planPwaDesignWriteback(input.draft, input.root)
  let sourceRevision = input.sourceRevision
  let applied = 0
  const changedFiles = new Set<string>()
  const actions = plan.deterministic
    .map((action) => ({ ...action, node: findSourceNode(input.root, action.nodeKey) }))
    .filter((action): action is typeof action & { node: SourcePreviewNode } => Boolean(action.node))
    .sort((left, right) => right.node.source.startTagStart - left.node.source.startTagStart)
  for (const action of actions) {
    const response = await commitSourcePreview({
      projectRoot: input.projectRoot,
      layoutFile: action.layoutFile,
      sourceRevision,
      nodeKey: action.nodeKey,
      startTagStart: action.node.source.startTagStart,
      startTagEnd: action.node.source.startTagEnd,
      changes: action.changes,
    })
    sourceRevision = response.sourceRevision
    changedFiles.add(action.layoutFile)
    applied += 1
  }
  return { applied, sourceRevision, changedFiles: [...changedFiles] }
}

function deterministicAndroidValue(property: PwaStyleProperty, value: string) {
  const propertyMap: Partial<Record<PwaStyleProperty, string>> = {
    width: 'width', height: 'height', paddingLeft: 'paddingStart', paddingTop: 'paddingTop',
    paddingRight: 'paddingEnd', paddingBottom: 'paddingBottom', marginLeft: 'marginStart',
    marginTop: 'marginTop', marginRight: 'marginEnd', marginBottom: 'marginBottom',
    borderRadius: 'borderRadius', fontSize: 'fontSize', color: 'textColor',
    backgroundColor: 'background', opacity: 'opacity',
  }
  const target = propertyMap[property]
  if (!target) return null
  const normalized = String(value).trim()
  if (property === 'width' || property === 'height') {
    if (normalized === 'auto') return { property: target, value: 'wrap_content' }
    if (normalized === '100%') return { property: target, value: 'match_parent' }
  }
  if (['color', 'backgroundColor', 'opacity'].includes(property)) {
    return normalized ? { property: target, value: normalized } : null
  }
  const numeric = normalized.match(/^(-?\d+(?:\.\d+)?)(?:px|dp|sp)?$/)
  if (!numeric) return null
  const unit = property === 'fontSize' ? 'sp' : property === 'opacity' ? '' : 'dp'
  return { property: target, value: `${numeric[1]}${unit}` }
}
