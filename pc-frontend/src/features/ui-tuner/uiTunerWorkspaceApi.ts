import { api } from '../../api/client'
import type { UiTunerCodexContextPack } from './contextPack'
import {
  normalizeUiTunerWorkspace,
  type LegacyUiTunerModuleMemory,
  type UiTunerModuleMemoryRecord,
  type UiTunerProjectSessionRecord,
  type UiTunerWorkspaceResponse,
} from './projectSessions'

function moduleBase(projectId: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/modules/ui-tuner`
}

export async function importLegacyUiTunerWorkspace(
  projectId: string,
  memory: LegacyUiTunerModuleMemory,
) {
  const response = await api.post<UiTunerWorkspaceResponse>(
    `${moduleBase(projectId)}/workspace/legacy-import`,
    memory,
  )
  return normalizeUiTunerWorkspace(response)
}

export async function loadUiTunerWorkspace(projectId: string) {
  const response = await api.get<UiTunerWorkspaceResponse>(`${moduleBase(projectId)}/workspace`)
  return normalizeUiTunerWorkspace(response)
}

export async function createUiTunerContextArtifact(input: {
  projectId: string
  conversationId: string
  userIntent: string
  pack: UiTunerCodexContextPack
}) {
  return api.post<{ id: string; payloadSha256: string }>(
    `${moduleBase(input.projectId)}/context-artifacts`,
    {
      conversationId: input.conversationId,
      userIntent: input.userIntent,
      payload: input.pack,
      selectedElementName: input.pack.selectedElement?.name,
      resourceId: input.pack.runtimeBinding.resourceId,
      sourceFile: input.pack.runtimeBinding.sourceFile,
    },
  )
}

export async function forkUiTunerConversation(input: {
  projectId: string
  conversationId: string
  title: string
  selectedElementName?: string
}) {
  const session = await api.post<Omit<UiTunerProjectSessionRecord, 'id'>>(
    `${moduleBase(input.projectId)}/conversations/${encodeURIComponent(input.conversationId)}/fork`,
    {
      title: input.title,
      selectedElementName: input.selectedElementName,
    },
  )
  return { ...session, id: session.conversationId } satisfies UiTunerProjectSessionRecord
}

export async function reviewUiTunerMemory(input: {
  projectId: string
  memoryId: string
  decision: 'accepted' | 'rejected'
  scopeType?: 'user' | 'project'
}) {
  return api.patch<UiTunerModuleMemoryRecord>(
    `${moduleBase(input.projectId)}/memories/${encodeURIComponent(input.memoryId)}`,
    {
      decision: input.decision,
      scopeType: input.scopeType ?? 'user',
    },
  )
}
