export const PWA_DESIGN_DRAFT_SCHEMA_VERSION = 2 as const
export const PWA_DESIGN_ARTIFACT_VERSION = 'elon.pwa.cross-platform-draft.v2' as const

export type PwaMappingConfidence = 'high' | 'medium' | 'low'
export type PwaDesignScope = 'instance' | 'component' | 'route' | 'project'

export type PwaStyleProperty =
  | 'width' | 'height'
  | 'paddingTop' | 'paddingRight' | 'paddingBottom' | 'paddingLeft'
  | 'marginTop' | 'marginRight' | 'marginBottom' | 'marginLeft'
  | 'borderRadius' | 'fontSize' | 'fontWeight' | 'lineHeight'
  | 'color' | 'backgroundColor' | 'opacity'

export type PwaStyleValues = Partial<Record<PwaStyleProperty, string>>

export type PwaStyleBindingKind = 'css-rule' | 'style-object' | 'token-json'

export interface PwaExplicitStyleBinding {
  version: 1
  sourceFile: string
  sourceRevision: string
  kind: PwaStyleBindingKind
  target: string
  range: { start: number; end: number }
  propertyMap: Partial<Record<PwaStyleProperty, string>>
}

export interface PwaPropertyWritebackReceipt {
  value: string
  sourceFile: string
  sourceRevision: string
  completedAt: string
}

export interface PwaElementWritebackReceipts {
  pwa?: Partial<Record<PwaStyleProperty, PwaPropertyWritebackReceipt>>
  android?: Partial<Record<PwaStyleProperty, PwaPropertyWritebackReceipt>>
}

export interface PwaElementIdentity {
  key: string
  selector: string
  strategy: 'data-ui-node' | 'id' | 'data-attribute' | 'aria-label' | 'dom-path'
  confidence: PwaMappingConfidence
  confidenceScore: number
  needsBinding: boolean
  stableId: string
  testId: string
  resourceId: string
  sourceSymbol: string
  componentPath: string
  uiNode: string
  id: string
  ariaLabel: string
  role: string
  text: string
  tag: string
  classNames: string[]
}

export interface PwaOriginalStyleSnapshot {
  computed: PwaStyleValues
  authored: PwaStyleValues
  inlineStyle: string | null
}

export interface PwaDomContextNode {
  stableKey: string
  relation: 'parent' | 'self' | 'sibling'
  tag: string
  text: string
  role: string
}

export interface PwaSourceCandidate {
  platform: 'pwa' | 'android'
  stableKey: string
  file?: string
  symbol?: string
  componentPath?: string
  resourceId?: string
  confidence: number
  reason: string
}

export interface PwaSourceBinding {
  status: 'BOUND' | 'CANDIDATE' | 'NEEDS_AI'
  bindingConfidence: PwaMappingConfidence
  needsBinding: boolean
  pwaCandidates: PwaSourceCandidate[]
  androidCandidates: PwaSourceCandidate[]
  pwaStyle?: PwaExplicitStyleBinding
}

export interface PwaVisualReferences {
  screenshot?: string
  targetCrop?: string
  currentCrop?: string
  visualDiff?: string
}

export interface PwaDraftElement {
  identity: PwaElementIdentity
  originalStyle: PwaOriginalStyleSnapshot
  afterStyle: PwaStyleValues
  styleDiff: PwaStyleValues
  binding: PwaSourceBinding
  scope: PwaDesignScope
  domContext: PwaDomContextNode[]
  visualReferences: PwaVisualReferences
  writeback?: PwaElementWritebackReceipts
  revision: number
  createdAt: string
  updatedAt: string
}

export interface PwaDesignDraft {
  schemaVersion: typeof PWA_DESIGN_DRAFT_SCHEMA_VERSION
  artifactVersion: typeof PWA_DESIGN_ARTIFACT_VERSION
  kind: 'elon.pwa.manual_style_draft'
  project: {
    id: string
    workspaceIdentity: string
    sourceRevision: string
  }
  pageSource: {
    kind: 'authenticated-pwa'
    origin: string
    entryPath: string
    href?: string
    title?: string
  }
  route: {
    path: string
    search: string
    hash: string
    screenKey?: string
    screenTitle?: string
  }
  viewport: { width: number; height: number }
  scope: 'route'
  visualReferences: PwaVisualReferences
  elements: Record<string, PwaDraftElement>
  revision: number
  createdAt: string
  updatedAt: string
}

export interface PwaRouteIdentity {
  path: string
  search: string
  hash: string
  screenKey?: string
  screenTitle?: string
  href?: string
  title?: string
  viewport: { width: number; height: number }
}

export interface PwaDraftCliPackage {
  version: 1
  kind: 'elon_ui_tuner_pwa_cli_package'
  generatedAt: string
  capabilities: {
    PWA_CODE_GENERATION: true
    deterministicStyleWriteback: true
    codexStructuralFallback: true
  }
  artifact: PwaDesignDraft
  instructions: string[]
}

const PWA_STYLE_PROPERTIES = new Set<PwaStyleProperty>([
  'width', 'height',
  'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft',
  'marginTop', 'marginRight', 'marginBottom', 'marginLeft',
  'borderRadius', 'fontSize', 'fontWeight', 'lineHeight',
  'color', 'backgroundColor', 'opacity',
])

export function safePwaSourceFile(value: string): string | null {
  const normalized = String(value || '').trim().replace(/\\/g, '/')
  if (!normalized || normalized.length > 500 || normalized.startsWith('/') || /^[a-z]:\//i.test(normalized)) return null
  const segments = normalized.split('/')
  if (segments.some((segment) => !segment || segment === '.' || segment === '..' || segment.includes('\0'))) return null
  return normalized
}

export function normalizePwaExplicitStyleBinding(value: unknown): PwaExplicitStyleBinding | null {
  if (!value || typeof value !== 'object') return null
  const input = value as Partial<PwaExplicitStyleBinding>
  const sourceFile = safePwaSourceFile(String(input.sourceFile || ''))
  if (input.version !== 1 || !sourceFile || !/^[a-f0-9]{64}$/i.test(String(input.sourceRevision || ''))) return null
  if (!['css-rule', 'style-object', 'token-json'].includes(String(input.kind || ''))) return null
  const target = String(input.target || '').trim()
  const start = input.range?.start
  const end = input.range?.end
  if (!target || target.length > 240 || !Number.isSafeInteger(start) || !Number.isSafeInteger(end)
    || (start as number) < 0 || (end as number) <= (start as number)) return null
  if (!input.propertyMap || typeof input.propertyMap !== 'object') return null
  const propertyMap: Partial<Record<PwaStyleProperty, string>> = {}
  for (const [property, sourcePropertyValue] of Object.entries(input.propertyMap)) {
    if (!PWA_STYLE_PROPERTIES.has(property as PwaStyleProperty)) return null
    const sourceProperty = String(sourcePropertyValue || '').trim()
    if (!sourceProperty || sourceProperty.length > 160 || !/^[a-zA-Z_$][\w$.-]*$/.test(sourceProperty)) return null
    propertyMap[property as PwaStyleProperty] = sourceProperty
  }
  if (!Object.keys(propertyMap).length) return null
  return {
    version: 1,
    sourceFile,
    sourceRevision: String(input.sourceRevision).toLowerCase(),
    kind: input.kind as PwaStyleBindingKind,
    target,
    range: { start: start as number, end: end as number },
    propertyMap,
  }
}

const STORAGE_PREFIX = 'elon.pc.pwaDesignDraft.v1:'

function normalizedSearch(search: string): string {
  const params = new URLSearchParams(search)
  params.delete('ui_tuner_preview')
  const value = params.toString()
  return value ? `?${value}` : ''
}

function normalizedScreenValue(value: string | undefined, maxLength: number): string | undefined {
  const normalized = String(value || '').replace(/\s+/g, ' ').trim().slice(0, maxLength)
  return normalized || undefined
}

function hasIdentifiedScreen(route: Pick<PwaRouteIdentity, 'screenKey'>): boolean {
  return Boolean(route.screenKey && route.screenKey !== 'screen:unidentified')
}

export function normalizePwaRoute(route: PwaRouteIdentity): PwaRouteIdentity {
  return {
    path: route.path || '/web',
    search: normalizedSearch(route.search || ''),
    hash: route.hash || '',
    screenKey: normalizedScreenValue(route.screenKey, 240),
    screenTitle: normalizedScreenValue(route.screenTitle, 160),
    href: route.href,
    title: route.title,
    viewport: {
      width: Math.max(1, Math.round(route.viewport.width)),
      height: Math.max(1, Math.round(route.viewport.height)),
    },
  }
}

export function stablePwaIdentityKey(identity: Partial<PwaElementIdentity>): string {
  if (identity.stableId) return `stable:${identity.stableId}`
  if (identity.testId) return `test:${identity.testId}`
  if (identity.resourceId) return `resource:${identity.resourceId}`
  if (identity.sourceSymbol) return `symbol:${identity.sourceSymbol}`
  if (identity.componentPath) return `component:${identity.componentPath}`
  if (identity.uiNode) return `ui-node:${identity.uiNode}`
  if (identity.id) return `id:${identity.id}`
  return `selector-evidence:${identity.selector || identity.key || 'unknown'}`
}

export function resolvedPwaAfterStyle(
  original: PwaOriginalStyleSnapshot,
  diff: PwaStyleValues,
): PwaStyleValues {
  return { ...original.computed, ...original.authored, ...diff }
}

export function createPwaDesignDraft(
  project: PwaDesignDraft['project'],
  routeInput: PwaRouteIdentity,
): PwaDesignDraft {
  const route = normalizePwaRoute(routeInput)
  const now = new Date().toISOString()
  let origin = ''
  try { origin = new URL(route.href || route.path, window.location.origin).origin } catch { /* no browser origin */ }
  return {
    schemaVersion: PWA_DESIGN_DRAFT_SCHEMA_VERSION,
    artifactVersion: PWA_DESIGN_ARTIFACT_VERSION,
    kind: 'elon.pwa.manual_style_draft',
    project,
    pageSource: {
      kind: 'authenticated-pwa',
      origin,
      entryPath: route.path,
      href: route.href,
      title: route.screenTitle || route.title,
    },
    route: {
      path: route.path,
      search: route.search,
      hash: route.hash,
      screenKey: route.screenKey,
      screenTitle: route.screenTitle,
    },
    viewport: route.viewport,
    scope: 'route',
    visualReferences: {},
    elements: {},
    revision: 0,
    createdAt: now,
    updatedAt: now,
  }
}

export function pwaDraftStorageKey(project: PwaDesignDraft['project'], routeInput: PwaRouteIdentity): string {
  const route = normalizePwaRoute(routeInput)
  return STORAGE_PREFIX + encodeURIComponent([
    project.id || 'unknown-project',
    project.workspaceIdentity || 'unknown-workspace',
    route.path,
    route.search,
    route.hash,
    route.screenKey || 'screen:unidentified',
    `${route.viewport.width}x${route.viewport.height}`,
  ].join('|'))
}

export function parsePwaDesignDraft(value: string): PwaDesignDraft | null {
  try {
    const parsed = JSON.parse(value) as Record<string, unknown>
    if (parsed.kind !== 'elon.pwa.manual_style_draft') return null
    if (parsed.schemaVersion === PWA_DESIGN_DRAFT_SCHEMA_VERSION) return parsed as unknown as PwaDesignDraft
    if (parsed.schemaVersion === 1) return migrateDraftV1(parsed)
    return null
  } catch {
    return null
  }
}

export function readPwaDesignDraft(
  project: PwaDesignDraft['project'],
  route: PwaRouteIdentity,
): PwaDesignDraft | null {
  const normalizedRoute = normalizePwaRoute(route)
  if (!hasIdentifiedScreen(normalizedRoute)) return null
  try {
    const value = window.localStorage.getItem(pwaDraftStorageKey(project, normalizedRoute))
    const draft = value ? parsePwaDesignDraft(value) : null
    return draft?.route.screenKey === normalizedRoute.screenKey ? draft : null
  } catch {
    return null
  }
}

export function savePwaDesignDraft(draft: PwaDesignDraft): void {
  if (!hasIdentifiedScreen(draft.route)) return
  try {
    window.localStorage.setItem(pwaDraftStorageKey(draft.project, {
      ...draft.route,
      viewport: draft.viewport,
    }), JSON.stringify(draft))
  } catch {
    // Local persistence is best effort when storage is disabled or full.
  }
}

export function removePwaDesignDraft(draft: PwaDesignDraft): void {
  if (!hasIdentifiedScreen(draft.route)) return
  try {
    window.localStorage.removeItem(pwaDraftStorageKey(draft.project, {
      ...draft.route,
      viewport: draft.viewport,
    }))
  } catch {
    // Local persistence is best effort when storage is disabled.
  }
}

export function buildPwaDraftCliPackage(draft: PwaDesignDraft): PwaDraftCliPackage {
  return {
    version: 1,
    kind: 'elon_ui_tuner_pwa_cli_package',
    generatedAt: new Date().toISOString(),
    capabilities: {
      PWA_CODE_GENERATION: true,
      deterministicStyleWriteback: true,
      codexStructuralFallback: true,
    },
    artifact: draft,
    instructions: [
      '只读取 artifact 中的样式 diff、稳定身份、局部 DOM 上下文和来源候选，不默认扫描整仓库。',
      '先对明确绑定的 token、Style JSON、资源或属性执行确定性写回。',
      '只有结构调整、PWA 源码未绑定或复杂 Kotlin/TSX 才交给 Codex；Runtime DOM 不是源码真相。',
      '同时验证 PWA 与 APK 目标，并保持 sourceRevision 可追溯。',
    ],
  }
}

export function stringifyPwaDraftCliPackage(draft: PwaDesignDraft): string {
  return JSON.stringify(buildPwaDraftCliPackage(draft), null, 2)
}

function migrateDraftV1(value: Record<string, unknown>): PwaDesignDraft {
  const project = value.project as PwaDesignDraft['project']
  const route = value.route as PwaDesignDraft['route']
  const viewport = value.viewport as PwaDesignDraft['viewport']
  const updatedAt = typeof value.updatedAt === 'string' ? value.updatedAt : new Date().toISOString()
  const previousElements = (value.elements ?? {}) as Record<string, Partial<PwaDraftElement>>
  const elements = Object.values(previousElements).reduce<Record<string, PwaDraftElement>>((result, item) => {
    const identity = normalizeIdentity(item.identity ?? {})
    const originalStyle = item.originalStyle ?? { computed: {}, authored: {}, inlineStyle: null }
    const styleDiff = item.styleDiff ?? {}
    const key = stablePwaIdentityKey(identity)
    result[key] = {
      identity: { ...identity, key },
      originalStyle,
      afterStyle: resolvedPwaAfterStyle(originalStyle, styleDiff),
      styleDiff,
      binding: defaultBinding(identity),
      scope: 'instance',
      domContext: [],
      visualReferences: {},
      revision: item.revision ?? 1,
      createdAt: item.updatedAt ?? updatedAt,
      updatedAt: item.updatedAt ?? updatedAt,
    }
    return result
  }, {})
  return {
    schemaVersion: PWA_DESIGN_DRAFT_SCHEMA_VERSION,
    artifactVersion: PWA_DESIGN_ARTIFACT_VERSION,
    kind: 'elon.pwa.manual_style_draft',
    project,
    pageSource: { kind: 'authenticated-pwa', origin: '', entryPath: route.path },
    route,
    viewport,
    scope: 'route',
    visualReferences: {},
    elements,
    revision: typeof value.revision === 'number' ? value.revision : 0,
    createdAt: updatedAt,
    updatedAt,
  }
}

function normalizeIdentity(value: Partial<PwaElementIdentity>): PwaElementIdentity {
  const identity = {
    key: value.key ?? '', selector: value.selector ?? '', strategy: value.strategy ?? 'dom-path',
    confidence: value.confidence ?? 'low', confidenceScore: value.confidenceScore ?? 0.4,
    needsBinding: value.needsBinding ?? true, stableId: value.stableId ?? '', testId: value.testId ?? '',
    resourceId: value.resourceId ?? '', sourceSymbol: value.sourceSymbol ?? '',
    componentPath: value.componentPath ?? '', uiNode: value.uiNode ?? '', id: value.id ?? '',
    ariaLabel: value.ariaLabel ?? '', role: value.role ?? '', text: value.text ?? '',
    tag: value.tag ?? '', classNames: value.classNames ?? [],
  }
  return { ...identity, key: stablePwaIdentityKey(identity) }
}

function defaultBinding(identity: PwaElementIdentity): PwaSourceBinding {
  const pwaCandidates: PwaSourceCandidate[] = identity.key.startsWith('selector-evidence:') ? [] : [{
    platform: 'pwa', stableKey: identity.key, symbol: identity.sourceSymbol || undefined,
    componentPath: identity.componentPath || undefined, resourceId: identity.resourceId || identity.id || undefined,
    confidence: identity.confidenceScore, reason: '从真实 DOM 稳定身份生成的 PWA 源码反查候选',
  }]
  return {
    status: 'NEEDS_AI',
    bindingConfidence: identity.confidence,
    needsBinding: true,
    pwaCandidates,
    androidCandidates: [],
  }
}
