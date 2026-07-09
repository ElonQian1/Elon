import type { UiTunerFilterResult } from './filtering'
import type { UiTunerStandardInsight } from './standards'
import type { UiTunerDocument, UiTunerElement } from './types'
import type { MetricItem } from './uiTunerGeometry'
import { UI_TUNER_CLOSURE_PRIORITIES } from './closurePriorities'

export interface UiTunerCodexContextPack {
  version: 1
  kind: 'elon_ui_tuner_codex_context'
  generatedAt: string
  screen: {
    canvasName: string
    width: number
    height: number
    packageName?: string
    activityName?: string
    sourceRoot?: string
    capturedAt?: string
  }
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
  }
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

interface BuildPackArgs {
  document: UiTunerDocument
  selected: UiTunerElement | null
  metrics: MetricItem[]
  filterResult: UiTunerFilterResult
  standardInsight: UiTunerStandardInsight | null
}

export function buildUiTunerCodexContextPack({
  document,
  selected,
  metrics,
  filterResult,
  standardInsight,
}: BuildPackArgs): UiTunerCodexContextPack {
  const runtime = selected?.runtime
  const source = selected?.source
  return {
    version: 1,
    kind: 'elon_ui_tuner_codex_context',
    generatedAt: new Date().toISOString(),
    screen: {
      canvasName: document.canvas.name,
      width: document.canvas.width,
      height: document.canvas.height,
      packageName: document.runtimeSnapshot?.packageName,
      activityName: document.runtimeSnapshot?.activityName,
      sourceRoot: document.runtimeSnapshot?.sourceRoot,
      capturedAt: document.runtimeSnapshot?.capturedAt,
    },
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
    },
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
        ? [source.file]
        : [
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
    `用户意图：${intent}`,
    '',
    '请按以下顺序执行：',
    '1. 先读 context pack 中的 runtimeBinding、standardDraft、codexContract。',
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
