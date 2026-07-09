export type UiTunerElementKind = 'text' | 'card' | 'button' | 'media'

export interface UiTunerCanvas {
  name: string
  width: number
  height: number
  background: string
}

export interface UiTunerElement {
  id: string
  name: string
  kind: UiTunerElementKind
  x: number
  y: number
  width: number
  height: number
  text: string
  fontSize: number
  lineHeight: number
  fontWeight: number
  letterSpacing: number
  paddingX: number
  paddingY: number
  borderRadius: number
  borderWidth: number
  color: string
  background: string
  borderColor: string
  opacity: number
}

export interface UiTunerDocument {
  version: 1
  canvas: UiTunerCanvas
  elements: UiTunerElement[]
  updatedAt: string
}

export interface UiTunerExportElement {
  id: string
  name: string
  kind: UiTunerElementKind
  rect: {
    x: number
    y: number
    width: number
    height: number
  }
  text: string
  style: {
    fontSize: number
    lineHeight: number
    fontWeight: number
    letterSpacing: number
    paddingX: number
    paddingY: number
    borderRadius: number
    borderWidth: number
    color: string
    background: string
    borderColor: string
    opacity: number
  }
}
