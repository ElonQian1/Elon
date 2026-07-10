import { nodeApi } from '../../node/localNodeApi'
import { inspectorAdminUrl } from '../device/deviceInspectorApi'

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

export async function startLiveUiSession(input: {
  deviceId: string
  packageName: string
  projectRoot?: string
}): Promise<LiveUiSession> {
  const response = await nodeApi<{ session: LiveUiSession }>(
    inspectorAdminUrl(),
    '/api/android-live/sessions',
    {
      method: 'POST',
      body: JSON.stringify(input),
    },
    15_000,
  )
  return response.session
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

export async function applyLiveUiPatch(input: {
  sessionId: string
  target: LiveUiNode
  scope: LiveUiScope
  operation: LivePatchOperation
}): Promise<LivePatchAck> {
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
        sequence: Date.now(),
        target: {
          scope: input.scope,
          runtimeNodeId: input.scope === 'INSTANCE' ? input.target.runtimeNodeId : undefined,
          definitionId: input.target.definitionId,
          instanceKey: input.scope === 'INSTANCE' ? input.target.instanceKey : undefined,
        },
        atomic: true,
        ephemeral: true,
        operations: [input.operation],
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
