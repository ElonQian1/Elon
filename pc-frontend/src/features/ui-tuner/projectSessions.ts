export interface UiTunerProjectSessionRecord {
  id: string
  conversationId: string
  title: string
  isCanonical: boolean
  parentConversationId?: string | null
  sourceMessageId?: string | null
  sourceCheckpointId?: string | null
  selectedElementName?: string | null
  taskId?: string | null
  status: string
  createdAt: string
  updatedAt: string
}

export interface UiTunerModuleMemoryRecord {
  id: string
  ownerUserId?: string | null
  scopeType: 'user' | 'project'
  category: string
  content: string
  status: 'candidate' | 'accepted' | 'rejected' | 'superseded'
  importance: number
  sourceConversationId?: string | null
  sourceMessageId?: string | null
  sourceTaskId?: string | null
  reviewedBy?: string | null
  reviewedAt?: string | null
  createdAt: string
  updatedAt: string
}

export interface UiTunerModuleWorkspace {
  projectId: string
  userId: string
  moduleKey: 'ui-tuner'
  canonicalConversationId: string
  activeConversationId: string
  stableSummary: string
  memoryRevision: number
  lastCheckpointId?: string | null
  createdAt: string
  updatedAt: string
}

export interface UiTunerModuleCheckpoint {
  id: string
  conversationId: string
  sourceMessageId: string
  taskId: string
  contextArtifactId?: string | null
  memoryRevision: number
  status: string
  summary: string
  createdAt: string
}

export interface UiTunerWorkspaceResponse {
  workspace: UiTunerModuleWorkspace
  sessions: UiTunerProjectSessionRecord[]
  memories: UiTunerModuleMemoryRecord[]
  latestCheckpoint?: UiTunerModuleCheckpoint | null
}

export interface LegacyUiTunerModuleMemory {
  stableSummary: string
  acceptedDecisions: string[]
  openQuestions: string[]
  preferredStandards: string[]
}

const LEGACY_MEMORY_KEY = 'elon.uiTuner.moduleMemory.v1'
const LEGACY_SESSIONS_KEY = 'elon.uiTuner.projectSessions.v1'

export function normalizeUiTunerWorkspace(response: UiTunerWorkspaceResponse): UiTunerWorkspaceResponse {
  return {
    ...response,
    sessions: response.sessions.map((session) => ({
      ...session,
      id: session.id || session.conversationId,
      taskId: session.taskId ?? null,
    })),
  }
}

export function readLegacyUiTunerModuleMemory(projectId: string): LegacyUiTunerModuleMemory | null {
  if (typeof window === 'undefined') return null
  try {
    const sessions = JSON.parse(window.localStorage.getItem(LEGACY_SESSIONS_KEY) || '[]') as Array<{ projectId?: string }>
    if (!sessions.some((session) => session.projectId === projectId)) return null
    const memory = JSON.parse(window.localStorage.getItem(LEGACY_MEMORY_KEY) || 'null') as Partial<LegacyUiTunerModuleMemory> | null
    if (!memory?.stableSummary) return null
    return {
      stableSummary: memory.stableSummary,
      acceptedDecisions: compactLegacyValues(memory.acceptedDecisions),
      openQuestions: compactLegacyValues(memory.openQuestions),
      preferredStandards: compactLegacyValues(memory.preferredStandards),
    }
  } catch {
    return null
  }
}

export function clearLegacyUiTunerModuleMemory() {
  if (typeof window === 'undefined') return
  window.localStorage.removeItem(LEGACY_MEMORY_KEY)
  window.localStorage.removeItem(LEGACY_SESSIONS_KEY)
}

function compactLegacyValues(values?: string[]) {
  return Array.from(new Set((values ?? []).map((value) => value.trim()).filter(Boolean))).slice(0, 24)
}
