import type { DesignPlatform } from './types'

export interface DesignIntentAction {
  order: number
  action: string
  tool: string
  reason: string
  requiresApproval: boolean
}

export interface DesignIntentPlan {
  schemaVersion: 1
  planId: string
  taskId?: string | null
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
  needsClarification: boolean
  clarifications: string[]
  status: 'PLANNED'
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

export interface DesignEventCheckpoint {
  schemaVersion: 1
  consumerId: string
  taskId: string
  cursor: string
  revision: number
  createdAt: string
  updatedAt: string
}
