import { realtimeResources, type RealtimeResourceKey } from './resourceKeys'

export const REALTIME_SERVER_TYPES = {
  projectMessageUpdated: 'project_message_updated',
  projectTaskDone: 'project_task_done',
  projectAiMatterEvent: 'project_ai_matter_event',
  projectMembersUpdated: 'project_members_updated',
  presence: 'presence',
} as const

export const REALTIME_DOM_EVENTS = {
  realtime: 'elon:realtime-event',
  projectMessageUpdated: 'elon:project-message-updated',
  projectTaskDone: 'elon:project-task-done',
  projectAiMatterEvent: 'elon:project-ai-matter-event',
  projectMembersUpdated: 'elon:project-members-updated',
  presence: 'elon:presence',
} as const

export type RealtimeServerType = typeof REALTIME_SERVER_TYPES[keyof typeof REALTIME_SERVER_TYPES]

export interface RealtimeEvent {
  type: RealtimeServerType
  projectId?: string
  channelId?: string
  conversationId?: string
  taskId?: string
  matterId?: string
  targetUserId?: string
  userId?: string
  raw: Record<string, unknown>
  resources: RealtimeResourceKey[]
}

export function stringField(value: unknown): string | undefined {
  const text = String(value ?? '').trim()
  return text ? text : undefined
}

export function resourcesForRealtimeEvent(event: {
  type: string
  projectId?: string
  channelId?: string
  conversationId?: string
  taskId?: string
  matterId?: string
  targetUserId?: string
  userId?: string
}): RealtimeResourceKey[] {
  const resources = new Set<RealtimeResourceKey>()
  const addProjectSpace = () => {
    if (event.projectId) resources.add(realtimeResources.projectSpace(event.projectId))
  }

  switch (event.type) {
    case REALTIME_SERVER_TYPES.projectMessageUpdated:
      addProjectSpace()
      if (event.projectId && event.channelId) {
        resources.add(realtimeResources.channelMessages(event.projectId, event.channelId))
      }
      if (event.projectId && event.conversationId) {
        resources.add(realtimeResources.conversationAny(event.projectId, event.conversationId))
      }
      if (event.projectId && event.taskId) {
        resources.add(realtimeResources.taskTimeline(event.projectId, event.taskId))
        resources.add(realtimeResources.taskAny(event.projectId))
      }
      break
    case REALTIME_SERVER_TYPES.projectTaskDone:
      addProjectSpace()
      if (event.projectId) resources.add(realtimeResources.taskAny(event.projectId))
      if (event.projectId && event.taskId) resources.add(realtimeResources.taskTimeline(event.projectId, event.taskId))
      if (event.projectId && event.conversationId) {
        resources.add(realtimeResources.conversationAny(event.projectId, event.conversationId))
      }
      break
    case REALTIME_SERVER_TYPES.projectAiMatterEvent:
      addProjectSpace()
      if (event.projectId && event.matterId) resources.add(realtimeResources.groupAiMatter(event.projectId, event.matterId))
      break
    case REALTIME_SERVER_TYPES.projectMembersUpdated:
      addProjectSpace()
      if (event.projectId) resources.add(realtimeResources.projectMembers(event.projectId))
      break
    case REALTIME_SERVER_TYPES.presence:
      if (event.userId) resources.add(realtimeResources.presence(event.userId))
      break
    default:
      break
  }

  return Array.from(resources)
}

export function normalizeRealtimeEvent(raw: Record<string, unknown>): RealtimeEvent | null {
  const type = stringField(raw.type)
  if (!type || !Object.values(REALTIME_SERVER_TYPES).includes(type as RealtimeServerType)) return null

  const event = {
    type: type as RealtimeServerType,
    projectId: stringField(raw.projectId),
    channelId: stringField(raw.channelId),
    conversationId: stringField(raw.conversationId),
    taskId: stringField(raw.taskId),
    matterId: stringField(raw.matterId),
    targetUserId: stringField(raw.targetUserId),
    userId: stringField(raw.userId),
    raw,
    resources: [] as RealtimeResourceKey[],
  }
  event.resources = resourcesForRealtimeEvent(event)
  return event
}

export function legacyDomEventNameForType(type: RealtimeServerType): string {
  switch (type) {
    case REALTIME_SERVER_TYPES.projectMessageUpdated:
      return REALTIME_DOM_EVENTS.projectMessageUpdated
    case REALTIME_SERVER_TYPES.projectTaskDone:
      return REALTIME_DOM_EVENTS.projectTaskDone
    case REALTIME_SERVER_TYPES.projectAiMatterEvent:
      return REALTIME_DOM_EVENTS.projectAiMatterEvent
    case REALTIME_SERVER_TYPES.projectMembersUpdated:
      return REALTIME_DOM_EVENTS.projectMembersUpdated
    case REALTIME_SERVER_TYPES.presence:
      return REALTIME_DOM_EVENTS.presence
    default:
      return REALTIME_DOM_EVENTS.realtime
  }
}
