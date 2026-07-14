import { androidNodeApi as nodeApi } from '../device/androidNodeTransport'
import { inspectorAdminUrl } from '../device/deviceInspectorApi'
import type { UiTunerDocument, UiTunerElement, UiTunerReferenceImage } from '../types'
import type { LivePatchOperation } from './liveUiApi'

export interface LiveTargetDesign {
  id: string
  name: string
  path: string
  sha256: string
  width: number
  height: number
  mimeType: string
  figmaUrl?: string
}

export interface LiveUiIrDocument {
  version: number
  revision: string
  treeRevision: number
  selectedRuntimeNodeId?: string
  targetDesign?: LiveTargetDesign
  summary: {
    screenId?: string
    nodeCount: number
    visibleNodeCount: number
    editableNodeCount: number
    selectedDefinitionId?: string
    hasTargetDesign: boolean
  }
}

export interface VisualDiffResult {
  meanAbsoluteColorError: number
  edgeError: number
  alphaError: number
  geometryError: number
  visualLoss: number
  scoreReport?: {
    optimizationScore: number
    targetGate: {
      passed: boolean
      failedMetrics: string[]
    }
    geometry: {
      widthErrorPx: number
      heightErrorPx: number
      sizeErrorRatio: number
      aspectErrorRatio: number
    }
    position: {
      maxEdgeErrorPx: number
      centerErrorPx: number
    }
    color: { meanAbsoluteError: number; p95AbsoluteError: number }
    edge: { similarity: number; error: number }
    perceptual: { luminanceError: number; structuralError: number }
    coverage: { ratio: number }
  }
}

export interface VisualSolverResult {
  status: 'APPLIED' | 'NO_CHANGE'
  runtimeNodeId: string
  evaluations: number
  baseline: VisualDiffResult
  finalDiff: VisualDiffResult
  improvementPercent: number
  operations: LivePatchOperation[]
}

export interface PixelRect {
  left: number
  top: number
  right: number
  bottom: number
}

export async function uploadLiveTargetDesign(
  sessionId: string,
  image: UiTunerReferenceImage,
  figmaUrl?: string,
) {
  const response = await nodeApi<{ targetDesign: LiveTargetDesign }>(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(sessionId)}/target-design`,
    {
      method: 'POST',
      body: JSON.stringify({
        name: image.name,
        dataUrl: image.dataUrl,
        figmaUrl: validFigmaUrl(figmaUrl ?? image.figmaUrl),
      }),
    },
    20_000,
  )
  return response.targetDesign
}

function validFigmaUrl(value?: string) {
  const normalized = value?.trim()
  return normalized && /^https:\/\/(www\.)?figma\.com\//i.test(normalized)
    ? normalized
    : undefined
}

export async function bindLiveUiIr(input: {
  sessionId: string
  document: UiTunerDocument
  selected: UiTunerElement | null
  selectedRuntimeNodeId?: string
  targetDesign?: LiveTargetDesign
}) {
  const artifact = input.document.runtimeSnapshot?.artifact
  const response = await nodeApi<{ document: LiveUiIrDocument }>(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(input.sessionId)}/ui-ir`,
    {
      method: 'POST',
      body: JSON.stringify({
        snapshot: artifact ? {
          snapshotId: artifact.id,
          screenshotPath: artifact.screenshotPath,
          hierarchyPath: artifact.hierarchyPath,
          manifestPath: artifact.manifestPath,
          width: input.document.canvas.width,
          height: input.document.canvas.height,
        } : undefined,
        selectedRuntimeNodeId: input.selectedRuntimeNodeId,
        sourceCandidates: input.selected?.sourceCandidates ?? [],
        targetDesign: input.targetDesign,
        clearTargetDesign: !input.document.canvas.targetDesign,
      }),
    },
    12_000,
  )
  return response.document
}

export async function runLiveVisualSolver(input: {
  sessionId: string
  runtimeNodeId: string
  targetRect: PixelRect
  projectedCurrentRect: PixelRect
  properties?: string[]
  maxEvaluations?: number
}) {
  const response = await nodeApi<{ result: VisualSolverResult }>(
    inspectorAdminUrl(),
    `/api/android-live/sessions/${encodeURIComponent(input.sessionId)}/visual-solver`,
    {
      method: 'POST',
      body: JSON.stringify({
        runtimeNodeId: input.runtimeNodeId,
        targetRect: input.targetRect,
        projectedCurrentRect: input.projectedCurrentRect,
        properties: input.properties,
        maxEvaluations: input.maxEvaluations ?? 12,
      }),
    },
    60_000,
  )
  return response.result
}
