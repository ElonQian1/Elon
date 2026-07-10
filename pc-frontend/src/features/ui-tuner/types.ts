export type UiTunerElementKind = 'text' | 'card' | 'button' | 'media'
export type UiTunerElementVisibility = 'visible' | 'hidden' | 'locked'
export type UiTunerStandardScope = 'local_draft' | 'screen_override' | 'project_component' | 'design_token'
export type UiTunerSelectionScope = 'instance' | 'component' | 'screen' | 'project'

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
  kind: 'apk' | 'manual' | 'device_snapshot' | 'runtime_xml'
  label: string
  file?: string
  line?: number
  token?: string
  rawValue?: string
  signature?: string
  files?: string[]
  confidence?: number
  reason?: string
  matchKind?: string
  componentKey?: string
  scope?: string
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
  visibility?: UiTunerElementVisibility
  standard?: UiTunerElementStandard
  source?: UiTunerSource
  sourceCandidates?: UiTunerSource[]
  runtime?: UiTunerRuntimeElement
}

export interface UiTunerElementStandard {
  scope: UiTunerStandardScope
  role: string
  component: string
  variant: string
  tokenRefs: {
    color?: string
    background?: string
    typography?: string
    spacing?: string
    radius?: string
  }
  reuseKey?: string
  note?: string
}

export interface UiTunerDocument {
  version: 1
  canvas: UiTunerCanvas
  elements: UiTunerElement[]
  updatedAt: string
  source?: UiTunerSource
  runtimeSnapshot?: UiTunerRuntimeSnapshot
}

export interface UiTunerRuntimeSnapshot {
  deviceId: string
  packageName?: string
  activityName?: string
  capturedAt: string
  nodeCount: number
  sourceRoot?: string
  sourceFingerprint?: string
  sourceBindingsPath?: string
  artifact?: UiTunerSnapshotArtifact
}

export interface UiTunerSnapshotArtifact {
  id: string
  rootDir: string
  manifestPath: string
  screenshotPath: string
  hierarchyPath: string
  rawXmlPath?: string
}

export interface UiTunerRuntimeElement {
  nodeId: string
  resourceId?: string
  className?: string
  packageName?: string
  xpath: string
  indexPath: number[]
  originalBounds: {
    left: number
    top: number
    right: number
    bottom: number
    width: number
    height: number
  }
  originalStyle: UiTunerRuntimeStyle
}

export interface UiTunerRuntimeStyle {
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
  visibility?: UiTunerElementVisibility
  standard?: UiTunerElementStandard
  runtime?: UiTunerRuntimeElement
}
