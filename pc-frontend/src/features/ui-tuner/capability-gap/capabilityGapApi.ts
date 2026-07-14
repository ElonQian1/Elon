import { nodeApi } from '../../node/localNodeApi'
import { inspectorAdminUrl } from '../device/deviceInspectorApi'
import type { CapabilityGapDocument } from './types'

export async function listCapabilityGaps(sessionId: string) {
  const response = await nodeApi<{
    ok: boolean
    result: { gaps: CapabilityGapDocument[] }
  }>(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(sessionId)}/capability-gaps`,
    {},
    10_000,
  )
  return response.result.gaps
}
