import type { UiTunerDocument, UiTunerElement, UiTunerRuntimeStyle, UiTunerSource } from '../types'
import type { LiveUiFrame, LiveUiNode, LiveUiSession } from './liveUiApi'

const MAX_LIVE_ELEMENTS = 320

export function runtimeNodesToTunerDocument(
  current: UiTunerDocument,
  session: LiveUiSession,
  nodes: LiveUiNode[],
  frame: LiveUiFrame,
): UiTunerDocument {
  const sourceByResource = new Map<string, { source?: UiTunerSource; candidates?: UiTunerSource[] }>()
  for (const element of current.elements) {
    const resource = comparableResourceId(element.runtime?.resourceId)
    if (resource) sourceByResource.set(resource, { source: element.source, candidates: element.sourceCandidates })
  }

  const elements = nodes
    .filter(isUsefulRuntimeNode)
    .slice(0, MAX_LIVE_ELEMENTS)
    .map((node, index) => nodeToElement(node, index, session.packageName, sourceByResource))

  return {
    ...current,
    canvas: {
      ...current.canvas,
      name: `${session.deviceId} · Android 真实渲染`,
      width: Math.max(280, frame.width),
      height: Math.max(360, frame.height),
      background: '#000000',
      source: {
        kind: 'device_snapshot',
        label: `${session.packageName} Android Runtime`,
        signature: `${session.id}:${session.treeRevision}:${frame.capturedAt}`,
        files: session.projectRoot ? [session.projectRoot] : undefined,
      },
    },
    elements,
    source: {
      kind: 'device_snapshot',
      label: `${session.packageName} Android Runtime`,
      signature: `${session.id}:${session.treeRevision}`,
      files: session.projectRoot ? [session.projectRoot] : undefined,
    },
    runtimeSnapshot: {
      ...current.runtimeSnapshot,
      deviceId: session.deviceId,
      packageName: session.packageName,
      activityName: nodes[0]?.screenId,
      capturedAt: frame.capturedAt,
      nodeCount: nodes.length,
      sourceRoot: session.projectRoot ?? current.runtimeSnapshot?.sourceRoot,
    },
    updatedAt: new Date().toISOString(),
  }
}

export function preferredRuntimeSelection(
  previous: UiTunerElement | null,
  elements: UiTunerElement[],
): string | null {
  if (previous) {
    const sameRuntime = elements.find((element) => element.runtime?.nodeId === previous.runtime?.nodeId)
    if (sameRuntime) return sameRuntime.id
    const resource = comparableResourceId(previous.runtime?.resourceId)
    const sameResource = resource && elements.find(
      (element) => comparableResourceId(element.runtime?.resourceId) === resource,
    )
    if (sameResource) return sameResource.id
    const definitionId = previous.runtime?.xpath?.trim()
    const sameDefinition = definitionId && elements.find(
      (element) => element.runtime?.xpath?.trim() === definitionId,
    )
    if (sameDefinition) return sameDefinition.id
  }
  return elements.find((element) => element.runtime?.resourceId)?.id ?? elements[0]?.id ?? null
}

function isUsefulRuntimeNode(node: LiveUiNode) {
  const bounds = node.geometry.boundsInDisplayPx
  if (!node.geometry.visible || bounds.width < 4 || bounds.height < 4) return false
  if (node.resourceId || node.text?.trim()) return true
  return !node.kind.includes('container') && !/Layout|ViewGroup|DecorView/i.test(node.className)
}

function nodeToElement(
  node: LiveUiNode,
  index: number,
  packageName: string,
  sourceByResource: Map<string, { source?: UiTunerSource; candidates?: UiTunerSource[] }>,
): UiTunerElement {
  const bounds = node.geometry.boundsInDisplayPx
  const resource = comparableResourceId(node.resourceId)
  const remembered = resource ? sourceByResource.get(resource) : undefined
  const style = runtimeStyle(node)
  const label = node.text?.trim() || resource || simpleClassName(node.className) || `运行时节点 ${index + 1}`
  return {
    id: `live-${node.runtimeNodeId}`,
    name: label,
    kind: nodeKind(node),
    x: bounds.left,
    y: bounds.top,
    width: Math.max(8, bounds.width),
    height: Math.max(8, bounds.height),
    text: node.text ?? label,
    ...style,
    source: remembered?.source ?? {
      kind: 'runtime_xml',
      label: node.resourceId ? 'Android Runtime · 稳定 resource-id' : 'Android Runtime · 运行时节点',
      token: node.definitionId,
      rawValue: node.resourceId ?? node.runtimeNodeId,
    },
    sourceCandidates: remembered?.candidates ?? [],
    runtime: {
      nodeId: node.runtimeNodeId,
      resourceId: node.resourceId,
      className: node.className,
      packageName,
      xpath: node.definitionId,
      indexPath: [],
      originalBounds: bounds,
      originalStyle: style,
    },
  }
}

function runtimeStyle(node: LiveUiNode): UiTunerRuntimeStyle {
  const start = numeric(node, 'padding.start')
  const end = numeric(node, 'padding.end')
  const top = numeric(node, 'padding.top')
  const bottom = numeric(node, 'padding.bottom')
  const fontSize = numeric(node, 'textSize', 13)
  return {
    fontSize,
    lineHeight: numeric(
      node,
      'lineHeight',
      Math.max(fontSize + 4, Math.round(node.geometry.boundsInDisplayPx.height / node.geometry.density)),
    ),
    fontWeight: numeric(node, 'fontWeight', 500),
    letterSpacing: numeric(node, 'letterSpacing'),
    paddingX: Math.round((start + end) / 2),
    paddingY: Math.round((top + bottom) / 2),
    borderRadius: numeric(node, 'cornerRadius.all'),
    borderWidth: numeric(node, 'borderWidth'),
    color: stringValue(node, 'contentColor', '#ffffff'),
    background: stringValue(node, 'backgroundColor', 'transparent'),
    borderColor: stringValue(node, 'borderColor', 'transparent'),
    opacity: numeric(node, 'opacity', 1),
  }
}

function numeric(node: LiveUiNode, property: string, fallback = 0) {
  const raw = node.properties[property]?.effective?.value
  const value = typeof raw === 'number' ? raw : Number(raw)
  return Number.isFinite(value) ? value : fallback
}

function stringValue(node: LiveUiNode, property: string, fallback: string) {
  const raw = node.properties[property]?.effective?.value
  return typeof raw === 'string' && raw ? normalizeColor(raw) : fallback
}

function normalizeColor(value: string) {
  return /^#[0-9a-f]{8}$/i.test(value) ? `#${value.slice(3)}${value.slice(1, 3)}` : value
}

function nodeKind(node: LiveUiNode): UiTunerElement['kind'] {
  const value = `${node.kind} ${node.className}`.toLowerCase()
  if (value.includes('image')) return 'media'
  if (value.includes('button') || value.includes('tab')) return 'button'
  if (value.includes('text')) return 'text'
  return 'card'
}

function comparableResourceId(value?: string) {
  return value?.trim().replace(/^.*:id\//, '').replace(/^.*\/id\//, '') || ''
}

function simpleClassName(value: string) {
  return value.split('.').pop() ?? value
}
