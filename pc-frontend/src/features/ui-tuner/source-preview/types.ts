export type SourcePreviewMode = 'headless' | 'source' | 'evidence'
export type SourcePreviewSaveState = 'preview' | 'saving' | 'saved' | 'error'
export type SourcePreviewBackend = 'android_layoutlib' | 'android_preview_host' | 'pwa_interactive' | 'react_twin'

export interface ComposePreviewEntry {
  id: string
  kotlinFile: string
  composable: string
  label: string
}

export interface SourceRendererCapabilities {
  ok: boolean
  recommendedBackend: SourcePreviewBackend
  layoutlib: { available: boolean; command?: string; detail: string }
  previewHost: { availableAfterDebugBuild: boolean; detail: string }
  pwaPreview: { available: boolean; url?: string; detail: string }
  reactTwin: { available: boolean; authoritative: boolean; detail: string }
  composePreviews: ComposePreviewEntry[]
}

export interface ComposePreviewRender {
  ok: boolean
  backend: 'android_layoutlib'
  authoritative: true
  kotlinFile: string
  composable: string
  dataUrl: string
  semanticsText: string
}

export interface SourcePreviewDocument {
  ok: boolean
  irKind: 'elon.source_ui_ir'
  irVersion: number
  projectRoot: string
  layoutFiles: string[]
  selectedLayout: string
  sourceRevision: string
  rendering: {
    backend: 'react_twin'
    authoritative: false
    sourceOfTruth: 'android_source'
    calibrationRequired: true
  }
  fidelity?: SourcePreviewFidelity
  canvas: { width: number; height: number; background: string }
  root: SourcePreviewNode
}

export interface SourcePreviewFidelity {
  score: number
  level: 'high' | 'medium' | 'low'
  safeForDefaultPreview: boolean
  totalNodes: number
  unsupportedNodes: number
  dynamicNodes: number
  issues: string[]
}
export interface SourcePreviewNode {
  key: string
  resourceId?: string
  tag: string
  name: string
  kind: 'group' | 'button' | 'text' | 'input' | 'image' | 'list' | 'spacer' | string
  source: {
    layoutFile: string
    startTagStart: number
    startTagEnd: number
    attributes: Record<string, string>
  }
  layout: {
    mode: 'flow' | 'stack' | 'leaf' | string
    orientation: 'row' | 'column'
    width: string
    height: string
    weight: number
    gravity: string
    margin: SourcePreviewEdges
    padding: SourcePreviewEdges
    gap: number
  }
  style: {
    text: string
    textColor: string
    background: string
    fontSize: number
    fontWeight: number
    borderRadius: number
    opacity: number
    visible: boolean
    contentDescription: string
  }
  editable: string[]
  children: SourcePreviewNode[]
}

export interface SourcePreviewEdges {
  start: number
  top: number
  end: number
  bottom: number
}

export interface SourcePreviewPatch {
  nodeKey: string
  property: string
  value: string | number | boolean
}

export interface PendingSourceNodePatch {
  nodeKey: string
  startTagStart: number
  startTagEnd: number
  changes: Record<string, string>
}
