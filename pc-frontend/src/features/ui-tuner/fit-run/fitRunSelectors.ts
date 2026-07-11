import type { FitRunCandidate, FitRunDocument } from './types'

export function fitTargetPassed(run: FitRunDocument, candidate = run.best) {
  if (!candidate || candidate.score.hardFailures.length > 0) return false
  return candidate.score.overallLoss <= run.thresholds.maxOverallLoss
    && candidate.score.geometryError <= run.thresholds.maxGeometryError
    && candidate.score.colorError <= run.thresholds.maxColorError
    && candidate.score.edgeError <= run.thresholds.maxEdgeError
}

export function fitSourceParityPassed(run: FitRunDocument, candidate = run.best) {
  return Boolean(candidate?.sourceParityVerified
    && candidate.sourceParityLoss !== undefined
    && candidate.sourceParityLoss <= run.thresholds.maxSourceParityLoss)
}

export function fitFailedMetrics(candidate?: FitRunCandidate) {
  return candidate?.score.hardFailures ?? []
}
