import type { UiTunerCodexContextPack } from '../contextPack'
import {
  buildPwaDraftCliCompactHandoff,
  type PwaDesignDraft,
} from './pwaDesignDraft'
import type { PwaRuntimeCaptureArtifact } from './pwaVerificationModel'
import type { CrossPlatformWritebackReceipt } from './crossPlatformWritebackReceipt'
import type {
  PwaCrossPlatformWritebackResult,
  PwaDesignWritebackPlan,
} from './pwaDesignWriteback'
import type { PwaSelection } from './usePwaDesignSession'
import type { SourcePreviewNode } from './types'

export function buildPwaDesignContextPack(input: {
  draft: PwaDesignDraft
  root: SourcePreviewNode | null
  selection: PwaSelection | null
  plan: PwaDesignWritebackPlan
  deterministicResult: PwaCrossPlatformWritebackResult
  runtimeCapture?: PwaRuntimeCaptureArtifact
  writebackReceipt?: CrossPlatformWritebackReceipt
}): UiTunerCodexContextPack {
  const unresolvedKeys = new Set(input.plan.codexChanges.map((change) => change.elementKey))
  const selected = selectedUnresolvedElement(input.draft, input.selection, unresolvedKeys)
  const selectedKey = selected ? Object.entries(input.draft.elements).find(([, element]) => element === selected)?.[0] : undefined
  const selectedStyle = Object.fromEntries(input.plan.codexChanges
    .filter((change) => change.elementKey === selectedKey)
    .map((change) => [change.property, change.after]))
  const androidCandidate = selected?.binding.androidCandidates[0]
  const unresolvedFiles = compact(input.plan.codexChanges.map((change) => change.sourceFile))
  const adjustments = input.plan.codexChanges.map((change) => ({
    property: `${change.platform}.${change.property}`,
    before: change.before,
    after: change.after,
    sourceHint: change.reason,
  }))
  const visual = input.draft.visualReferences
  return {
    version: 4,
    kind: 'elon_ui_tuner_codex_context',
    generatedAt: new Date().toISOString(),
    screen: {
      canvasName: input.draft.pageSource.title || input.draft.route.path,
      width: input.draft.viewport.width,
      height: input.draft.viewport.height,
      sourceRoot: input.draft.project.workspaceIdentity,
      capturedAt: input.draft.updatedAt,
      screenshotPath: input.runtimeCapture?.path || visual.screenshot,
    },
    selectionScope: selected?.scope === 'route' ? 'screen' : selected?.scope ?? 'instance',
    selectedElement: selected ? {
      id: selected.identity.key,
      name: selected.identity.ariaLabel || selected.identity.text || selected.identity.id || selected.identity.tag,
      kind: selected.identity.tag,
      text: selected.identity.text,
      rect: input.selection?.rect ? {
        x: input.selection.rect.left, y: input.selection.rect.top,
        width: input.selection.rect.width, height: input.selection.rect.height,
      } : { x: 0, y: 0, width: 0, height: 0 },
      style: selectedStyle,
      metrics: [],
    } : null,
    runtimeBinding: {
      resourceId: androidCandidate?.resourceId,
      sourceFile: androidCandidate?.file || selected?.binding.pwaStyle?.sourceFile,
      sourceToken: androidCandidate?.symbol || selected?.binding.pwaStyle?.target,
      bindingConfidence: selected?.binding.bindingConfidence,
      bindingReason: selected?.binding.pwaStyle
        ? 'PWA 显式样式绑定已摘要；只把未映射属性交给 AI'
        : '需要 AI 为未解决节点建立 PWA 来源绑定',
      sourceCandidates: compactCandidates(selected),
    },
    liveRuntime: null,
    fitRun: null,
    selectionVisual: {
      available: Boolean(input.runtimeCapture?.path || visual.currentCrop || visual.screenshot),
      cropPath: input.runtimeCapture?.path || visual.currentCrop || visual.screenshot,
      contextPath: visual.visualDiff,
    },
    repeatedComponent: null,
    requestedAdjustments: adjustments,
    standardDraft: null,
    layerClarity: {
      visibleCount: unresolvedKeys.size,
      totalCount: unresolvedKeys.size,
      hiddenCount: 0, structuralCount: 0, duplicateCount: 0,
      sourceMappedCount: [...unresolvedKeys].filter((key) => Boolean(input.draft.elements[key]?.binding.pwaStyle)).length,
      selectedHiddenReasons: [],
    },
    codexContract: {
      readBeforeEdit: compact([input.runtimeCapture?.path, visual.targetCrop, visual.currentCrop, visual.visualDiff, ...unresolvedFiles]),
      writeTargets: unresolvedFiles,
      forbiddenShortcuts: [
        '不要读取整仓库或整棵 DOM',
        '不要通过全文搜索猜测 Runtime selector 对应的源码',
        '不要重复执行 deterministicSummary 中已完成的 APK/PWA 写回',
      ],
      acceptance: [
        '只处理 codexFallback.changes 中仍未绑定或无法确定性翻译的属性',
        '结构修改保持在本次 screenKey 和局部组件范围内',
        '不得覆盖 sourceRevision 冲突；冲突时返回明确阻塞证据',
      ],
    },
    closurePriorityIds: [],
    pwaDesign: {
      artifactVersion: input.draft.artifactVersion,
      sourceRevision: input.draft.project.sourceRevision,
      route: input.draft.route,
      viewport: input.draft.viewport,
      compactHandoff: buildPwaDraftCliCompactHandoff(input.draft),
      changes: input.plan.codexChanges,
      compactSourceBundle: compactSourceBundle(input.root, input.plan),
      bindingSummary: bindingSummary(input.draft, unresolvedKeys),
      codexFallback: {
        reasons: input.plan.codexReasons,
        changes: input.plan.codexChanges,
      },
      deterministicSummary: deterministicSummary(input.deterministicResult),
      visualReferences: { ...input.draft.visualReferences },
      runtimeCapture: input.runtimeCapture ? {
        path: input.runtimeCapture.path,
        sha256: input.runtimeCapture.sha256,
        manifestPath: input.runtimeCapture.manifestPath,
        width: input.runtimeCapture.width,
        height: input.runtimeCapture.height,
        mediaType: input.runtimeCapture.mediaType,
      } : undefined,
      writebackPlan: {
        targets: input.plan.targets,
        requiresCodex: input.plan.requiresCodex,
        codexReasons: input.plan.codexReasons,
      },
      deterministicResult: deterministicSummary(input.deterministicResult),
      machineReceipt: input.writebackReceipt ? {
        receiptId: input.writebackReceipt.receiptId,
        sourceRevisionBefore: input.writebackReceipt.sourceRevisionBefore,
        sourceRevision: input.writebackReceipt.sourceRevision,
        sourceHash: input.writebackReceipt.sourceHash,
        changedFiles: input.writebackReceipt.changedFiles,
        targetPlatforms: input.writebackReceipt.targetPlatforms,
        platformResults: input.writebackReceipt.platformResults,
      } : undefined,
      contextPolicy: {
        fullRepositoryIncluded: false,
        fullDomIncluded: false,
        screenshotsEmbeddedAsBase64: false,
      },
      capabilities: { PWA_CODE_GENERATION: true },
    },
  }
}

function selectedUnresolvedElement(
  draft: PwaDesignDraft,
  selection: PwaSelection | null,
  unresolvedKeys: Set<string>,
) {
  if (selection) {
    const match = Object.entries(draft.elements).find(([key, element]) => (
      unresolvedKeys.has(key)
      && (element.identity.key === selection.identity.key || element.identity.selector === selection.identity.selector)
    ))?.[1]
    if (match) return match
  }
  const firstKey = unresolvedKeys.values().next().value as string | undefined
  return firstKey ? draft.elements[firstKey] : undefined
}

function compactCandidates(element: PwaDesignDraft['elements'][string] | undefined) {
  if (!element) return []
  return [
    ...element.binding.pwaCandidates.slice(0, 2),
    ...element.binding.androidCandidates.slice(0, 2),
  ].map((candidate) => ({
    file: candidate.file,
    token: candidate.symbol,
    confidence: candidate.confidence,
    reason: candidate.reason,
    componentKey: candidate.stableKey,
    scope: element.scope,
  }))
}

function bindingSummary(draft: PwaDesignDraft, unresolvedKeys: Set<string>) {
  return [...unresolvedKeys].slice(0, 16).map((elementKey) => {
    const element = draft.elements[elementKey]
    const pwa = element?.binding.pwaStyle
    const android = element?.binding.androidCandidates[0]
    return {
      elementKey,
      pwa: pwa ? {
        sourceFile: pwa.sourceFile,
        sourceRevision: pwa.sourceRevision,
        kind: pwa.kind,
        target: pwa.target,
        mappedProperties: Object.keys(pwa.propertyMap),
      } : null,
      android: android ? { sourceFile: android.file, nodeKey: android.stableKey, resourceId: android.resourceId } : null,
    }
  })
}

function compactSourceBundle(root: SourcePreviewNode | null, plan: PwaDesignWritebackPlan) {
  if (!root) return []
  const targetKeys = new Set(plan.codexChanges
    .filter((change) => change.platform === 'android' && change.nodeKey)
    .map((change) => change.nodeKey as string))
  const bundle: Array<Record<string, unknown>> = []
  function visit(node: SourcePreviewNode, parent: SourcePreviewNode | null) {
    if (targetKeys.has(node.key)) {
      const relatives = [parent, node, ...(parent?.children ?? []).filter((child) => child.key !== node.key).slice(0, 3)]
      for (const relative of relatives) {
        if (!relative || bundle.some((item) => item.nodeKey === relative.key)) continue
        bundle.push({
          relation: relative.key === node.key ? 'self' : relative === parent ? 'parent' : 'sibling',
          nodeKey: relative.key,
          resourceId: relative.resourceId,
          component: relative.name,
          layoutFile: relative.source.layoutFile,
          attributes: relative.source.attributes,
        })
      }
    }
    node.children.forEach((child) => visit(child, node))
  }
  visit(root, null)
  return bundle.slice(0, 16)
}

function deterministicSummary(result: PwaCrossPlatformWritebackResult) {
  return {
    android: {
      applied: result.android.applied,
      changedFiles: result.android.changedFiles,
      sourceRevision: result.android.sourceRevision,
    },
    pwa: {
      applied: result.pwa.applied,
      changedFiles: result.pwa.changedFiles,
      sourceRevisions: result.pwa.sourceRevisions,
    },
  }
}

function compact(values: Array<string | undefined>) {
  return [...new Set(values.filter((value): value is string => Boolean(value)))].slice(0, 16)
}
