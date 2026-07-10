import type { UiTunerFilterResult } from './filtering'
import type { UiTunerStandardInsight } from './standards'
import type { UiTunerDocument, UiTunerElement } from './types'
import type { UiTunerSelectionScope } from './types'
import type { MetricItem } from './uiTunerGeometry'
import { UI_TUNER_CLOSURE_PRIORITIES } from './closurePriorities'
import type { UiTunerRepeatGroup } from './runtime/repeatComponents'
import type { UiTunerSelectionVisualContext } from './runtime/selectionArtifact'

export interface UiTunerCodexContextPack {
  version: 2
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
}: BuildPackArgs): UiTunerCodexContextPack {
  const runtime = selected?.runtime
  const source = selected?.source
  return {
    version: 2,
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
      sourceCandidates: (selected?.sourceCandidates ?? []).map((candidate) => ({
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
    '1. 先读 context pack 中的选区截图、runtimeBinding、sourceCandidates、requestedAdjustments 和 codexContract。',
    '2. 定位 Android 源码或 UI 标准配置，区分 design token、组件标准和页面覆盖。',
    '3. 给出或实施可审查的源码/config 修改，不要只改截图坐标。',
    '4. 输出验证方式：构建检查、重新 ADB 捕获或可执行的验收命令。',
    '5. 如果绑定置信度不足，先列出需要人工确认的 resourceId/xpath/source 文件。',
    '',
    'Context pack JSON:',
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
