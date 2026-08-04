import type {
  DesignBindingHealth,
  DesignEventCheckpoint,
  DesignIntentPlan,
  DesignWritebackPlan,
} from './designPlanningTypes'
import { callDesignNode } from './designSessionApi'
import type { DesignPlatform } from './types'

export function planDesignIntent(input: {
  projectRoot: string
  intent: string
  taskId?: string
  platform?: DesignPlatform
  route?: string
  state?: string
  designSessionId?: string
}) {
  return callDesignNode<{ schema: 'elon.ui-design-intent-plan.v1'; plan: DesignIntentPlan; sourceModified: false; runtimeStarted: false }>(
    '/api/android-live/design/intents/plan', input,
  )
}

export function getDesignIntentPlan(projectRoot: string, planId: string) {
  return callDesignNode<{ schema: 'elon.ui-design-intent-plan.v1'; plan: DesignIntentPlan; contentEmbedded: false }>(
    `/api/android-live/design/intents/${encodeURIComponent(planId)}`, { projectRoot },
  )
}

export function checkDesignSourceBinding(input: {
  projectRoot: string
  draftId: string
  includeRecoveryCandidates?: boolean
  limit?: number
}) {
  const { draftId, ...body } = input
  return callDesignNode<{
    schema: 'elon.ui-design-binding-health.v1'
    draftId: string
    health: DesignBindingHealth
    recovery: { candidates: unknown[]; error?: string | null; autoRebound: false }
    sourceModified: false
    contentEmbedded: false
  }>(`/api/android-live/design/drafts/${encodeURIComponent(draftId)}/source-binding/health`, body)
}

export function planDesignWriteback(projectRoot: string, draftId: string) {
  return callDesignNode<{ schema: 'elon.ui-design-writeback-plan.v1'; plan: DesignWritebackPlan; action: 'PLANNED' | 'UNCHANGED'; sourceModified: false }>(
    `/api/android-live/design/drafts/${encodeURIComponent(draftId)}/writeback/plan`, { projectRoot },
  )
}

export function getDesignWritebackPlan(projectRoot: string, planId: string) {
  return callDesignNode<{ schema: 'elon.ui-design-writeback-plan.v1'; plan: DesignWritebackPlan; contentEmbedded: false }>(
    `/api/android-live/design/writeback/plans/${encodeURIComponent(planId)}`, { projectRoot },
  )
}

export function decideDesignWritebackPlan(input: {
  projectRoot: string
  planId: string
  expectedPlanRevision: number
  decision: 'APPROVE' | 'REJECT'
  reason?: string
}) {
  const { planId, ...body } = input
  return callDesignNode<{ schema: 'elon.ui-design-writeback-plan.v1'; action: 'APPROVED' | 'REJECTED'; plan: DesignWritebackPlan; sourceModified: false }>(
    `/api/android-live/design/writeback/plans/${encodeURIComponent(planId)}/decision`, body,
  )
}

export function getDesignEventCheckpoint(projectRoot: string, consumerId: string, taskId: string) {
  return callDesignNode<{
    schema: 'elon.ui-design-event-checkpoint.v1'
    checkpoint?: DesignEventCheckpoint | null
    resumeAfterCursor: string
    revision: number
    contentEmbedded: false
  }>(
    `/api/android-live/design/events/checkpoints/${encodeURIComponent(consumerId)}/${encodeURIComponent(taskId)}`,
    { projectRoot },
  )
}

export function commitDesignEventCheckpoint(input: {
  projectRoot: string
  consumerId: string
  taskId: string
  cursor: string
  expectedRevision: number
}) {
  const { consumerId, taskId, ...body } = input
  return callDesignNode<{
    schema: 'elon.ui-design-event-checkpoint.v1'
    action: 'COMMITTED' | 'UNCHANGED'
    checkpoint: DesignEventCheckpoint
  }>(
    `/api/android-live/design/events/checkpoints/${encodeURIComponent(consumerId)}/${encodeURIComponent(taskId)}/commit`,
    body,
  )
}
