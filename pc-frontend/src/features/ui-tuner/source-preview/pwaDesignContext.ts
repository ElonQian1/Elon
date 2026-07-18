import type { UiTunerCodexContextPack } from '../contextPack'
import type { PwaDesignDraft } from './pwaDesignDraft'
import type { PwaDesignWritebackPlan, PwaDeterministicWritebackResult } from './pwaDesignWriteback'
import type { PwaSelection } from './usePwaDesignSession'
import type { SourcePreviewNode } from './types'

export function buildPwaDesignContextPack(input: {
  draft: PwaDesignDraft
  root: SourcePreviewNode | null
  selection: PwaSelection | null
  plan: PwaDesignWritebackPlan
  deterministicResult: PwaDeterministicWritebackResult
}): UiTunerCodexContextPack {
  const selected = selectedDraftElement(input.draft, input.selection)
  const androidCandidate = selected?.binding.androidCandidates[0]
  const adjustments = Object.values(input.draft.elements).flatMap((element) => (
    Object.entries(element.styleDiff).map(([property, after]) => ({
      property,
      before: element.originalStyle.authored[property as keyof typeof element.originalStyle.authored]
        ?? element.originalStyle.computed[property as keyof typeof element.originalStyle.computed]
        ?? '',
      after,
      sourceHint: element.binding.needsBinding
        ? '先按稳定身份建立来源绑定，再写回 PWA/Android 源码'
        : '优先使用明确绑定的样式或资源属性确定性写回',
    }))
  ))
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
      screenshotPath: visual.screenshot,
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
      style: selected.afterStyle,
      metrics: [],
    } : null,
    runtimeBinding: {
      resourceId: androidCandidate?.resourceId,
      sourceFile: androidCandidate?.file,
      sourceToken: androidCandidate?.symbol,
      bindingConfidence: selected?.binding.bindingConfidence,
      bindingReason: selected?.binding.needsBinding ? '需要 AI 建立完整 PWA/Android 来源绑定' : '稳定身份与双端源码已绑定',
      sourceCandidates: (selected?.binding.androidCandidates ?? []).map((candidate) => ({
        file: candidate.file,
        token: candidate.symbol,
        confidence: candidate.confidence,
        reason: candidate.reason,
        componentKey: candidate.stableKey,
        scope: selected?.scope,
      })),
    },
    liveRuntime: null,
    fitRun: null,
    selectionVisual: {
      available: Boolean(visual.currentCrop || visual.screenshot),
      cropPath: visual.currentCrop || visual.screenshot,
      contextPath: visual.visualDiff,
    },
    repeatedComponent: null,
    requestedAdjustments: adjustments,
    standardDraft: null,
    layerClarity: {
      visibleCount: Object.keys(input.draft.elements).length,
      totalCount: Object.keys(input.draft.elements).length,
      hiddenCount: 0, structuralCount: 0, duplicateCount: 0,
      sourceMappedCount: Object.values(input.draft.elements).filter((element) => !element.binding.needsBinding).length,
      selectedHiddenReasons: [],
    },
    codexContract: {
      readBeforeEdit: compact([
        visual.targetCrop, visual.currentCrop, visual.visualDiff,
        ...input.plan.deterministic.map((action) => action.layoutFile),
      ]),
      writeTargets: compact([
        ...input.deterministicResult.changedFiles,
        ...Object.values(input.draft.elements).flatMap((element) => element.binding.pwaCandidates.map((item) => item.file)),
      ]),
      forbiddenShortcuts: [
        '不要读取整仓库或整棵 DOM',
        '不要把 Runtime DOM 或 selector 当成最终源码真相',
        '不要重复执行已完成的 Android 确定性写回',
      ],
      acceptance: [
        '按 stableId/testId/resourceId/source symbol/组件路径建立 PWA 来源绑定',
        '只处理草稿 diff 和 compactSourceBundle 指向的局部源码',
        'PWA 与 APK 两端都必须给出源码写回或明确阻塞证据',
      ],
    },
    closurePriorityIds: [],
    pwaDesign: {
      artifactVersion: input.draft.artifactVersion,
      sourceRevision: input.draft.project.sourceRevision,
      route: input.draft.route,
      viewport: input.draft.viewport,
      changes: Object.values(input.draft.elements),
      compactSourceBundle: compactSourceBundle(input.root, input.draft),
      visualReferences: { ...input.draft.visualReferences },
      writebackPlan: input.plan,
      deterministicResult: input.deterministicResult,
      contextPolicy: {
        fullRepositoryIncluded: false,
        fullDomIncluded: false,
        screenshotsEmbeddedAsBase64: false,
      },
      capabilities: { PWA_CODE_GENERATION: true },
    },
  }
}

function selectedDraftElement(draft: PwaDesignDraft, selection: PwaSelection | null) {
  if (selection) {
    const match = Object.values(draft.elements).find((element) => (
      element.identity.key === selection.identity.key || element.identity.selector === selection.identity.selector
    ))
    if (match) return match
  }
  return Object.values(draft.elements)[0]
}

function compactSourceBundle(root: SourcePreviewNode | null, draft: PwaDesignDraft) {
  if (!root) return []
  const targetKeys = new Set(Object.values(draft.elements).flatMap((element) => (
    element.binding.androidCandidates.slice(0, 2).map((candidate) => candidate.stableKey)
  )))
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

function compact(values: Array<string | undefined>) {
  return [...new Set(values.filter((value): value is string => Boolean(value)))]
}
