import type { LivePatchOperation } from '../live/liveUiApi'
import type { PixelRect } from '../live/liveUiIrApi'

export type FitRunPhase =
  | 'CREATED'
  | 'BASELINING'
  | 'LOCAL_SOLVING'
  | 'AWAITING_CODEX'
  | 'CODEX_RUNNING'
  | 'REBUILDING'
  | 'EVALUATING'
  | 'CANDIDATE_READY'
  | 'SOURCE_VERIFYING'
  | 'PAUSED'
  | 'ACCEPTED'
  | 'PLATEAU'
  | 'FAILED'
  | 'CANCELLED'

export type FitRunCommandType =
  | 'START'
  | 'PAUSE'
  | 'RESUME'
  | 'CANCEL'
  | 'REBIND_SESSION'
  | 'ACCEPT_BEST'
  | 'CODEX_STARTED'
  | 'CODEX_COMPLETED'
  | 'CODEX_FAILED'

export interface FitRunPairInput {
  targetDesignId: string
  targetSha256: string
  targetRect: PixelRect
  runtimeNodeId: string
  definitionId: string
  componentKind?: string
  parentLayoutKind?: string
  instanceKey?: string
  currentRect: PixelRect
  projectedTargetRect: PixelRect
  calibrationId?: string
  confidence?: number
}

export interface FitRunEnvironment {
  screenId?: string
  scenario?: string
  theme?: string
  locale?: string
  viewportWidth?: number
  viewportHeight?: number
  density?: number
  fontScale?: number
  rotation?: number
  insets?: Record<string, number>
}

export interface FitRunBudget {
  maxDurationMs: number
  maxLocalEvaluations: number
  maxCodexRounds: number
  maxBuildRounds: number
  maxNoImprovementTrials: number
}

export interface FitRunUsage {
  elapsedMs: number
  localEvaluations: number
  codexRounds: number
  buildRounds: number
  noImprovementTrials: number
  codexTokens?: number
}

export interface FitRunThresholds {
  maxOverallLoss: number
  maxGeometryError: number
  maxColorError: number
  maxEdgeError: number
  maxSourceParityLoss: number
  minMeaningfulImprovement: number
  plateauWindow: number
}

export interface FitRunScore {
  scorerVersion: string
  overallLoss: number
  geometryError: number
  colorError: number
  edgeError: number
  alphaError: number
  shapeError?: number
  typographyError?: number
  hardFailures: string[]
}

export interface FitRunCandidate {
  trialId: string
  score: FitRunScore
  operations: LivePatchOperation[]
  screenshotPath?: string
  diffArtifactPath?: string
  sourceRevision?: string
  runtimeBuildId?: string
  commitId?: string
  sourceParityLoss?: number
  sourceParityVerified: boolean
}

export interface FitRunHandoff {
  handoffId: string
  runId: string
  reason: string
  status: 'PENDING' | 'RUNNING' | 'COMPLETED' | 'FAILED'
  createdAt: string
  artifactPath?: string
  taskId?: string
  targetCropPath?: string
  currentCropPath?: string
  bestCropPath?: string
  sourceRevisionBefore?: string
  sourceRevisionAfter?: string
  changedFiles: string[]
  commitId?: string
  error?: string
}

export interface FitRunDocument {
  schemaVersion: number
  runId: string
  sessionId: string
  projectRoot?: string
  packageName: string
  deviceId: string
  phase: FitRunPhase
  stopReason?: string
  pair: FitRunPairInput
  environment: FitRunEnvironment
  properties: string[]
  budget: FitRunBudget
  usage: FitRunUsage
  thresholds: FitRunThresholds
  baseline?: FitRunCandidate
  current?: FitRunCandidate
  best?: FitRunCandidate
  handoff?: FitRunHandoff
  resumePhase?: FitRunPhase
  createdAt: string
  updatedAt: string
  lastSequence: number
  lastError?: string
}

export interface CreateFitRunInput {
  pair: FitRunPairInput
  environment?: FitRunEnvironment
  properties?: string[]
  budget?: Partial<FitRunBudget>
  thresholds?: Partial<FitRunThresholds>
  autoStart?: boolean
}

export type FitRunCommand =
  | { commandId: string; type: 'START' | 'PAUSE' | 'RESUME' | 'CANCEL' | 'ACCEPT_BEST' }
  | {
      commandId: string
      type: 'REBIND_SESSION'
      newSessionId: string
      newRuntimeNodeId?: string
      newCurrentRect?: PixelRect
    }
  | { commandId: string; type: 'CODEX_STARTED'; handoffId: string; taskId: string }
  | {
      commandId: string
      type: 'CODEX_COMPLETED'
      handoffId: string
      taskId?: string
      sourceRevisionBefore?: string
      sourceRevisionAfter: string
      changedFiles?: string[]
      commitId?: string
      tokenUsage?: number
    }
  | { commandId: string; type: 'CODEX_FAILED'; handoffId: string; error: string }

type WithoutCommandId<T> = T extends unknown
  ? Omit<T, 'commandId'> & { commandId?: string }
  : never

export type FitRunCommandInput = WithoutCommandId<FitRunCommand>

export const ACTIVE_FIT_RUN_PHASES = new Set<FitRunPhase>([
  'BASELINING',
  'LOCAL_SOLVING',
  'CODEX_RUNNING',
  'REBUILDING',
  'EVALUATING',
  'SOURCE_VERIFYING',
])

export const TERMINAL_FIT_RUN_PHASES = new Set<FitRunPhase>([
  'ACCEPTED',
  'PLATEAU',
  'FAILED',
  'CANCELLED',
])
