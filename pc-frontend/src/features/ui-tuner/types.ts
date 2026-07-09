export type UiTunerElementKind = 'text' | 'card' | 'button' | 'media'

export interface UiTunerCanvas {
  name: string
  width: number
  height: number
  background: string
  referenceImage?: UiTunerReferenceImage
  source?: UiTunerSource
}

export interface UiTunerReferenceImage {
  dataUrl: string
  name: string
  width: number
  height: number
  opacity: number
  visible: boolean
}

export interface UiTunerSource {
  kind: 'apk' | 'manual'
  label: string
  file?: string
  line?: number
  token?: string
  rawValue?: string
  signature?: string
  files?: string[]
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
  source?: UiTunerSource
}

export interface UiTunerDocument {
  version: 1
  canvas: UiTunerCanvas
  elements: UiTunerElement[]
  updatedAt: string
  source?: UiTunerSource
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
  source?: UiTunerSource
}
