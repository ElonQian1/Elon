export type SourcePreviewMode = 'source' | 'evidence'
export type SourcePreviewSaveState = 'preview' | 'saving' | 'saved' | 'error'

export interface SourcePreviewDocument {
  ok: boolean
  projectRoot: string
  layoutFiles: string[]
  selectedLayout: string
  sourceRevision: string
  canvas: { width: number; height: number; background: string }
  root: SourcePreviewNode
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
