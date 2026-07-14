import { nodeApi } from '../../node/localNodeApi'
import { inspectorAdminUrl } from '../device/deviceInspectorApi'
import type { VisualDiffResult } from './liveUiIrApi'
import type { AndroidDeviceLeaseProof } from '../device/deviceLeaseApi'

export type LiveUiScope = 'INSTANCE' | 'DEFINITION'

export interface LivePropertyValue {
  type: 'dp' | 'sp' | 'float' | 'argb' | 'text' | 'enum' | 'dimension' | 'bool'
  value: string | number | boolean
}

export interface LivePropertySnapshot {
  effective?: LivePropertyValue
  measured?: LivePropertyValue
  changeLevel: 'LIVE' | 'COMPILE' | 'REDEPLOY'
  commitMode: 'DETERMINISTIC' | 'CODEX' | 'SESSION_ONLY' | 'READ_ONLY'
  binding?: unknown
  constraints?: { minimum?: number; maximum?: number; step?: number }
}

export interface LiveUiNode {
  runtimeNodeId: string
  definitionId: string
  instanceKey?: string
  parentRuntimeNodeId?: string
  screenId: string
  kind: string
  text?: string
  resourceId?: string
  className: string
  source?: unknown
  geometry: {
    boundsInDisplayPx: {
      left: number
      top: number
      right: number
      bottom: number
      width: number
      height: number
    }
    density: number
    fontScale: number
    rotation: number
    visible: boolean
  }
  properties: Record<string, LivePropertySnapshot>
  capabilities: Record<string, boolean>
}

export interface LiveUiSession {
  id: string
  deviceId: string
  packageName: string
  projectRoot?: string
  devicePort: number
  createdAt: string
  connected: boolean
  runtimeBuildId?: string
  runtimeVersion?: string
  treeRevision: number
  nodeCount: number
  historyCount: number
  redoCount: number
  lastSeenAt?: string
  lastError?: string
}

export interface LiveMcpDescriptor {
  name: string
  transport: 'streamable-http'
  sessionId: string
  protocolVersion: string
  purpose: string
  configPath: string
}

export interface LiveUiSessionStart {
  session: LiveUiSession
  mcp: LiveMcpDescriptor
}

export interface LivePatchOperation {
  property: string
  value: LivePropertyValue
}

export interface LivePatchAck {
  status: 'APPLIED' | 'REJECTED'
  requestId: string
  newTreeRevision: number
  beforeValues?: Record<string, LivePropertyValue>
  effectiveValues?: Record<string, LivePropertyValue>
  measuredGeometry?: Record<string, number>
  error?: string
}

export interface LiveUiFrame {
  dataUrl: string
  width: number
  height: number
  bytes: number
  capturedAt: string
}

export interface LivePreviewRequest {
  screenId: string
  scenario: 'normal' | 'loading' | 'empty' | 'error'
  theme: 'system' | 'light' | 'dark'
  fontScale: number
  locale: string
}

export interface LiveBuildVerifyResult {
  status: 'BUILD_VERIFIED' | 'SOURCE_MISMATCH' | 'TARGET_MISMATCH' | 'TARGET_NOT_CONFIGURED'
  apkPath: string
  buildDurationMs: number
  installOutput: string
  runtimeConnected: boolean
  runtimeBuildId?: string
  nodeCount: number
  screenshotWidth: number
  screenshotHeight: number
  visualDiff?: VisualDiffResult
  sourceParityDiff?: VisualDiffResult
  sourceParityVerified?: boolean
  verificationGate?: {
    status: LiveBuildVerifyResult['status']
    verified: boolean
    sourceParity: 'PASSED' | 'FAILED' | 'NOT_REQUIRED' | 'NOT_CONFIGURED'
    targetFidelity: 'PASSED' | 'FAILED' | 'NOT_REQUIRED' | 'NOT_CONFIGURED'
    failedMetrics: string[]
  }
  message: string
}

export interface LiveDebugRuntimePrepareResult {
  packageName: string
  build: LiveBuildVerifyResult
}

export async function prepareLiveDebugRuntime(input: {
  deviceId: string
  basePackageName: string
  projectRoot: string
  debugApplicationIdSuffix: string
  lease: AndroidDeviceLeaseProof
}): Promise<LiveDebugRuntimePrepareResult> {
  const response = await nodeApi<{ result: LiveDebugRuntimePrepareResult }>(
    inspectorAdminUrl(),
    '/api/android-live/debug-runtime/prepare',
    { method: 'POST', body: JSON.stringify(input) },
    20 * 60_000,
  )
  return response.result
}

export async function startLiveUiSession(input: {
  deviceId: string
  packageName: string
  projectRoot?: string
  lease: AndroidDeviceLeaseProof
}): Promise<LiveUiSessionStart> {
  return nodeApi<LiveUiSessionStart>(
    inspectorAdminUrl(),
    '/api/android-live/sessions',
    {
      method: 'POST',
      body: JSON.stringify(input),
    },
    15_000,
  )
}

export async function getLiveUiSession(sessionId: string): Promise<LiveUiSession> {
  const response = await nodeApi<{ session: LiveUiSession }>(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(sessionId)}`,
    {},
    5_000,
  )
  return response.session
}

export async function stopLiveUiSession(sessionId: string): Promise<void> {
  await nodeApi(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(sessionId)}`,
    { method: 'DELETE' },
    10_000,
  )
}

export async function getLiveUiTree(sessionId: string): Promise<{
  treeRevision: number
  nodes: LiveUiNode[]
}> {
  return nodeApi(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(sessionId)}/tree`,
    {},
    8_000,
  )
}

export async function getLiveUiFrame(sessionId: string): Promise<LiveUiFrame> {
  const response = await nodeApi<{ frame: LiveUiFrame }>(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(sessionId)}/frame`,
    {},
    8_000,
  )
  return response.frame
}

export async function reconnectLiveUiSession(sessionId: string): Promise<LiveUiSession> {
  const response = await nodeApi<{ session: LiveUiSession }>(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(sessionId)}/reconnect`,
    { method: 'POST' },
    15_000,
  )
  return response.session
}

export async function openLiveUiPreview(
  sessionId: string,
  request: LivePreviewRequest,
): Promise<void> {
  await nodeApi(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(sessionId)}/preview`,
    { method: 'POST', body: JSON.stringify(request) },
    15_000,
  )
}

export async function buildAndVerifyLiveUi(
  sessionId: string,
  preview?: LivePreviewRequest,
  debugApplicationIdSuffix?: string,
): Promise<LiveBuildVerifyResult> {
  const response = await nodeApi<{ result: LiveBuildVerifyResult }>(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(sessionId)}/build-verify`,
    { method: 'POST', body: JSON.stringify({ preview, debugApplicationIdSuffix }) },
    20 * 60_000,
  )
  return response.result
}

export async function applyLiveUiPatch(input: {
  sessionId: string
  target: LiveUiNode
  scope: LiveUiScope
  operation?: LivePatchOperation
  operations?: LivePatchOperation[]
  gestureId?: string
}): Promise<LivePatchAck> {
  const operations = input.operations ?? (input.operation ? [input.operation] : [])
  if (operations.length === 0) throw new Error('LiveStylePatch 至少需要一个操作')
  const response = await nodeApi<{ ack: LivePatchAck }>(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(input.sessionId)}/patch`,
    {
      method: 'POST',
      body: JSON.stringify({
        protocolVersion: 1,
        messageType: 'patch.apply',
        sessionId: input.sessionId,
        requestId: '',
        gestureId: input.gestureId,
        sequence: Date.now(),
        target: {
          scope: input.scope,
          runtimeNodeId: input.scope === 'INSTANCE' ? input.target.runtimeNodeId : undefined,
          definitionId: input.target.definitionId,
          instanceKey: input.scope === 'INSTANCE' ? input.target.instanceKey : undefined,
        },
        atomic: true,
        ephemeral: true,
        operations,
      }),
    },
    10_000,
  )
  return response.ack
}

export async function liveUiHistoryAction(
  sessionId: string,
  action: 'undo' | 'redo',
): Promise<LivePatchAck> {
  const response = await nodeApi<{ ack: LivePatchAck }>(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(sessionId)}/${action}`,
    { method: 'POST' },
    10_000,
  )
  return response.ack
}
