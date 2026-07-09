import type { AndroidInspectorNode, AndroidInspectorSnapshot } from '../device/deviceInspectorApi'
import type { UiTunerDocument, UiTunerElement, UiTunerElementKind, UiTunerSource } from '../types'

const MAX_RUNTIME_ELEMENTS = 180

export function snapshotToTunerDocument(snapshot: AndroidInspectorSnapshot): UiTunerDocument {
  const screenshot = snapshot.screenshot
  const width = Math.max(280, screenshot?.width ?? maxRight(snapshot.nodes))
  const height = Math.max(360, screenshot?.height ?? maxBottom(snapshot.nodes))
  const elements = usefulNodes(snapshot.nodes)
    .slice(0, MAX_RUNTIME_ELEMENTS)
    .map((node, index) => nodeToElement(node, index))

  return {
    version: 1,
    canvas: {
      name: `${snapshot.deviceId} 真机快照`,
      width,
      height,
      background: '#000000',
      referenceImage: screenshot?.dataUrl
        ? {
            dataUrl: screenshot.dataUrl,
            name: `${snapshot.deviceId}-${new Date(snapshot.capturedAt).toLocaleString()}.png`,
            width: screenshot.width,
            height: screenshot.height,
            opacity: 1,
            visible: true,
          }
        : undefined,
      source: snapshotSource(snapshot),
    },
    elements,
    source: snapshotSource(snapshot),
    runtimeSnapshot: {
      deviceId: snapshot.deviceId,
      packageName: snapshot.packageName,
      activityName: snapshot.activityName,
      capturedAt: snapshot.capturedAt,
      nodeCount: snapshot.xml.nodeCount,
      sourceRoot: snapshot.sourceRoot,
    },
    updatedAt: new Date().toISOString(),
  }
}

function usefulNodes(nodes: AndroidInspectorNode[]): AndroidInspectorNode[] {
  return nodes.filter((node) => {
    if (!node.visible || !node.enabled) return false
    if (node.bounds.width < 4 || node.bounds.height < 4) return false
    if (node.password) return false
    return Boolean(node.resourceId || node.text || node.contentDesc || node.clickable || node.scrollable)
  })
}

function nodeToElement(node: AndroidInspectorNode, index: number): UiTunerElement {
  const label = node.text || node.contentDesc || lastResourceName(node.resourceId) || node.className || `节点 ${index + 1}`
  const kind = nodeKind(node)
  return {
    id: `runtime-${node.id}`,
    name: label,
    kind,
    x: node.bounds.left,
    y: node.bounds.top,
    width: Math.max(node.bounds.width, 8),
    height: Math.max(node.bounds.height, 8),
    text: label,
    fontSize: kind === 'text' ? 13 : 12,
    lineHeight: kind === 'text' ? 18 : 16,
    fontWeight: node.clickable ? 700 : 500,
    letterSpacing: 0,
    paddingX: 4,
    paddingY: 2,
    borderRadius: kind === 'button' ? 8 : 4,
    borderWidth: node.resourceId ? 1 : 0,
    color: node.text || node.contentDesc ? '#f4f7fb' : 'rgba(244, 247, 251, 0.68)',
    background: node.clickable ? 'rgba(76, 175, 120, 0.13)' : 'rgba(168, 199, 250, 0.08)',
    borderColor: node.source ? '#a8c7fa' : 'rgba(255, 255, 255, 0.24)',
    opacity: 1,
    source: node.source
      ? {
          kind: 'runtime_xml',
          label: node.source.reason,
          file: node.source.file,
          line: node.source.line,
          token: node.source.token,
          rawValue: node.resourceId,
        }
      : {
          kind: 'runtime_xml',
          label: node.resourceId ? 'runtime resource-id' : 'runtime XML node',
          token: node.resourceId,
          rawValue: node.xpath,
        },
    runtime: {
      nodeId: node.id,
      resourceId: node.resourceId,
      className: node.className,
      packageName: node.packageName,
      xpath: node.xpath,
      indexPath: node.indexPath,
      originalBounds: node.bounds,
    },
  }
}

function nodeKind(node: AndroidInspectorNode): UiTunerElementKind {
  const className = (node.className ?? '').toLowerCase()
  if (className.includes('image')) return 'media'
  if (node.clickable || className.includes('button')) return 'button'
  if (className.includes('text')) return 'text'
  return 'card'
}

function snapshotSource(snapshot: AndroidInspectorSnapshot): UiTunerSource {
  return {
    kind: 'device_snapshot',
    label: `${snapshot.packageName ?? 'APK'} 真机快照`,
    signature: `${snapshot.deviceId}:${snapshot.capturedAt}:${snapshot.xml.nodeCount}`,
    rawValue: snapshot.activityName,
    files: snapshot.sourceRoot ? [snapshot.sourceRoot] : undefined,
  }
}

function lastResourceName(resourceId?: string): string {
  if (!resourceId) return ''
  const normalized = resourceId.replace(/.*(?:[:/]id\/|\+id\/)/, '')
  return normalized.split('/').pop() ?? normalized
}

function maxRight(nodes: AndroidInspectorNode[]): number {
  return Math.max(...nodes.map((node) => node.bounds.right), 280)
}

function maxBottom(nodes: AndroidInspectorNode[]): number {
  return Math.max(...nodes.map((node) => node.bounds.bottom), 360)
}
