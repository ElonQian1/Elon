import type {
  UiTunerDocument,
  UiTunerElement,
  UiTunerElementStandard,
  UiTunerStandardScope,
} from './types'
import { UI_TUNER_CLOSURE_PRIORITIES } from './closurePriorities'

export interface UiTunerStandardInsight {
  standard: UiTunerElementStandard
  saveTargets: UiTunerSaveTarget[]
  bindingConfidence: 'high' | 'medium' | 'low'
  bindingReason: string
  reusableFields: string[]
  screenOnlyFields: string[]
}

export interface UiTunerSaveTarget {
  id: UiTunerStandardScope
  label: string
  path: string
  intent: string
  recommended: boolean
}

export function buildStandardInsight(
  document: UiTunerDocument,
  element: UiTunerElement | null,
): UiTunerStandardInsight | null {
  if (!element) return null
  const component = inferComponentName(element)
  const role = inferRole(element)
  const standard: UiTunerElementStandard = {
    scope: inferRecommendedScope(element),
    role,
    component,
    variant: inferVariant(element),
    tokenRefs: inferTokenRefs(element),
    reuseKey: buildReuseKey(document, component, role),
    note: inferNote(element),
  }

  return {
    standard,
    saveTargets: buildSaveTargets(document, standard.scope),
    bindingConfidence: inferBindingConfidence(element),
    bindingReason: inferBindingReason(element),
    reusableFields: reusableFields(element),
    screenOnlyFields: screenOnlyFields(),
  }
}

export function stringifyStandardPackage(document: UiTunerDocument, selected: UiTunerElement | null) {
  const selectedInsight = buildStandardInsight(document, selected)
  const components = document.elements
    .filter((element) => element.visibility !== 'hidden')
    .map((element) => {
      const insight = buildStandardInsight(document, element)
      return insight
        ? {
            elementId: element.id,
            resourceId: element.runtime?.resourceId,
            source: element.source,
            standard: element.standard ?? insight.standard,
            rect: {
              x: element.x,
              y: element.y,
              width: element.width,
              height: element.height,
            },
          }
        : null
    })
    .filter(Boolean)

  return JSON.stringify({
    version: 1,
    kind: 'elon_ui_tuner_standard_draft',
    goal: '把微调画布中的运行时节点沉淀为可复用的项目 UI 标准，再由 CLI 转换为 Android values/layout 修改。',
    recommendedPaths: [
      '.elon/ui-standards/tokens.json',
      '.elon/ui-standards/components.json',
      '.elon/ui-tuner/screens/<package>/<activity>.json',
    ],
    rules: {
      selectionIdentity: '每个标准必须优先绑定 resourceId/source token/xpath，低置信度节点只能保存为 screen_override 或 local_draft。',
      reusePolicy: '颜色、字号、间距、圆角进入 tokens；按钮、卡片、导航进入 components；绝对坐标只进入 screens。',
      codexContract: 'Codex 修改 APK UI 时必须读取 context pack、回写源码或 JSON 标准，并给出真机或构建验收。',
      visibilityPolicy: '产品模式默认隐藏结构容器、重复边界和非目标包节点；debug 模式保留全部 XML 可追溯。',
    },
    source: document.source,
    runtimeSnapshot: document.runtimeSnapshot,
    selected: selectedInsight,
    components,
    closurePriorities: UI_TUNER_CLOSURE_PRIORITIES,
    exportedAt: new Date().toISOString(),
  }, null, 2)
}

function buildSaveTargets(document: UiTunerDocument, recommendedScope: UiTunerStandardScope): UiTunerSaveTarget[] {
  const packageName = document.runtimeSnapshot?.packageName ?? 'apk'
  const activity = document.runtimeSnapshot?.activityName ?? 'screen'
  return [
    {
      id: 'local_draft',
      label: '本机草稿',
      path: 'browser localStorage',
      intent: '快速试错，不作为团队标准。',
      recommended: recommendedScope === 'local_draft',
    },
    {
      id: 'project_component',
      label: '项目组件标准',
      path: '.elon/ui-standards/components.json',
      intent: '按钮、卡片、导航、输入框等跨页面复用组件。',
      recommended: recommendedScope === 'project_component',
    },
    {
      id: 'design_token',
      label: '设计 Token',
      path: '.elon/ui-standards/tokens.json',
      intent: '颜色、字号、间距、圆角等基础标准。',
      recommended: recommendedScope === 'design_token',
    },
    {
      id: 'screen_override',
      label: '页面覆盖',
      path: `.elon/ui-tuner/screens/${packageName}/${activity}.json`,
      intent: '只属于当前页面的一次性位置或尺寸调整。',
      recommended: recommendedScope === 'screen_override',
    },
  ]
}

function inferRecommendedScope(element: UiTunerElement): UiTunerStandardScope {
  if (!element.runtime) return 'local_draft'
  if (!element.source?.file && !element.runtime.resourceId) return 'screen_override'
  if (element.kind === 'text') return 'design_token'
  return 'project_component'
}

function inferComponentName(element: UiTunerElement) {
  const className = (element.runtime?.className ?? '').toLowerCase()
  if (element.kind === 'button' || className.includes('button')) return 'Button'
  if (className.includes('toolbar') || /top|title|bar/i.test(element.name)) return 'TopBar'
  if (/bottom|nav|tab/i.test(element.name) || className.includes('tab')) return 'BottomNavigation'
  if (element.kind === 'text' || className.includes('text')) return 'Typography'
  if (element.kind === 'media' || className.includes('image')) return 'Media'
  return 'SurfaceCard'
}

function inferRole(element: UiTunerElement) {
  return lastResourceName(element.runtime?.resourceId ?? element.source?.token) || element.name || element.kind
}

function inferVariant(element: UiTunerElement) {
  if (element.kind === 'button') return element.background.includes('76, 175') ? 'primary' : 'action'
  if (element.kind === 'text') return element.fontWeight >= 700 ? 'title' : 'body'
  if (element.kind === 'card') return element.borderWidth > 0 ? 'outlined' : 'filled'
  return 'default'
}

function inferTokenRefs(element: UiTunerElement): UiTunerElementStandard['tokenRefs'] {
  return {
    color: tokenName('color', element.color),
    background: tokenName('surface', element.background),
    typography: `type.${inferVariant(element)}.${element.fontSize}`,
    spacing: `space.${Math.max(element.paddingX, element.paddingY)}`,
    radius: `radius.${element.borderRadius}`,
  }
}

function buildReuseKey(
  document: UiTunerDocument,
  component: string,
  role: string,
) {
  const scope = document.runtimeSnapshot?.packageName ?? document.source?.signature ?? 'manual'
  return `${scope}/${component}/${role}`
}

function inferNote(element: UiTunerElement) {
  if (element.source?.file) return '已有源码映射，适合由 CLI 回写 XML 或 values token。'
  if (element.runtime?.resourceId) return '已有 resourceId，建议先在 Android res/layout 中反查源码。'
  return '缺少稳定源码标识，建议只作为当前页面覆盖。'
}

function inferBindingConfidence(element: UiTunerElement): UiTunerStandardInsight['bindingConfidence'] {
  if (element.source?.file) return 'high'
  if (element.runtime?.resourceId) return 'medium'
  return 'low'
}

function inferBindingReason(element: UiTunerElement) {
  if (element.source?.file) return `源码已定位到 ${element.source.file}${element.source.line ? `:${element.source.line}` : ''}`
  if (element.runtime?.resourceId) return `可通过 resourceId 反查：${element.runtime.resourceId}`
  return '只有截图/XML 坐标，缺少稳定源码绑定。'
}

function reusableFields(element: UiTunerElement) {
  const fields = ['颜色', '字号', '字重', '内距', '圆角']
  if (element.kind === 'button' || element.kind === 'card') fields.push('背景', '边框')
  return fields
}

function screenOnlyFields() {
  return ['X/Y 坐标', '与父容器约束', '当前截图中的绝对宽高']
}

function tokenName(prefix: string, value: string) {
  const safeValue = value
    .replace(/[^a-zA-Z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 24)
    .toLowerCase()
  return `${prefix}.${safeValue || 'default'}`
}

function lastResourceName(resourceId?: string) {
  if (!resourceId) return ''
  const normalized = resourceId.replace(/.*(?:[:/]id\/|\+id\/)/, '')
  return normalized.split('/').pop() ?? normalized
}
