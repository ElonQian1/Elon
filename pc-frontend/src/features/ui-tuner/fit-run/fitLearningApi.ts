import { androidNodeApi as nodeApi } from '../device/androidNodeTransport'
import { inspectorAdminUrl } from '../device/deviceInspectorApi'

export interface FitLearningPriorSummary {
  priorId: string
  scope: 'EXACT_COMPONENT' | 'CROSS_COMPONENT'
  componentKind: string
  definitionId?: string
  propertySet: string[]
  successCount: number
  screenCount: number
  confidence: number
  medianFactors: Record<string, number>
  translationFeatures: {
    parentLayoutKind?: string
    widthScale?: number
    heightScale?: number
    targetWidthRatio?: number
    targetHeightRatio?: number
  }
}

export interface FitLearningSummary {
  caseCount: number
  promotedCaseCount: number
  priorCount: number
  exactPriorCount: number
  crossComponentPriorCount: number
  reusablePriors: FitLearningPriorSummary[]
}

export async function getFitLearningSummary(sessionId: string) {
  const response = await nodeApi<{ summary: FitLearningSummary }>(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(sessionId)}/fit-learning`,
    {},
    10_000,
  )
  return response.summary
}
