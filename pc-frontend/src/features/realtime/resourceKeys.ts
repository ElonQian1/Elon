export type RealtimeResourceKey = string

function cleanPart(value: string | undefined | null) {
  return String(value ?? '').trim()
}

function resourceKey(prefix: string, ...parts: Array<string | undefined | null>): RealtimeResourceKey {
  return [prefix, ...parts.map(cleanPart).filter(Boolean)].join(':')
}

export const realtimeResources = {
  projectSpace: (projectId: string) => resourceKey('project.space', projectId),
  projectMembers: (projectId: string) => resourceKey('project.members', projectId),
  channelMessages: (projectId: string, channelId: string) => resourceKey('channel.messages', projectId, channelId),
  conversationAny: (projectId: string, conversationId: string) => resourceKey('conversation.any', projectId, conversationId),
  conversationMessages: (projectId: string, targetUserId: string, conversationId: string) =>
    resourceKey('conversation.messages', projectId, targetUserId, conversationId),
  conversationList: (projectId: string, targetUserId: string) => resourceKey('conversation.list', projectId, targetUserId),
  taskAny: (projectId: string) => resourceKey('task.any', projectId),
  taskTimeline: (projectId: string, taskId: string) => resourceKey('task.timeline', projectId, taskId),
  groupAiMatter: (projectId: string, matterId: string) => resourceKey('group-ai.matter', projectId, matterId),
  presence: (userId: string) => resourceKey('presence', userId),
} as const
