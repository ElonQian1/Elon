export type CapabilityGapStatus =
  | 'APPROVED'
  | 'UPGRADING'
  | 'PUBLISHED'
  | 'RESUMED'
  | 'HUMAN_REQUIRED'

export interface CapabilityUpgradePolicy {
  trustedBoundary: 'LOCAL_GIT_WORKSPACE'
  automaticSourceUpgrade: boolean
  automaticPublish: boolean
  maxUpgradeRounds: number
}

export interface CapabilityUpgradeAttempt {
  round: number
  startedAt: string
  sourceRevisionBefore: string
  sourceRevisionAfter?: string
  commitId?: string
  version?: string
  changedFiles: string[]
}

export interface CapabilityGapDocument {
  schemaVersion: number
  gapId: string
  taskId: string
  fitRunId?: string
  projectRoot: string
  status: CapabilityGapStatus
  missingCapabilities: string[]
  evidence: string[]
  proposedChanges: string[]
  resumeTarget: string
  policy: CapabilityUpgradePolicy
  upgradeRounds: number
  attempts: CapabilityUpgradeAttempt[]
  createdAt: string
  updatedAt: string
  lastError?: string
}
