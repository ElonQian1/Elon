import type { UiTunerFilterResult } from './filtering'
import type { UiTunerStandardInsight } from './standards'
import type { UiTunerDocument, UiTunerElement } from './types'
import type { UiTunerSelectionScope } from './types'
import type { MetricItem } from './uiTunerGeometry'
import { UI_TUNER_CLOSURE_PRIORITIES } from './closurePriorities'
import type { UiTunerRepeatGroup } from './runtime/repeatComponents'
import type { UiTunerSelectionVisualContext } from './runtime/selectionArtifact'
import type { FitRunDocument } from './fit-run/types'
import type { PwaDraftCliCompactHandoff } from './source-preview/pwaDesignDraft'
import type { PwaDraftAiFitTask } from './source-preview/pwaAiFitTask'
import type {
  DesignArtifactRef,
  DesignBrowserRuntime,
  DesignCapabilities,
  DesignDraft,
  DesignDraftPreviewResult,
  DesignEvent,
  DesignPlatform,
  DesignSourceBindingCandidate,
  DesignTaskBinding,
  DesignWritebackReceipt,
  DesignVerificationMatrix,
  SemanticUiNode,
  TauriNativeHostEvidence,
  TauriBehaviorEvidence,
} from './headless-design/types'

export interface UiTunerCodexContextPack {
  version: 4
  kind: 'elon_ui_tuner_codex_context'
  generatedAt: string
  screen: {
    canvasName: string
    width: number
    height: number
    deviceId?: string
    packageName?: string
    activityName?: string
    sourceRoot?: string
    capturedAt?: string
    snapshotId?: string
    screenshotPath?: string
    hierarchyPath?: string
    manifestPath?: string
  }
  selectionScope: UiTunerSelectionScope
  selectedElement: {
    id: string
    name: string
    kind: string
    text: string
    rect: Rect
    style: Record<string, string | number>
    metrics: MetricItem[]
  } | null
  runtimeBinding: {
    resourceId?: string
    className?: string
    packageName?: string
    xpath?: string
    originalBounds?: Rect
    sourceFile?: string
    sourceLine?: number
    sourceToken?: string
    bindingConfidence?: UiTunerStandardInsight['bindingConfidence']
    bindingReason?: string
    sourceCandidates: Array<{
      file?: string
      line?: number
      token?: string
      confidence?: number
      reason?: string
      matchKind?: string
      componentKey?: string
      scope?: string
    }>
  }
  liveRuntime: {
    sessionId: string
    uiIrRevision?: string
    treeRevision: number
    runtimeNodeId?: string
    definitionId?: string
    mcpConfigPath?: string
    targetDesign?: {
      path: string
      sha256: string
      width: number
      height: number
    }
  } | null
  fitRun: {
    runId: string
    phase: string
    targetRect: { left: number; top: number; right: number; bottom: number }
    projectedTargetRect: { left: number; top: number; right: number; bottom: number }
    currentRect: { left: number; top: number; right: number; bottom: number }
    bestLoss?: number
    failedMetrics: string[]
    handoffPath?: string
    handoffReason?: string
    localEvaluations: number
    codexRounds: number
  } | null
  selectionVisual: {
    available: boolean
    cropPath?: string
    contextPath?: string
    error?: string
  }
  repeatedComponent: {
    groupId: string
    label: string
    count: number
    representativeId: string
    memberIds: string[]
    componentKey?: string
    sourceFile?: string
    confidence: string
  } | null
  requestedAdjustments: UiTunerRequestedAdjustment[]
  standardDraft: UiTunerStandardInsight | null
  layerClarity: {
    visibleCount: number
    totalCount: number
    hiddenCount: number
    structuralCount: number
    duplicateCount: number
    sourceMappedCount: number
    selectedHiddenReasons: string[]
  }
  codexContract: {
    readBeforeEdit: string[]
    writeTargets: string[]
    forbiddenShortcuts: string[]
    acceptance: string[]
  }
  closurePriorityIds: string[]
  pwaDesign?: {
    artifactVersion: string
    sourceRevision: string
    route: { path: string; search: string; hash: string }
    viewport: { width: number; height: number }
    compactHandoff: PwaDraftCliCompactHandoff
    aiFitTask: PwaDraftAiFitTask
    changes: unknown[]
    compactSourceBundle: Array<Record<string, unknown>>
    bindingSummary: Array<Record<string, unknown>>
    codexFallback: {
      reasons: string[]
      changes: unknown[]
    }
    deterministicSummary: Record<string, unknown>
    visualReferences: Record<string, string | undefined>
    runtimeCapture?: {
      path: string
      sha256: string
      manifestPath: string
      width: number
      height: number
      mediaType: 'image/png'
    }
    writebackPlan: unknown
    deterministicResult: unknown
    machineReceipt?: {
      receiptId: string
      sourceRevisionBefore: string
      sourceRevision: string
      sourceHash: string
      changedFiles: string[]
      targetPlatforms: string[]
      platformResults: Record<string, unknown>
    }
    contextPolicy: {
      fullRepositoryIncluded: false
      fullDomIncluded: false
      screenshotsEmbeddedAsBase64: false
    }
    capabilities: { PWA_CODE_GENERATION: true }
  }
  headlessDesign?: {
    designSessionId?: string
    platform: DesignPlatform
    route: string
    url?: string
    state: string
    evidenceLevel: string
    nativeHostVerified: boolean
    pixels?: DesignArtifactRef
    uiTree?: DesignArtifactRef
    nativeHost?: TauriNativeHostEvidence
    designDraft?: DesignDraft
    writebackReceipt?: DesignWritebackReceipt
    capabilities?: DesignCapabilities
    browserRuntime?: DesignBrowserRuntime
    tauriBehavior?: TauriBehaviorEvidence
    verificationMatrix?: DesignVerificationMatrix
    draftPreview?: DesignDraftPreviewResult
    sourceBindingCandidates?: DesignSourceBindingCandidate[]
    liveFollow?: {
      active: boolean
      taskId?: string
      binding?: DesignTaskBinding
      cursor?: string
      latestEvents?: DesignEvent[]
      lastSyncedAt?: string
      error?: string
    }
    selectedNode?: SemanticUiNode
    contextPolicy: {
      fullRepositoryIncluded: false
      fullDomIncluded: false
      screenshotsEmbeddedAsBase64: false
    }
  }
}

interface Rect {
  x: number
  y: number
  width: number
  height: number
}

export interface UiTunerRequestedAdjustment {
  property: string
  before: string | number
  after: string | number
  unit?: string
  sourceHint: string
}

interface BuildPackArgs {
  document: UiTunerDocument
  selected: UiTunerElement | null
  metrics: MetricItem[]
  filterResult: UiTunerFilterResult
  standardInsight: UiTunerStandardInsight | null
  selectionScope: UiTunerSelectionScope
  repeatGroup: UiTunerRepeatGroup | null
  selectionVisual: UiTunerSelectionVisualContext | null
  liveContext?: UiTunerLiveContext
  fitRun?: FitRunDocument | null
}

export interface UiTunerLiveContext {
  sessionId?: string
  uiIrRevision?: string
  treeRevision?: number
  runtimeNodeId?: string
  definitionId?: string
  mcpConfigPath?: string
  targetDesign?: {
    path: string
    sha256: string
    width: number
    height: number
  }
}

export function buildUiTunerCodexContextPack({
  document,
  selected,
  metrics,
  filterResult,
  standardInsight,
  selectionScope,
  repeatGroup,
  selectionVisual,
  liveContext,
  fitRun,
}: BuildPackArgs): UiTunerCodexContextPack {
  const runtime = selected?.runtime
  const source = selected?.source
  return {
    version: 4,
    kind: 'elon_ui_tuner_codex_context',
    generatedAt: new Date().toISOString(),
    screen: {
      canvasName: document.canvas.name,
      width: document.canvas.width,
      height: document.canvas.height,
      deviceId: document.runtimeSnapshot?.deviceId,
      packageName: document.runtimeSnapshot?.packageName,
      activityName: document.runtimeSnapshot?.activityName,
      sourceRoot: document.runtimeSnapshot?.sourceRoot,
      capturedAt: document.runtimeSnapshot?.capturedAt,
      snapshotId: document.runtimeSnapshot?.artifact?.id,
      screenshotPath: document.runtimeSnapshot?.artifact?.screenshotPath,
      hierarchyPath: document.runtimeSnapshot?.artifact?.hierarchyPath,
      manifestPath: document.runtimeSnapshot?.artifact?.manifestPath,
    },
    selectionScope,
    selectedElement: selected ? {
      id: selected.id,
      name: selected.name,
      kind: selected.kind,
      text: selected.text,
      rect: rectFromElement(selected),
      style: styleFromElement(selected),
      metrics,
    } : null,
    runtimeBinding: {
      resourceId: runtime?.resourceId,
      className: runtime?.className,
      packageName: runtime?.packageName,
      xpath: runtime?.xpath,
      originalBounds: runtime ? {
        x: runtime.originalBounds.left,
        y: runtime.originalBounds.top,
        width: runtime.originalBounds.width,
        height: runtime.originalBounds.height,
      } : undefined,
      sourceFile: source?.file,
      sourceLine: source?.line,
      sourceToken: source?.token,
      bindingConfidence: standardInsight?.bindingConfidence,
      bindingReason: standardInsight?.bindingReason,
      sourceCandidates: (selected?.sourceCandidates ?? []).slice(0, 8).map((candidate) => ({
        file: candidate.file,
        line: candidate.line,
        token: candidate.token,
        confidence: candidate.confidence,
        reason: candidate.reason,
        matchKind: candidate.matchKind,
        componentKey: candidate.componentKey,
        scope: candidate.scope,
      })),
    },
    liveRuntime: liveContext?.sessionId ? {
      sessionId: liveContext.sessionId,
      uiIrRevision: liveContext.uiIrRevision,
      treeRevision: liveContext.treeRevision ?? 0,
      runtimeNodeId: liveContext.runtimeNodeId,
      definitionId: liveContext.definitionId,
      mcpConfigPath: liveContext.mcpConfigPath,
      targetDesign: liveContext.targetDesign,
    } : null,
    fitRun: fitRun ? {
      runId: fitRun.runId,
      phase: fitRun.phase,
      targetRect: fitRun.pair.targetRect,
      projectedTargetRect: fitRun.pair.projectedTargetRect,
      currentRect: fitRun.pair.currentRect,
      bestLoss: fitRun.best?.score.overallLoss,
      failedMetrics: fitRun.best?.score.hardFailures ?? [],
      handoffPath: fitRun.handoff?.artifactPath,
      handoffReason: fitRun.handoff?.reason,
      localEvaluations: fitRun.usage.localEvaluations,
      codexRounds: fitRun.usage.codexRounds,
    } : null,
    selectionVisual: {
      available: Boolean(selectionVisual?.previewDataUrl),
      cropPath: selectionVisual?.artifact?.cropPath,
      contextPath: selectionVisual?.artifact?.contextPath,
      error: selectionVisual?.error,
    },
    repeatedComponent: repeatGroup ? {
      groupId: repeatGroup.id,
      label: repeatGroup.label,
      count: repeatGroup.count,
      representativeId: repeatGroup.representativeId,
      memberIds: repeatGroup.memberIds,
      componentKey: repeatGroup.componentKey,
      sourceFile: repeatGroup.sourceFile,
      confidence: repeatGroup.confidence,
    } : null,
    requestedAdjustments: selected ? requestedAdjustments(selected) : [],
    standardDraft: standardInsight,
    layerClarity: {
      visibleCount: filterResult.visible.length,
      totalCount: filterResult.totalCount,
      hiddenCount: filterResult.hiddenCount,
      structuralCount: filterResult.structuralCount,
      duplicateCount: filterResult.duplicateCount,
      sourceMappedCount: filterResult.sourceMappedCount,
      selectedHiddenReasons: selected
        ? filterResult.analysisById[selected.id]?.hiddenReasons ?? []
        : [],
    },
    codexContract: {
      readBeforeEdit: source?.file
        ? compactPaths([
            document.runtimeSnapshot?.artifact?.manifestPath,
            document.runtimeSnapshot?.artifact?.hierarchyPath,
            selectionVisual?.artifact?.cropPath,
            source.file,
          ])
        : [
            ...compactPaths([
              document.runtimeSnapshot?.artifact?.manifestPath,
              document.runtimeSnapshot?.artifact?.hierarchyPath,
              selectionVisual?.artifact?.cropPath,
            ]),
            'AndroidManifest.xml',
            'app/src/main/res/layout/*.xml',
            'app/src/main/res/values/*.xml',
          ],
      writeTargets: standardInsight?.saveTargets.map((target) => target.path) ?? [
        '.elon/ui-standards/tokens.json',
        '.elon/ui-standards/components.json',
      ],
      forbiddenShortcuts: [
        '不要只修改画布 JSON 后宣称完成',
        '不要只按截图坐标硬编码绝对位置',
        '不要在没有源码绑定时改全局 token',
      ],
      acceptance: [
        '说明选中 XML 节点如何映射到源码',
        '输出 Android 源码或 UI 标准配置 diff',
        `按 ${selectionScope} 作用范围修改，不能误改其他实例`,
        '重新采集真机页面或给出可执行验证命令',
        '把可复用标准保存到 JSON 配置而不是 Markdown',
      ],
    },
    closurePriorityIds: UI_TUNER_CLOSURE_PRIORITIES.map((item) => item.id),
  }
}

export function stringifyUiTunerCodexContextPack(args: BuildPackArgs) {
  return JSON.stringify(buildUiTunerCodexContextPack(args), null, 2)
}

export function buildUiTunerCodexTaskPrompt(pack: UiTunerCodexContextPack, userIntent: string) {
  if (pack.headlessDesign) return buildHeadlessDesignTaskPrompt(pack, userIntent)
  const selected = pack.selectedElement
  const target = selected
    ? `${selected.name} (${selected.kind}, ${selected.rect.x},${selected.rect.y} ${selected.rect.width}x${selected.rect.height})`
    : '当前页面'
  const intent = userIntent.trim() || '请基于当前选中节点给出 APK UI 美观标准，并把可复用部分沉淀为配置。'
  return [
    '你正在处理一龙项目的 `/pc/ui-tuner` 微调画布闭环任务。',
    '',
    `用户选中的 APK 元素：${target}`,
    `作用范围：${scopeLabel(pack.selectionScope)}${pack.repeatedComponent ? `（同组件共 ${pack.repeatedComponent.count} 个实例）` : ''}`,
    `用户意图：${intent}`,
    '',
    '请按以下顺序执行：',
    pack.fitRun?.handoffPath
      ? `1. 先读取 FitRun handoff ${pack.fitRun.handoffPath}，再通过 yilong-ui-live MCP 按需获取最新局部画面和源码。`
      : pack.liveRuntime?.mcpConfigPath
        ? `1. 先读取本机 ${pack.liveRuntime.mcpConfigPath}，优先通过 yilong-ui-live 按需工具获取 screen summary、节点、局部源码和视觉差异。`
      : '1. 先读 context pack 中的选区截图、runtimeBinding、sourceCandidates、requestedAdjustments 和 codexContract。',
    '2. 不要重复读取整棵树或全仓库；定位 Android 源码或 UI 标准配置，区分 design token、组件标准和页面覆盖。',
    '3. 给出或实施可审查的源码/config 修改，不要只改截图坐标。',
    '4. 输出验证方式：构建检查、重新 ADB 捕获或可执行的验收命令。',
    '5. 如果绑定置信度不足，先列出需要人工确认的 resourceId/xpath/source 文件。',
    '',
    'Compact context pack JSON:',
    '```json',
    JSON.stringify(pack, null, 2),
    '```',
  ].join('\n')
}

function buildHeadlessDesignTaskPrompt(pack: UiTunerCodexContextPack, userIntent: string) {
  const design = pack.headlessDesign!
  const selected = pack.selectedElement
  return [
    '你正在处理一龙 `/pc/ui-tuner` 的多端后台 designSession。',
    `平台：${design.platform}；路由：${design.route}；designSessionId：${design.designSessionId ?? '尚未打开'}`,
    selected ? `当前语义节点：${selected.name}（${design.selectedNode?.selector ?? selected.id}）` : '当前没有选中语义节点。',
    `用户意图：${userIntent.trim() || '请修改当前页面并重新捕获后台证据。'}`,
    '',
    '执行顺序：',
    '1. 先用 ui_get_design_capabilities 确认已安装节点 schema，再用 ui_list_design_sessions 恢复 designSessionId；不要先操控 PC 桌面。',
    '2. 结合 route、selector 和 UI tree 调用 ui_suggest_design_source_binding；候选先保持 CANDIDATE，核对文件哈希与范围后才更新为 BOUND。',
    '3. 若 context pack 含 designDraft，可先用 ui_preview_design_draft / ui_restore_design_draft_preview 对话式查看；预览不修改源码，也不是完成证明。',
    '4. Web/PWA/Tauri 前端调试优先复用 ui_prepare_design_browser / ui_interact_design_browser 的同一页面状态；fill/select 只能引用 fixtureProfile.formValues，禁止在参数中传秘密。',
    '5. Tauri 原生层按窗口、菜单/对话框、项目插桩 command trace 分层取证；不得点击任意系统菜单或执行任意 Rust command。',
    '6. 用 ui_complete_design_writeback 提交 changedFiles、源码哈希和各平台 evidence，再读取 ui_get_design_verification_matrix；只有 receipt.complete=true 且矩阵 PASSED 才声明完成。',
    '7. 明确平台覆盖：只有 nativeHost.nativeHostVerified=true 才能声明 Tauri 原生窗口证据；Android 必须使用 Android Runtime。',
    '',
    'Compact context pack JSON:',
    '```json',
    JSON.stringify(pack, null, 2),
    '```',
  ].join('\n')
}

function requestedAdjustments(element: UiTunerElement): UiTunerRequestedAdjustment[] {
  const runtime = element.runtime
  if (!runtime) return []
  const values: Array<[string, string | number, string | number, string | undefined, string]> = [
    ['x', runtime.originalBounds.left, element.x, 'px', '优先转换为父布局约束、margin 或 alignment'],
    ['y', runtime.originalBounds.top, element.y, 'px', '优先转换为父布局约束、margin 或列表间距'],
    ['width', runtime.originalBounds.width, element.width, 'px', '优先修改 layout width、weight 或约束'],
    ['height', runtime.originalBounds.height, element.height, 'px', '优先修改 layout height、minHeight 或内容间距'],
    ['fontSize', runtime.originalStyle?.fontSize, element.fontSize, 'sp', '优先修改 typography token'],
    ['lineHeight', runtime.originalStyle?.lineHeight, element.lineHeight, 'sp', '优先修改 typography token'],
    ['paddingX', runtime.originalStyle?.paddingX, element.paddingX, 'dp', '优先修改组件水平 padding token'],
    ['paddingY', runtime.originalStyle?.paddingY, element.paddingY, 'dp', '优先修改组件垂直 padding token'],
    ['borderRadius', runtime.originalStyle?.borderRadius, element.borderRadius, 'dp', '优先修改 shape/radius token'],
    ['color', runtime.originalStyle?.color, element.color, undefined, '优先修改语义颜色 token'],
    ['background', runtime.originalStyle?.background, element.background, undefined, '优先修改组件背景 token'],
    ['opacity', runtime.originalStyle?.opacity, element.opacity, undefined, '仅在源码确有透明度语义时修改'],
  ]
  return values
    .filter(([, before, after]) => before !== undefined && before !== after)
    .map(([property, before, after, unit, sourceHint]) => ({ property, before, after, unit, sourceHint }))
}

function compactPaths(values: Array<string | undefined>) {
  return values.filter((value): value is string => Boolean(value))
}

function scopeLabel(scope: UiTunerSelectionScope) {
  if (scope === 'component') return '同类组件全部实例'
  if (scope === 'screen') return '当前页面'
  if (scope === 'project') return '全项目设计标准'
  return '仅当前实例'
}

function rectFromElement(element: UiTunerElement): Rect {
  return {
    x: element.x,
    y: element.y,
    width: element.width,
    height: element.height,
  }
}

function styleFromElement(element: UiTunerElement) {
  return {
    fontSize: element.fontSize,
    lineHeight: element.lineHeight,
    fontWeight: element.fontWeight,
    letterSpacing: element.letterSpacing,
    paddingX: element.paddingX,
    paddingY: element.paddingY,
    borderRadius: element.borderRadius,
    borderWidth: element.borderWidth,
    color: element.color,
    background: element.background,
    borderColor: element.borderColor,
    opacity: element.opacity,
  }
}
