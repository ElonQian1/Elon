export const PWA_DESIGN_DRAFT_SCHEMA_VERSION = 1 as const

export type PwaMappingConfidence = 'high' | 'medium' | 'low'

export type PwaStyleProperty =
  | 'width' | 'height'
  | 'paddingTop' | 'paddingRight' | 'paddingBottom' | 'paddingLeft'
  | 'marginTop' | 'marginRight' | 'marginBottom' | 'marginLeft'
  | 'borderRadius' | 'fontSize' | 'fontWeight' | 'lineHeight'
  | 'color' | 'backgroundColor' | 'opacity'

export type PwaStyleValues = Partial<Record<PwaStyleProperty, string>>

export interface PwaElementIdentity {
  key: string
  selector: string
  strategy: 'data-ui-node' | 'id' | 'data-attribute' | 'aria-label' | 'dom-path'
  confidence: PwaMappingConfidence
  confidenceScore: number
  needsBinding: boolean
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

export interface PwaDraftElement {
  identity: PwaElementIdentity
  originalStyle: PwaOriginalStyleSnapshot
  styleDiff: PwaStyleValues
  revision: number
  updatedAt: string
}

export interface PwaDesignDraft {
  schemaVersion: typeof PWA_DESIGN_DRAFT_SCHEMA_VERSION
  kind: 'elon.pwa.manual_style_draft'
  project: {
    id: string
    workspaceIdentity: string
    sourceRevision: string
  }
  route: { path: string; search: string; hash: string }
  viewport: { width: number; height: number }
  elements: Record<string, PwaDraftElement>
  revision: number
  updatedAt: string
}

export interface PwaRouteIdentity {
  path: string
  search: string
  hash: string
  viewport: { width: number; height: number }
}

const STORAGE_PREFIX = 'elon.pc.pwaDesignDraft.v1:'

function normalizedSearch(search: string): string {
  const params = new URLSearchParams(search)
  params.delete('ui_tuner_preview')
  const value = params.toString()
  return value ? `?${value}` : ''
}

export function normalizePwaRoute(route: PwaRouteIdentity): PwaRouteIdentity {
  return {
    path: route.path || '/web',
    search: normalizedSearch(route.search || ''),
    hash: route.hash || '',
    viewport: {
      width: Math.max(1, Math.round(route.viewport.width)),
      height: Math.max(1, Math.round(route.viewport.height)),
    },
  }
}

export function createPwaDesignDraft(
  project: PwaDesignDraft['project'],
  routeInput: PwaRouteIdentity,
): PwaDesignDraft {
  const route = normalizePwaRoute(routeInput)
  return {
    schemaVersion: PWA_DESIGN_DRAFT_SCHEMA_VERSION,
    kind: 'elon.pwa.manual_style_draft',
    project,
    route: { path: route.path, search: route.search, hash: route.hash },
    viewport: route.viewport,
    elements: {},
    revision: 0,
    updatedAt: new Date().toISOString(),
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
    `${route.viewport.width}x${route.viewport.height}`,
  ].join('|'))
}

export function readPwaDesignDraft(
  project: PwaDesignDraft['project'],
  route: PwaRouteIdentity,
): PwaDesignDraft | null {
  try {
    const parsed = JSON.parse(window.localStorage.getItem(pwaDraftStorageKey(project, route)) || 'null') as PwaDesignDraft | null
    if (!parsed || parsed.schemaVersion !== PWA_DESIGN_DRAFT_SCHEMA_VERSION || parsed.kind !== 'elon.pwa.manual_style_draft') return null
    return parsed
  } catch {
    return null
  }
}

export function savePwaDesignDraft(draft: PwaDesignDraft): void {
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
  try {
    window.localStorage.removeItem(pwaDraftStorageKey(draft.project, {
      ...draft.route,
      viewport: draft.viewport,
    }))
  } catch {
    // Local persistence is best effort when storage is disabled.
  }
}
