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
  nativeBehavior?: TauriBehaviorEvidence | null
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
  nativeBehavior?: TauriBehaviorEvidence | null
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

export interface DesignStylePatch {
  property: string
  before?: string | null
  after: string
  unit?: string | null
}

export interface DesignSourceBinding {
  status: 'BOUND' | 'CANDIDATE' | 'NEEDS_AI'
  sourceFile: string
  symbol?: string | null
  kind: string
  range?: { start: number; end: number } | null
  sourceRevision?: string | null
  confidence: 'high' | 'medium' | 'low'
  reason: string
}

export interface DesignDraft {
  schemaVersion: 1
  draftId: string
  designSessionId: string
  platform: DesignPlatform
  route: string
  selector: string
  scope: 'instance' | 'component' | 'route' | 'project'
  patches: DesignStylePatch[]
  sourceBinding?: DesignSourceBinding | null
  targetPlatforms: DesignPlatform[]
  revision: number
  status: string
  writebackReceiptId?: string | null
  historyDepth: number
  createdAt: string
  updatedAt: string
}

export interface DesignDraftSummary {
  draftId: string
  designSessionId: string
  platform: DesignPlatform
  route: string
  selector: string
  scope: DesignDraft['scope']
  revision: number
  status: string
  targetPlatforms: DesignPlatform[]
  sourceBindingStatus?: DesignSourceBinding['status'] | null
  writebackReceiptId?: string | null
  updatedAt: string
}

export interface DesignWritebackReceipt {
  schemaVersion: 1
  receiptId: string
  operationId: string
  draftRevision: number
  targetPlatforms: DesignPlatform[]
  sourceRevisionBefore: string
  sourceRevision: string
  sourceHash: string
  changedFiles: string[]
  sourceHashes: Record<string, string>
  platformResults: Record<string, Record<string, unknown>>
  status: string
  complete: boolean
  evidenceComplete: boolean
  diagnostics: string[]
  createdAt: string
  updatedAt: string
}

export interface DesignCapabilities {
  schema: 'elon.ui-design-capabilities.v1'
  runtimeSchema: string
  protocolRevision: string
  installedRuntimeEvidence: { source: 'MCP_TOOL_RESPONSE'; tool: string }
  capabilityIds: string[]
  platforms: Record<DesignPlatform, Record<string, unknown>>
  limits: {
    activeBrowserRuntimes: number
    browserIdleMinutes: number
    browserLifetimeMinutes: number
    browserOperations: number
    secretInputAllowed: false
    arbitraryTauriCommandAllowed: false
  }
  project: { detectedPlatforms: DesignPlatform[]; filesInspected: number; scanTruncated: boolean }
  contentEmbedded: false
}

export interface DesignBrowserRuntime {
  runtimeId: string
  status: 'READY' | string
  targetOrigin: string
  fixtureProfile?: string | null
  operationCount: number
  limits: {
    maxActiveSessions: number
    maxOperations: number
    idleTtlSeconds: number
    lifetimeTtlSeconds: number
  }
  statePreserved: true
}

export interface DesignBrowserResult {
  ok: boolean
  status: 'CAPTURED' | 'READY' | 'STOPPED' | 'NOT_RUNNING' | string
  runtime?: DesignBrowserRuntime
  artifact?: DesignArtifactRef
  uiTree?: DesignArtifactRef
  diagnostic?: { code: string; message: string; retryable: boolean; nextStep: string }
}

export type DesignBrowserInteraction =
  | { action: 'click'; selector: string }
  | { action: 'scrollIntoView'; selector: string }
  | { action: 'pressKey'; selector?: string; key: 'Enter' | 'Escape' | 'Tab' | 'ArrowUp' | 'ArrowDown' | 'ArrowLeft' | 'ArrowRight' | 'Space' | 'Home' | 'End' }
  | { action: 'fill'; selector: string; fixtureKey: string }
  | { action: 'selectOption'; selector: string; fixtureKey: string }
  | { action: 'setChecked'; selector: string; checked: boolean }

export interface TauriBehaviorEvidence {
  hostCoverage: 'TAURI_NATIVE_BEHAVIOR'
  artifact: DesignArtifactRef
  menuCoverage: 'WIN32_NATIVE_MENU_OBSERVED'
  menuItemCount: number
  dialogCoverage: 'DESCENDANT_TOP_LEVEL_WINDOWS_OBSERVED'
  dialogCount: number
  rustCommandCoverage: 'PROJECT_INSTRUMENTED_TRACE' | 'NOT_INSTRUMENTED'
  commandEventCount: number
  assertionsPassed: boolean
  capturedAt: string
  base64Embedded: false
}

export interface DesignVerificationPlatform {
  platform: DesignPlatform
  status: 'PASSED' | 'BLOCKED' | 'IN_PROGRESS' | 'READY' | 'NEEDS_DRAFT_OR_BINDING'
  requirements: string[]
  writeback: {
    status: string
    method?: string | null
    evidenceComplete: boolean
    changedFilesCount: number
    error?: string | null
  }
  currentDesignSessionEvidence: Record<string, unknown>
  codeCapabilityAvailable: true
  runtimeVerified: boolean
}

export interface DesignVerificationMatrix {
  schema: 'elon.ui-design-verification-matrix.v1'
  runtimeSchema: string
  draft: { draftId: string; revision: number; status: string; bindingReady: boolean; patchesReady: boolean }
  designSession: { designSessionId: string; platform: DesignPlatform; state: string; hasEvidence: boolean }
  receipt?: { receiptId: string; status: string; complete: boolean; evidenceComplete: boolean; sourceRevision: string; updatedAt: string } | null
  platforms: DesignVerificationPlatform[]
  overallStatus: string
  completionRule: string
  runtimeTestsExecuted: false
  contentEmbedded: false
}
