import type { DesignPlatform } from './types'

export interface DesignIntentAction {
  order: number
  action: string
  tool: string
  reason: string
  requiresApproval: boolean
}

export type DesignIntentPlanStatus =
  | 'PLANNED'
  | 'RUNNING'
  | 'PAUSED'
  | 'FAILED'
  | 'CANCELED'
  | 'COMPLETED'
  | 'SUPERSEDED'

export interface DesignIntentActionReceipt {
  order: number
  status: 'PENDING' | 'RUNNING' | 'SUCCEEDED' | 'FAILED' | 'SKIPPED'
  attempt: number
  summary?: string | null
  errorCode?: string | null
  evidenceRefs: string[]
  updatedAt: string
}

export interface DesignIntentPlan {
  schemaVersion: 1 | 2
  revision: number
  planId: string
  taskId?: string | null
  taskLeaseId?: string | null
  intentSha256: string
  intentSummary: string
  requestedPlatforms: DesignPlatform[]
  primaryPlatform?: DesignPlatform | null
  route: string
  stateHints: string[]
  targetId?: string | null
  designSessionId?: string | null
  sessionAction: 'REUSE_SESSION' | 'OPEN_SESSION'
  actions: DesignIntentAction[]
  actionReceipts: DesignIntentActionReceipt[]
  needsClarification: boolean
  clarifications: string[]
  status: DesignIntentPlanStatus
  replannedFrom?: string | null
  supersededBy?: string | null
  startedAt?: string | null
  finishedAt?: string | null
  executionSummary?: string | null
  createdAt: string
  updatedAt: string
}

export interface DesignBindingHealth {
  status: 'UNBOUND' | 'UNCONFIRMED' | 'FILE_MISSING' | 'FILE_TOO_LARGE' | 'RANGE_STALE' | 'SOURCE_CHANGED' | 'HEALTHY'
  readyForWriteback: boolean
  sourceFile?: string | null
  expectedSourceRevision?: string | null
  currentSourceRevision?: string | null
  rangeValid: boolean
  reason: string
}

export interface DesignWritebackItem {
  operationIndex: number
  operationType: string
  platform: DesignPlatform
  adapter: string
  mutationKind: string
  readiness: 'UNSUPPORTED' | 'BLOCKED_BINDING' | 'READY_FOR_AI_HANDOFF' | 'READY_FOR_REVIEW'
  deterministic: boolean
  sourceFile?: string | null
  range?: { start: number; end: number } | null
  reason: string
}

export interface DesignWritebackPlan {
  schemaVersion: 1
  planId: string
  planRevision: number
  draftId: string
  draftRevision: number
  designSessionId: string
  sourceFile?: string | null
  expectedSourceRevision?: string | null
  bindingHealth: DesignBindingHealth['status']
  targetPlatforms: DesignPlatform[]
  operationCount: number
  items: DesignWritebackItem[]
  impact: {
    riskLevel: 'LOW' | 'MEDIUM' | 'HIGH'
    requiresExplicitApproval: true
    structuralChange: boolean
    assetChange: boolean
    files: string[]
    blockedItemCount: number
    sourceDiffAvailable: false
    runtimeVerificationRequired: true
  }
  decision: 'PROPOSED' | 'APPROVED' | 'REJECTED'
  decisionReason?: string | null
  decidedAt?: string | null
  createdAt: string
  updatedAt: string
}

export interface DesignSourcePatchProposal {
  schemaVersion: 1
  proposalId: string
  revision: number
  writebackPlanId: string
  draftId: string
  draftRevision: number
  sourceFile: string
  sourceShaBefore: string
  sourceShaAfter: string
  status: 'PROPOSED' | 'APPROVED' | 'REJECTED' | 'APPLYING' | 'APPLIED'
  decisionReason?: string | null
  reviewArtifactPath: string
  edits: Array<{
    start: number
    end: number
    beforeSha256: string
    replacementSha256: string
    beforeBytes: number
    replacementBytes: number
  }>
  createdAt: string
  updatedAt: string
  appliedAt?: string | null
  contentEmbedded: false
}

export interface DesignSourceRollbackPlan {
  schemaVersion: 1
  rollbackId: string
  revision: number
  proposalId: string
  proposalRevision: number
  sourceFile: string
  expectedSourceRevision: string
  targetSourceRevision: string
  status: 'PLANNED'
  reviewArtifactPath: string
  edits: DesignSourcePatchProposal['edits']
  contentEmbedded: false
}

export interface DesignRegressionEvidenceRef {
  path: string
  sha256: string
  width?: number | null
  height?: number | null
  nodeCount?: number | null
}

export interface DesignRegressionBaseline {
  schemaVersion: 1
  baselineId: string
  revision: number
  designSessionId: string
  draftId?: string | null
  platform: DesignPlatform
  route: string
  state: string
  viewport: Record<string, unknown>
  pixels: DesignRegressionEvidenceRef
  uiTree: DesignRegressionEvidenceRef
  nativeHost?: DesignRegressionEvidenceRef | null
  label?: string | null
  createdAt: string
  contentEmbedded: false
}

export interface DesignRegressionComparison {
  schemaVersion: 1
  comparisonId: string
  revision: number
  baselineId: string
  beforeDesignSessionId: string
  afterDesignSessionId: string
  platform: DesignPlatform
  route: string
  beforePixels: DesignRegressionEvidenceRef
  afterPixels: DesignRegressionEvidenceRef
  beforeUiTree: DesignRegressionEvidenceRef
  afterUiTree: DesignRegressionEvidenceRef
  thresholds: {
    maxPixelDiffRatio: number
    maxMissingSelectors: number
    maxChangedSelectors: number
    requireSameViewport: boolean
    ignoreSelectors: string[]
  }
  changedSelectors: string[]
  status: 'READY_TO_COMPARE' | 'PASSED' | 'FAILED'
  result?: Record<string, unknown> | null
  createdAt: string
  updatedAt: string
  contentEmbedded: false
}

export interface DesignEventCheckpoint {
  schemaVersion: 1
  consumerId: string
  taskId: string
  cursor: string
  revision: number
  createdAt: string
  updatedAt: string
}
