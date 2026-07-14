import { api } from '../../../api/client'

export type UiLearnedRoute = 'ui' | 'non_ui'
export type UiRouteLearningStatus = 'candidate' | 'active' | 'revoked'

export interface UiRouteLearningAlias {
  id: string
  sampleText: string
  source: 'controlled_vocabulary' | 'user_override' | 'execution_verified' | 'codex_candidate'
  status: UiRouteLearningStatus
  hitCount: number
  conflictCount: number
  updatedAt: string
}

export interface UiRouteLearningEntry {
  id: string
  sampleText: string
  learnedRoute: UiLearnedRoute
  status: UiRouteLearningStatus
  source: 'codex_proposal' | 'user_override' | 'runtime_verified' | 'execution_verified' | 'admin'
  confidence: number
  evidenceCount: number
  conflictCount: number
  hitCount: number
  conceptKey?: string
  conceptLabel?: string
  conceptVersion?: number
  clusterHitCount: number
  aliasCount: number
  aliases: UiRouteLearningAlias[]
  updatedAt: string
}

function base(projectId: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/modules/ui-tuner/route-learning`
}

export async function listUiRouteLearning(projectId: string) {
  const response = await api.get<{ entries: UiRouteLearningEntry[] }>(base(projectId))
  return response.entries
}

export function confirmUiRouteLearning(input: {
  projectId: string
  message: string
  route: UiLearnedRoute
  reason?: string
}) {
  return api.post<UiRouteLearningEntry>(base(input.projectId), {
    message: input.message,
    route: input.route,
    reason: input.reason,
  })
}

export function revokeUiRouteLearning(input: {
  projectId: string
  entryId: string
  reason?: string
}) {
  return api.patch<UiRouteLearningEntry>(
    `${base(input.projectId)}/${encodeURIComponent(input.entryId)}`,
    { reason: input.reason },
  )
}
