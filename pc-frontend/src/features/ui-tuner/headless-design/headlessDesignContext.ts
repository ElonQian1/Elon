import { UI_TUNER_CLOSURE_PRIORITIES } from '../closurePriorities'
import type { UiTunerCodexContextPack } from '../contextPack'
import type {
  DesignSessionIdentity,
  DesignSurface,
  DesignTarget,
  SemanticUiNode,
} from './types'

export function buildHeadlessDesignContext(input: {
  projectRoot: string
  target: DesignTarget | null
  session: DesignSessionIdentity | null
  surface: DesignSurface | null
  selectedNode: SemanticUiNode | null
}): UiTunerCodexContextPack {
  const { projectRoot, target, session, surface, selectedNode } = input
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
      sourceCandidates: [...sourceRoots, ...configFiles].slice(0, 8).map((file) => ({
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
