import { nodeApi } from '../../node/localNodeApi'
import { inspectorAdminUrl } from '../device/deviceInspectorApi'
import type { LivePropertyValue } from './liveUiApi'

export interface LiveSourceCommitEntry {
  definitionId: string
  resourceId?: string
  scope: string
  property: string
  value: LivePropertyValue
  sourceFile?: string
  sourceKey?: string
  oldValue?: string
  commitMode: 'DETERMINISTIC' | 'CODEX' | 'SESSION_ONLY'
  impactCount: number
  reason: string
}

export interface LiveSourceCommitPlan {
  sessionId: string
  projectRoot: string
  sourceRevision: string
  deterministicCount: number
  codexCount: number
  entries: LiveSourceCommitEntry[]
}

export interface LiveSourceCommitResult {
  status: 'SOURCE_SAVED'
  committedCount: number
  deferredCount: number
  changedFiles: string[]
  sourceRevisionBefore: string
  sourceRevisionAfter: string
  deferred: LiveSourceCommitEntry[]
}

export async function getLiveSourceCommitPlan(sessionId: string) {
  const response = await nodeApi<{ plan: LiveSourceCommitPlan }>(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(sessionId)}/commit-plan`,
    {},
    12_000,
  )
  return response.plan
}

export async function commitLiveSource(
  sessionId: string,
  sourceRevision: string,
) {
  const response = await nodeApi<{ result: LiveSourceCommitResult }>(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(sessionId)}/commit`,
    {
      method: 'POST',
      body: JSON.stringify({ sourceRevision }),
    },
    15_000,
  )
  return response.result
}
