import { nodeApi } from '../../node/localNodeApi'
import { inspectorAdminUrl } from '../device/deviceInspectorApi'
import type { PixelRect } from './types'

export interface DesignNodeCandidate {
  runtimeNodeId: string
  definitionId: string
  instanceKey?: string
  kind: string
  text?: string
  targetBounds: PixelRect
  score: number
  editablePropertyCount: number
}

export interface DesignDiffRegion {
  id: string
  targetRect: PixelRect
  changedPixels: number
  candidates: DesignNodeCandidate[]
  recommendedRuntimeNodeId?: string
  confidence: number
}

export interface DesignDiffRegionAnalysis {
  baselineWidth: number
  baselineHeight: number
  targetWidth: number
  targetHeight: number
  scaleX: number
  scaleY: number
  changedPixelRatio: number
  regions: DesignDiffRegion[]
}

export async function analyzeDesignDiffRegions(sessionId: string) {
  const response = await nodeApi<{ analysis: DesignDiffRegionAnalysis }>(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(sessionId)}/design-diff-regions`,
    {
      method: 'POST',
      body: JSON.stringify({
        channelThreshold: 18,
        minimumRegionArea: 96,
        mergeGapPx: 12,
        maximumRegions: 24,
      }),
    },
    30_000,
  )
  return response.analysis
}
