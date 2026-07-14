import type {
  LivePatchAck,
  LivePatchOperation,
  LivePropertyValue,
  LiveUiFrame,
  LiveUiNode,
} from './liveUiApi'

export type RuntimeDraftPhase = 'local' | 'syncing' | 'acked' | 'rejected'

export interface RuntimeDraftRect {
  left: number
  top: number
  width: number
  height: number
}

export interface RuntimeDraftVisual {
  rect: RuntimeDraftRect
  baseRect: RuntimeDraftRect
  background: string
  color: string
  borderColor: string
  borderWidth: number
  borderRadius: number
  opacity: number
  paddingTop: number
  paddingRight: number
  paddingBottom: number
  paddingLeft: number
  fontSize: number
  lineHeight: number
  fontWeight: number
  letterSpacing: number
  text: string
  visible: boolean
}

export interface RuntimeDraftNode {
  runtimeNodeId: string
  definitionId: string
  kind: string
  localRevision: number
  confirmedRevision: number
  phase: RuntimeDraftPhase
  operations: Record<string, LivePropertyValue>
  visual: RuntimeDraftVisual
  baseFrameCapturedAt: string
  error?: string
}

export interface RuntimeDraftState {
  revision: number
  nodes: Record<string, RuntimeDraftNode>
}

export const EMPTY_RUNTIME_DRAFT_STATE: RuntimeDraftState = {
  revision: 0,
  nodes: {},
}

export function applyRuntimeDraftOperations(
  state: RuntimeDraftState,
  node: LiveUiNode,
  operations: LivePatchOperation[],
  frame: LiveUiFrame | null,
): RuntimeDraftState {
  if (operations.length === 0) return state
  const revision = state.revision + 1
  const current = state.nodes[node.runtimeNodeId]
  const values = {
    ...(current?.operations ?? {}),
    ...Object.fromEntries(operations.map((operation) => [operation.property, operation.value])),
  }
  return {
    revision,
    nodes: {
      ...state.nodes,
      [node.runtimeNodeId]: {
        runtimeNodeId: node.runtimeNodeId,
        definitionId: node.definitionId,
        kind: node.kind,
        localRevision: revision,
        confirmedRevision: current?.confirmedRevision ?? 0,
        phase: 'local',
        operations: values,
        visual: projectRuntimeVisual(node, values),
        baseFrameCapturedAt: current?.baseFrameCapturedAt ?? frame?.capturedAt ?? '',
      },
    },
  }
}

export function markRuntimeDraftSyncing(
  state: RuntimeDraftState,
  runtimeNodeId: string,
  revision: number,
): RuntimeDraftState {
  return updateIfCurrent(state, runtimeNodeId, revision, (draft) => ({
    ...draft,
    phase: 'syncing',
    error: undefined,
  }))
}

export function acknowledgeRuntimeDraft(
  state: RuntimeDraftState,
  runtimeNodeId: string,
  revision: number,
  ack: LivePatchAck,
): RuntimeDraftState {
  return updateIfCurrent(state, runtimeNodeId, revision, (draft) => ({
    ...draft,
    confirmedRevision: Math.max(draft.confirmedRevision, revision),
    phase: ack.status === 'APPLIED' ? 'acked' : 'rejected',
    error: ack.status === 'APPLIED' ? undefined : ack.error || 'Android 拒绝了本次修改',
  }))
}

export function rejectRuntimeDraft(
  state: RuntimeDraftState,
  runtimeNodeId: string,
  revision: number,
  error: string,
): RuntimeDraftState {
  return updateIfCurrent(state, runtimeNodeId, revision, (draft) => ({
    ...draft,
    phase: 'rejected',
    error,
  }))
}

export function confirmRuntimeDraftFrame(
  state: RuntimeDraftState,
  frame: LiveUiFrame | null,
): RuntimeDraftState {
  if (!frame) return state
  const nodes = Object.fromEntries(Object.entries(state.nodes).filter(([, draft]) => !(
    draft.phase === 'acked'
    && draft.confirmedRevision === draft.localRevision
    && draft.baseFrameCapturedAt !== frame.capturedAt
  )))
  return Object.keys(nodes).length === Object.keys(state.nodes).length ? state : { ...state, nodes }
}

export function runtimeDraftStatus(state: RuntimeDraftState) {
  const drafts = Object.values(state.nodes)
  if (drafts.some((draft) => draft.phase === 'rejected')) return 'rejected' as const
  if (drafts.some((draft) => draft.phase === 'local')) return 'local' as const
  if (drafts.some((draft) => draft.phase === 'syncing')) return 'syncing' as const
  if (drafts.some((draft) => draft.phase === 'acked')) return 'calibrating' as const
  return 'confirmed' as const
}

export function projectRuntimeVisual(
  node: LiveUiNode,
  overrides: Record<string, LivePropertyValue>,
): RuntimeDraftVisual {
  const density = positive(node.geometry.density, 1)
  const scaledDensity = density * positive(node.geometry.fontScale, 1)
  const bounds = node.geometry.boundsInDisplayPx
  const baseRect = {
    left: bounds.left,
    top: bounds.top,
    width: bounds.width,
    height: bounds.height,
  }
  const baseTranslationX = numericProperty(node, {}, 'translationX', 0)
  const baseTranslationY = numericProperty(node, {}, 'translationY', 0)
  const translationX = numericProperty(node, overrides, 'translationX', baseTranslationX)
  const translationY = numericProperty(node, overrides, 'translationY', baseTranslationY)
  const width = dimensionPx(node, overrides, 'width', baseRect.width, density)
  const height = dimensionPx(node, overrides, 'height', baseRect.height, density)
  const fontSize = numericProperty(node, overrides, 'textSize', Math.max(1, baseRect.height / scaledDensity * 0.45))
  const lineHeight = numericProperty(node, overrides, 'lineHeight', fontSize * 1.25)

  return {
    baseRect,
    rect: {
      left: baseRect.left + ((translationX - baseTranslationX) * density),
      top: baseRect.top + ((translationY - baseTranslationY) * density),
      width,
      height,
    },
    background: colorProperty(node, overrides, 'backgroundColor', 'transparent'),
    color: colorProperty(node, overrides, 'contentColor', '#ffffff'),
    borderColor: colorProperty(node, overrides, 'borderColor', 'transparent'),
    borderWidth: numericProperty(node, overrides, 'borderWidth', 0) * density,
    borderRadius: numericProperty(node, overrides, 'cornerRadius.all', 0) * density,
    opacity: clamp(numericProperty(node, overrides, 'opacity', 1), 0, 1),
    paddingTop: numericProperty(node, overrides, 'padding.top', 0) * density,
    paddingRight: numericProperty(node, overrides, 'padding.end', 0) * density,
    paddingBottom: numericProperty(node, overrides, 'padding.bottom', 0) * density,
    paddingLeft: numericProperty(node, overrides, 'padding.start', 0) * density,
    fontSize: fontSize * scaledDensity,
    lineHeight: lineHeight * scaledDensity,
    fontWeight: numericProperty(node, overrides, 'fontWeight', 500),
    letterSpacing: numericProperty(node, overrides, 'letterSpacing', 0) * scaledDensity,
    text: stringProperty(node, overrides, 'text', node.text ?? ''),
    visible: booleanProperty(node, overrides, 'visibility', true),
  }
}

export function nearestRuntimeSurfaceColor(
  node: LiveUiNode,
  nodes: LiveUiNode[],
  fallback: string,
) {
  const byId = new Map(nodes.map((candidate) => [candidate.runtimeNodeId, candidate]))
  let current = node.parentRuntimeNodeId ? byId.get(node.parentRuntimeNodeId) : undefined
  while (current) {
    const color = colorProperty(current, {}, 'backgroundColor', '')
    if (color && color !== 'transparent' && color !== '#00000000') return color
    current = current.parentRuntimeNodeId ? byId.get(current.parentRuntimeNodeId) : undefined
  }
  return fallback
}

function updateIfCurrent(
  state: RuntimeDraftState,
  runtimeNodeId: string,
  revision: number,
  update: (draft: RuntimeDraftNode) => RuntimeDraftNode,
) {
  const draft = state.nodes[runtimeNodeId]
  if (!draft || revision < draft.localRevision) return state
  return { ...state, nodes: { ...state.nodes, [runtimeNodeId]: update(draft) } }
}

function dimensionPx(
  node: LiveUiNode,
  overrides: Record<string, LivePropertyValue>,
  property: string,
  fallback: number,
  density: number,
) {
  const value = propertyValue(node, overrides, property)
  if (!value || typeof value.value !== 'number' || !Number.isFinite(value.value)) return fallback
  return Math.max(1, value.value * density)
}

function numericProperty(
  node: LiveUiNode,
  overrides: Record<string, LivePropertyValue>,
  property: string,
  fallback: number,
) {
  const value = propertyValue(node, overrides, property)?.value
  const number = typeof value === 'number' ? value : Number(value)
  return Number.isFinite(number) ? number : fallback
}

function stringProperty(
  node: LiveUiNode,
  overrides: Record<string, LivePropertyValue>,
  property: string,
  fallback: string,
) {
  const value = propertyValue(node, overrides, property)?.value
  return typeof value === 'string' ? value : fallback
}

function colorProperty(
  node: LiveUiNode,
  overrides: Record<string, LivePropertyValue>,
  property: string,
  fallback: string,
) {
  return normalizeCssColor(stringProperty(node, overrides, property, fallback))
}

function booleanProperty(
  node: LiveUiNode,
  overrides: Record<string, LivePropertyValue>,
  property: string,
  fallback: boolean,
) {
  const value = propertyValue(node, overrides, property)?.value
  if (typeof value === 'boolean') return value
  if (typeof value === 'string') return !['false', 'gone', 'invisible', 'hidden'].includes(value.toLowerCase())
  return fallback
}

function propertyValue(
  node: LiveUiNode,
  overrides: Record<string, LivePropertyValue>,
  property: string,
) {
  return overrides[property] ?? node.properties[property]?.effective
}

function normalizeCssColor(value: string) {
  return /^#[0-9a-f]{8}$/i.test(value) ? `#${value.slice(3)}${value.slice(1, 3)}` : value
}

function positive(value: number, fallback: number) {
  return Number.isFinite(value) && value > 0 ? value : fallback
}

function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(maximum, Math.max(minimum, value))
}
