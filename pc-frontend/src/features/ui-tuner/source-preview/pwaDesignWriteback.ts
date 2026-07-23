import { commitPwaStylePreview, commitSourcePreview } from './sourcePreviewApi'
import { findSourceNode } from './sourcePreviewTree'
import {
  normalizePwaExplicitStyleBinding,
  safePwaSourceFile,
  type PwaDesignDraft,
  type PwaExplicitStyleBinding,
  type PwaStyleProperty,
} from './pwaDesignDraft'
import type { SourcePreviewNode } from './types'

type WritebackTargetStatus = 'DETERMINISTIC' | 'DETERMINISTIC_PARTIAL' | 'CODEX_REQUIRED'

export interface PwaAndroidWritebackAction {
  elementKey: string
  nodeKey: string
  layoutFile: string
  startTagStart: number
  startTagEnd: number
  changes: Record<string, string>
  properties: Partial<Record<PwaStyleProperty, string>>
}

export interface PwaSourceWritebackAction {
  elementKey: string
  binding: PwaExplicitStyleBinding
  changes: Record<string, string>
  properties: Partial<Record<PwaStyleProperty, string>>
}

export interface PwaCodexFallbackChange {
  platform: 'pwa' | 'android'
  elementKey: string
  property: PwaStyleProperty
  before: string
  after: string
  sourceFile?: string
  nodeKey?: string
  reason: string
}

export interface PwaDesignWritebackPlan {
  strategy: 'DETERMINISTIC_THEN_CODEX' | 'CODEX_REQUIRED'
  targets: { pwa: WritebackTargetStatus; android: WritebackTargetStatus }
  deterministic: {
    pwa: PwaSourceWritebackAction[]
    android: PwaAndroidWritebackAction[]
  }
  codexChanges: PwaCodexFallbackChange[]
  codexReasons: string[]
  requiresCodex: boolean
}

export interface PwaCompletedWritebackAction {
  elementKey: string
  sourceFile: string
  sourceRevision: string
  properties: Partial<Record<PwaStyleProperty, string>>
}

export interface PwaDeterministicWritebackResult {
  applied: number
  sourceRevision: string
  changedFiles: string[]
  sourceHashes: Record<string, string>
  completed: PwaCompletedWritebackAction[]
  error?: string
  failedAction?: string
  stopped?: boolean
}

export interface PwaDeterministicPwaWritebackResult {
  applied: number
  changedFiles: string[]
  sourceRevisions: Record<string, string>
  sourceHashes: Record<string, string>
  completed: PwaCompletedWritebackAction[]
  error?: string
  failedAction?: string
  stopped?: boolean
}

export interface PwaCrossPlatformWritebackResult {
  android: PwaDeterministicWritebackResult
  pwa: PwaDeterministicPwaWritebackResult
}

export function planPwaDesignWriteback(
  draft: PwaDesignDraft | null,
  root: SourcePreviewNode | null,
): PwaDesignWritebackPlan {
  const androidActions = new Map<string, PwaAndroidWritebackAction>()
  const pwaActions = new Map<string, PwaSourceWritebackAction>()
  const codexChanges: PwaCodexFallbackChange[] = []
  const completed = { android: 0, pwa: 0 }

  for (const [elementKey, element] of Object.entries(draft?.elements ?? {})) {
    const androidCandidate = element.binding.androidCandidates.find((item) => item.file)
    const androidNode = androidCandidate && root ? findSourceNode(root, androidCandidate.stableKey) : null
    const pwaBinding = normalizePwaExplicitStyleBinding(element.binding.pwaStyle)
    for (const [propertyValue, afterValue] of Object.entries(element.styleDiff)) {
      const property = propertyValue as PwaStyleProperty
      const after = String(afterValue)
      const before = element.originalStyle.authored[property] ?? element.originalStyle.computed[property] ?? ''

      if (receiptMatches(element.writeback?.android?.[property], after)) completed.android += 1
      else {
        const translated = deterministicAndroidValue(property, after)
        if (!translated || !androidCandidate?.file || !androidNode
          || androidNode.source.layoutFile !== androidCandidate.file) {
          codexChanges.push({
            platform: 'android', elementKey, property, before, after,
            sourceFile: androidCandidate?.file,
            nodeKey: androidCandidate?.stableKey,
            reason: !androidNode || androidNode.source.layoutFile !== androidCandidate?.file
              ? '尚未绑定可校验的 Android 源码节点'
              : `Android 不支持确定性翻译 ${property}`,
          })
        } else {
          const actionKey = `${elementKey}\0${androidNode.key}\0${androidNode.source.layoutFile}`
          const action = androidActions.get(actionKey) ?? {
            elementKey,
            nodeKey: androidNode.key,
            layoutFile: androidNode.source.layoutFile,
            startTagStart: androidNode.source.startTagStart,
            startTagEnd: androidNode.source.startTagEnd,
            changes: {},
            properties: {},
          }
          action.changes[translated.property] = translated.value
          action.properties[property] = after
          androidActions.set(actionKey, action)
        }
      }

      if (receiptMatches(element.writeback?.pwa?.[property], after)) completed.pwa += 1
      else {
        const sourceProperty = pwaBinding?.propertyMap[property]
        if (!pwaBinding || !sourceProperty) {
          codexChanges.push({
            platform: 'pwa', elementKey, property, before, after,
            sourceFile: element.binding.pwaCandidates[0]?.file,
            reason: pwaBinding
              ? `显式 PWA 绑定未映射 ${property}`
              : 'PWA 节点未显式提供安全源码样式绑定',
          })
        } else {
          const actionKey = `${elementKey}\0${pwaBinding.sourceFile}\0${pwaBinding.range.start}`
          const action = pwaActions.get(actionKey) ?? {
            elementKey,
            binding: pwaBinding,
            changes: {},
            properties: {},
          }
          action.changes[sourceProperty] = after
          action.properties[property] = after
          pwaActions.set(actionKey, action)
        }
      }
    }
  }

  const deterministic = {
    pwa: [...pwaActions.values()].sort(comparePwaActions),
    android: [...androidActions.values()].sort(compareAndroidActions),
  }
  const androidCodex = codexChanges.filter((change) => change.platform === 'android').length
  const pwaCodex = codexChanges.filter((change) => change.platform === 'pwa').length
  const codexReasons = [...new Set(codexChanges.map((change) => `${change.elementKey} · ${change.platform} · ${change.reason}`))]
  return {
    strategy: deterministic.android.length || deterministic.pwa.length || completed.android || completed.pwa
      ? 'DETERMINISTIC_THEN_CODEX'
      : 'CODEX_REQUIRED',
    targets: {
      android: targetStatus(deterministic.android.length, androidCodex, completed.android),
      pwa: targetStatus(deterministic.pwa.length, pwaCodex, completed.pwa),
    },
    deterministic,
    codexChanges,
    codexReasons,
    requiresCodex: codexChanges.length > 0,
  }
}

export async function applyDeterministicAndroidWriteback(input: {
  draft: PwaDesignDraft
  root: SourcePreviewNode | null
  projectRoot: string
  sourceRevision: string
  commit?: typeof commitSourcePreview
}): Promise<PwaDeterministicWritebackResult> {
  const plan = planPwaDesignWriteback(input.draft, input.root)
  const commit = input.commit ?? commitSourcePreview
  let sourceRevision = input.sourceRevision
  const changedFiles = new Set<string>()
  const sourceHashes = new Map<string, string>()
  const completed: PwaCompletedWritebackAction[] = []
  for (const action of plan.deterministic.android) {
    try {
      const response = await commit({
        projectRoot: input.projectRoot,
        layoutFile: action.layoutFile,
        sourceRevision,
        nodeKey: action.nodeKey,
        startTagStart: action.startTagStart,
        startTagEnd: action.startTagEnd,
        changes: action.changes,
      })
      if (!response.ok || !response.sourceRevision) throw new Error('Android 写回端未返回新 sourceRevision')
      sourceRevision = response.sourceRevision
      for (const file of response.changedFiles ?? [action.layoutFile]) changedFiles.add(file)
      for (const [file, hash] of Object.entries(response.sourceHashes ?? {})) sourceHashes.set(file, hash)
      completed.push({
        elementKey: action.elementKey,
        sourceFile: action.layoutFile,
        sourceRevision,
        properties: action.properties,
      })
    } catch (error) {
      return {
        applied: completed.length,
        sourceRevision,
        changedFiles: [...changedFiles].sort(),
        sourceHashes: Object.fromEntries([...sourceHashes.entries()].sort()),
        completed,
        error: error instanceof Error ? error.message : 'Android 确定性写回失败',
        failedAction: `${action.layoutFile}#${action.nodeKey}`,
        stopped: true,
      }
    }
  }
  return {
    applied: completed.length,
    sourceRevision,
    changedFiles: [...changedFiles].sort(),
    sourceHashes: Object.fromEntries([...sourceHashes.entries()].sort()),
    completed,
  }
}

export async function applyDeterministicPwaWriteback(input: {
  draft: PwaDesignDraft
  root: SourcePreviewNode | null
  projectRoot: string
  commit?: typeof commitPwaStylePreview
}): Promise<PwaDeterministicPwaWritebackResult> {
  const plan = planPwaDesignWriteback(input.draft, input.root)
  const commit = input.commit ?? commitPwaStylePreview
  const revisions = new Map<string, string>()
  const baseRevisions = new Map<string, string>()
  const changedFiles = new Set<string>()
  const completed: PwaCompletedWritebackAction[] = []
  for (const action of plan.deterministic.pwa) {
    const sourceFile = safePwaSourceFile(action.binding.sourceFile)
    const previousBase = sourceFile ? baseRevisions.get(sourceFile) : undefined
    if (!sourceFile || (previousBase && previousBase !== action.binding.sourceRevision)) {
      return pwaFailure(completed, changedFiles, revisions, action, 'PWA 绑定路径或基线 revision 不一致')
    }
    baseRevisions.set(sourceFile, action.binding.sourceRevision)
    const sourceRevision = revisions.get(sourceFile) ?? action.binding.sourceRevision
    try {
      const response = await commit({
        projectRoot: input.projectRoot,
        binding: { ...action.binding, sourceFile, sourceRevision },
        sourceRevision,
        changes: action.changes,
      })
      if (!response.ok || !/^[a-f0-9]{64}$/i.test(response.sourceRevision)
        || !response.changedFiles.includes(sourceFile)) {
        throw new Error('PWA 写回端未确认目标文件和新 sourceRevision')
      }
      revisions.set(sourceFile, response.sourceRevision.toLowerCase())
      changedFiles.add(sourceFile)
      completed.push({
        elementKey: action.elementKey,
        sourceFile,
        sourceRevision: response.sourceRevision.toLowerCase(),
        properties: action.properties,
      })
    } catch (error) {
      return pwaFailure(
        completed, changedFiles, revisions, action,
        error instanceof Error ? error.message : 'PWA 确定性写回失败',
      )
    }
  }
  return {
    applied: completed.length,
    changedFiles: [...changedFiles].sort(),
    sourceRevisions: Object.fromEntries([...revisions.entries()].sort()),
    sourceHashes: Object.fromEntries([...revisions.entries()].sort()),
    completed,
  }
}

export function recordDeterministicWriteback(
  draft: PwaDesignDraft,
  result: PwaCrossPlatformWritebackResult,
): PwaDesignDraft {
  const completions = [
    ...result.android.completed.map((entry) => ({ platform: 'android' as const, entry })),
    ...result.pwa.completed.map((entry) => ({ platform: 'pwa' as const, entry })),
  ]
  if (!completions.length) return draft
  const elements = { ...draft.elements }
  const completedAt = new Date().toISOString()
  for (const { platform, entry } of completions) {
    const element = elements[entry.elementKey]
    if (!element) continue
    const platformReceipts = { ...(element.writeback?.[platform] ?? {}) }
    for (const [propertyValue, value] of Object.entries(entry.properties)) {
      const property = propertyValue as PwaStyleProperty
      platformReceipts[property] = {
        value: String(value),
        sourceFile: entry.sourceFile,
        sourceRevision: entry.sourceRevision,
        completedAt,
      }
    }
    elements[entry.elementKey] = {
      ...element,
      writeback: { ...element.writeback, [platform]: platformReceipts },
    }
  }
  for (const [elementKey, element] of Object.entries(elements)) {
    const binding = element.binding.pwaStyle
    const finalRevision = binding ? result.pwa.sourceRevisions[binding.sourceFile] : undefined
    if (!binding || !finalRevision || binding.sourceRevision === finalRevision) continue
    elements[elementKey] = {
      ...element,
      binding: { ...element.binding, pwaStyle: { ...binding, sourceRevision: finalRevision } },
    }
  }
  return { ...draft, elements, updatedAt: completedAt }
}

function pwaFailure(
  completed: PwaCompletedWritebackAction[],
  changedFiles: Set<string>,
  revisions: Map<string, string>,
  action: PwaSourceWritebackAction,
  error: string,
): PwaDeterministicPwaWritebackResult {
  return {
    applied: completed.length,
    changedFiles: [...changedFiles].sort(),
    sourceRevisions: Object.fromEntries([...revisions.entries()].sort()),
    sourceHashes: Object.fromEntries([...revisions.entries()].sort()),
    completed,
    error,
    failedAction: `${action.binding.sourceFile}#${action.binding.kind}:${action.binding.target}`,
    stopped: true,
  }
}

function receiptMatches(receipt: { value: string } | undefined, value: string): boolean {
  return receipt?.value === value
}

function targetStatus(deterministic: number, codex: number, completed: number): WritebackTargetStatus {
  if (codex && (deterministic || completed)) return 'DETERMINISTIC_PARTIAL'
  if (codex) return 'CODEX_REQUIRED'
  return deterministic || completed ? 'DETERMINISTIC' : 'CODEX_REQUIRED'
}

function compareAndroidActions(left: PwaAndroidWritebackAction, right: PwaAndroidWritebackAction): number {
  const file = left.layoutFile.localeCompare(right.layoutFile)
  if (file) return file
  return right.startTagStart - left.startTagStart || left.nodeKey.localeCompare(right.nodeKey)
}

function comparePwaActions(left: PwaSourceWritebackAction, right: PwaSourceWritebackAction): number {
  const file = left.binding.sourceFile.localeCompare(right.binding.sourceFile)
  if (file) return file
  const position = right.binding.range.start - left.binding.range.start
  return position || left.elementKey.localeCompare(right.elementKey)
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
  const unit = property === 'fontSize' ? 'sp' : 'dp'
  return { property: target, value: `${numeric[1]}${unit}` }
}
