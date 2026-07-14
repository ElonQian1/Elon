import { androidNodeApi as nodeApi } from '../device/androidNodeTransport'
import { inspectorAdminUrl } from '../device/deviceInspectorApi'
import type { CapabilityGapDocument, CapabilityReadiness } from './types'

export async function getCapabilityReadiness(sessionId: string) {
  const response = await nodeApi<{
    ok: boolean
    result: CapabilityReadiness
  }>(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(sessionId)}/capabilities`,
    {},
    10_000,
  )
  return response.result
}

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
