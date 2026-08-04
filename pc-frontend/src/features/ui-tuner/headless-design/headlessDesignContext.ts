import { UI_TUNER_CLOSURE_PRIORITIES } from '../closurePriorities'
import type { UiTunerCodexContextPack } from '../contextPack'
import type {
  DesignSessionIdentity,
  DesignDraft,
  DesignDraftPreviewResult,
  DesignEvent,
  DesignBrowserRuntime,
  DesignCapabilities,
  DesignSurface,
  DesignSourceBindingCandidate,
  DesignTarget,
  DesignTaskBinding,
  DesignWritebackReceipt,
  DesignVerificationMatrix,
  SemanticUiNode,
  TauriBehaviorEvidence,
} from './types'
import type {
  DesignBindingHealth,
  DesignEventCheckpoint,
  DesignIntentPlan,
  DesignSourcePatchProposal,
  DesignSourceRollbackPlan,
  DesignWritebackPlan,
} from './designPlanningTypes'

export function buildHeadlessDesignContext(input: {
  projectRoot: string
  target: DesignTarget | null
  session: DesignSessionIdentity | null
  surface: DesignSurface | null
  selectedNode: SemanticUiNode | null
  designDraft: DesignDraft | null
  writebackReceipt: DesignWritebackReceipt | null
  capabilities: DesignCapabilities | null
  browserRuntime: DesignBrowserRuntime | null
  tauriBehavior: TauriBehaviorEvidence | null
  verificationMatrix: DesignVerificationMatrix | null
  draftPreview: DesignDraftPreviewResult | null
  sourceBindingCandidates: DesignSourceBindingCandidate[]
  intentPlan: DesignIntentPlan | null
  bindingHealth: DesignBindingHealth | null
  writebackPlan: DesignWritebackPlan | null
  sourcePatch: DesignSourcePatchProposal | null
  rollbackPlan: DesignSourceRollbackPlan | null
  liveFollow: {
    active: boolean
    taskId: string
    binding: DesignTaskBinding | null
    cursor: string
    checkpoint: DesignEventCheckpoint | null
    latestEvents: DesignEvent[]
    lastSyncedAt: string
    error: string
  }
}): UiTunerCodexContextPack {
  const { projectRoot, target, session, surface, selectedNode, designDraft, writebackReceipt,
    capabilities, browserRuntime, tauriBehavior, verificationMatrix,
    draftPreview, sourceBindingCandidates, intentPlan, bindingHealth, writebackPlan,
    sourcePatch, rollbackPlan, liveFollow } = input
  const viewport = surface?.surface?.viewport ?? session?.viewport ?? { width: 1280, height: 800, deviceScaleFactor: 1 }
  const sourceRoots = target?.sourceRoots ?? []
  const configFiles = target?.configFiles ?? []
  return {
    version: 4,
    kind: 'elon_ui_tuner_codex_context',
    generatedAt: new Date().toISOString(),
    screen: {
      canvasName: `${target?.label ?? '多端页面'} · ${session?.route ?? '/'}`,
      width: viewport.width,
      height: viewport.height,
      sourceRoot: projectRoot,
      capturedAt: session && 'updatedAt' in session ? String(session.updatedAt) : undefined,
      screenshotPath: surface?.pixels?.path,
    },
    selectionScope: 'instance',
    selectedElement: selectedNode ? {
      id: selectedNode.id,
      name: selectedNode.label || selectedNode.selector,
      kind: selectedNode.role || selectedNode.tag,
      text: selectedNode.label,
      rect: {
        x: selectedNode.bounds.left,
        y: selectedNode.bounds.top,
        width: selectedNode.bounds.width,
        height: selectedNode.bounds.height,
      },
      style: { ...selectedNode.style },
      metrics: [],
    } : null,
    runtimeBinding: {
      xpath: selectedNode?.selector,
      bindingConfidence: 'low',
      bindingReason: selectedNode
        ? '当前只有后台语义 selector；修改前仍需通过项目源码建立 source binding'
        : '尚未选择语义 UI 节点',
      sourceCandidates: sourceBindingCandidates.length
        ? sourceBindingCandidates.slice(0, 8).map((candidate) => ({
            file: candidate.file,
            line: candidate.line,
            confidence: Math.min(1, candidate.score / 120),
            reason: candidate.suggestedBinding.reason,
            matchKind: candidate.suggestedBinding.kind,
            scope: target?.platform,
          }))
        : [...sourceRoots, ...configFiles].slice(0, 8).map((file) => ({
            file,
            reason: '由多端目标发现器提供，需按 selector/route 继续缩小源码绑定',
            scope: target?.platform,
          })),
    },
    liveRuntime: null,
    fitRun: null,
    selectionVisual: {
      available: Boolean(surface?.pixels?.path),
      cropPath: surface?.pixels?.path,
      contextPath: surface?.uiTree?.path,
    },
    repeatedComponent: null,
    requestedAdjustments: [],
    standardDraft: null,
    layerClarity: {
      visibleCount: surface?.nodes.length ?? 0,
      totalCount: surface?.surface?.nodeCount ?? surface?.nodes.length ?? 0,
      hiddenCount: 0,
      structuralCount: surface?.nodes.filter((node) => !node.interactive).length ?? 0,
      duplicateCount: 0,
      sourceMappedCount: 0,
      selectedHiddenReasons: [],
    },
    codexContract: {
      readBeforeEdit: compact([surface?.uiTree?.path, surface?.pixels?.path, ...configFiles]),
      writeTargets: sourceRoots,
      forbiddenShortcuts: [
        '不要操控 Windows 桌面来代替后台 designSession',
        '不要只按截图坐标硬编码布局',
        '不要把 Tauri WebView 前端证据冒充原生宿主验证',
        '不要读取整仓库或把 PNG/Base64 塞进上下文',
      ],
      acceptance: [
        'AI 任务通过 taskId lease 绑定 designSession，并用 cursor 增量发布设计事件',
        '存在 DesignIntentPlan 时，在打开匹配 platform/route 的 designSession 后以 expectedRevision 启动计划',
        '每个计划动作写入 RUNNING/SUCCEEDED/FAILED/SKIPPED 回执和紧凑证据引用；失败、暂停或意图变化时显式转换或重规划',
        '源码变更使用精确 range/SHA 的 source patch 提案；读取 review artifact，显式批准后才应用，并为已应用补丁生成回滚计划',
        '通过 designSessionId 读取当前语义 UI 树和截图哈希',
        '为选中 selector 建立可审查的源码绑定并修改源码',
        '重新捕获同一 platform/route 并给出新的 UI tree 与 PNG 哈希',
        '分别声明 Web、PWA、Tauri 前端或 Android Runtime 的证据覆盖范围',
      ],
    },
    closurePriorityIds: UI_TUNER_CLOSURE_PRIORITIES.map((item) => item.id),
    headlessDesign: {
      designSessionId: session?.designSessionId,
      platform: target?.platform ?? session?.platform ?? 'web',
      route: session?.route ?? '/',
      url: session?.url ?? undefined,
      state: session?.state ?? 'NOT_OPENED',
      evidenceLevel: target?.evidenceLevel ?? 'UNAVAILABLE',
      nativeHostVerified: surface?.nativeHost?.nativeHostVerified ?? target?.nativeHostVerified ?? false,
      pixels: surface?.pixels ?? undefined,
      uiTree: surface?.uiTree ?? undefined,
      nativeHost: surface?.nativeHost ?? undefined,
      designDraft: designDraft ?? undefined,
      writebackReceipt: writebackReceipt ?? undefined,
      capabilities: capabilities ?? undefined,
      browserRuntime: browserRuntime ?? undefined,
      tauriBehavior: tauriBehavior ?? undefined,
      verificationMatrix: verificationMatrix ?? undefined,
      draftPreview: draftPreview ?? undefined,
      sourceBindingCandidates: sourceBindingCandidates.slice(0, 8),
      intentPlan: intentPlan ?? undefined,
      bindingHealth: bindingHealth ?? undefined,
      writebackPlan: writebackPlan ?? undefined,
      sourcePatch: sourcePatch ?? undefined,
      rollbackPlan: rollbackPlan ?? undefined,
      liveFollow: {
        active: liveFollow.active,
        taskId: liveFollow.taskId || undefined,
        binding: liveFollow.binding ?? undefined,
        cursor: liveFollow.cursor || undefined,
        checkpoint: liveFollow.checkpoint ?? undefined,
        latestEvents: liveFollow.latestEvents.slice(-8),
        lastSyncedAt: liveFollow.lastSyncedAt || undefined,
        error: liveFollow.error || undefined,
      },
      selectedNode: selectedNode ?? undefined,
      contextPolicy: {
        fullRepositoryIncluded: false,
        fullDomIncluded: false,
        screenshotsEmbeddedAsBase64: false,
      },
    },
  }
}

function compact(values: Array<string | undefined>) {
  return [...new Set(values.filter((value): value is string => Boolean(value)))]
}
