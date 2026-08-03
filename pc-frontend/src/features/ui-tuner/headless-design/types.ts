export type DesignPlatform = 'web' | 'pwa' | 'tauri' | 'android'

export interface DesignViewport {
  width: number
  height: number
  deviceScaleFactor: number
}

export interface DesignTarget {
  id: string
  platform: DesignPlatform
  label: string
  adapter: string
  evidenceLevel: string
  sourceRoots: string[]
  configFiles: string[]
  capabilities: string[]
  nativeHostVerified: boolean
}

export interface DesignArtifactRef {
  path: string
  sha256: string
  width?: number
  height?: number
  mediaType?: string
  nodeCount?: number
  interactiveCount?: number
  truncated?: boolean
}

export interface DesignSessionIdentity {
  designSessionId: string
  platform: DesignPlatform
  route: string
  url?: string | null
  viewport: DesignViewport
  state: string
}

export interface DesignSessionRecord extends DesignSessionIdentity {
  schemaVersion: number
  mcpSessionId: string
  target: DesignTarget
  lastEvidence?: Record<string, unknown> | null
  createdAt: string
  updatedAt: string
}

export interface DesignSessionSummary extends DesignSessionIdentity {
  label: string
  adapter: string
  evidenceLevel: string
  nativeHostVerified: boolean
  nativeHost?: TauriNativeHostEvidence | null
  hasEvidence: boolean
  pixels?: DesignArtifactRef | null
  uiTree?: DesignArtifactRef | null
  createdAt: string
  updatedAt: string
}

export interface TauriNativeHostEvidence {
  runtimeId: string
  nativeHostVerified: true
  hostCoverage: 'TAURI_NATIVE_WINDOW'
  artifact: DesignArtifactRef
  window: {
    title: string
    processId: number
    bounds: { left: number; top: number; width: number; height: number }
  }
  launcherProcessId: number
  runtimeStartedAt: string
  capturedAt: string
  base64Embedded: false
}

export interface TauriRuntimeResult {
  ok: boolean
  status: 'STARTING' | 'READY' | 'FAILED' | 'STOPPED' | 'NOT_RUNNING' | string
  runtime?: {
    runtimeId: string
    status: string
    launcherProcessId: number
    projectRoot: string
    moduleRoot: string
    command: string
    startedAt: string
    window?: TauriNativeHostEvidence['window'] | null
  }
  nativeHost?: TauriNativeHostEvidence
  retryAfterMs?: number
  next?: string
  exitCode?: number | null
}

export interface SemanticUiNode {
  id: string
  selector: string
  parentSelector?: string | null
  tag: string
  role: string
  label: string
  interactive: boolean
  disabled: boolean
  checked?: boolean | null
  selected?: boolean | null
  inputType?: string | null
  bounds: { left: number; top: number; width: number; height: number }
  style: {
    display?: string
    color?: string
    backgroundColor?: string
    fontSize?: string
    fontWeight?: string
    borderRadius?: string
  }
}

export interface DesignSurface {
  session: DesignSessionRecord
  status: 'AWAITING_CAPTURE' | 'CAPTURED' | string
  surface?: {
    title?: string | null
    route?: string | null
    viewport?: DesignViewport | null
    nodeCount?: number | null
    interactiveCount?: number | null
    treeTruncated?: boolean | null
    returnedNodeCount?: number
    query?: string
    returnTruncated?: boolean
  }
  nodes: SemanticUiNode[]
  pixels?: DesignArtifactRef | null
  uiTree?: DesignArtifactRef | null
  nativeHost?: TauriNativeHostEvidence | null
  nativeHostVerified?: boolean
  base64Embedded?: false
  next?: string
}

export interface DesignCaptureResult {
  ok: boolean
  status: string
  designSessionId: string
  platform: DesignPlatform
  hostCoverage?: string
  nativeHostVerified?: boolean
  artifact?: DesignArtifactRef
  uiTree?: DesignArtifactRef
  diagnostic?: { code: string; message: string; retryable: boolean; nextStep: string }
  message?: string
}

export interface DesignTargetListResult {
  schemaVersion: number
  targets: DesignTarget[]
  scan: { filesInspected: number; truncated: boolean; contentEmbedded: false }
}

export interface DesignSessionListResult {
  schemaVersion: number
  sessions: DesignSessionSummary[]
  invalidRecordCount: number
  contentEmbedded: false
}
