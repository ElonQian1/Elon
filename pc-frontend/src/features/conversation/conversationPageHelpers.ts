import { api } from '../../api/client'
import { clean } from '../../lib/utils'
import type {
  Channel,
  ChannelMessagesResponse,
  Message,
  Project,
  UserPresenceSettings,
} from './types'
import { presenceLabel } from './memberUtils'

export function titleFromMessage(message: string): string {
  const title = message.replace(/\s+/g, ' ').trim()
  if (!title) return '新会话'
  return title.length > 24 ? `${title.slice(0, 24)}...` : title
}

export function mergeProjectRecords(listedProject?: Project, spaceProject?: Project | null): Project | undefined {
  if (!listedProject) return spaceProject ?? undefined
  if (!spaceProject) return listedProject
  return {
    ...listedProject,
    ...spaceProject,
    source_type: spaceProject.source_type ?? listedProject.source_type,
    workspace_path: spaceProject.workspace_path ?? listedProject.workspace_path,
    storage_worktree_path: spaceProject.storage_worktree_path ?? listedProject.storage_worktree_path,
    node_id: spaceProject.node_id ?? listedProject.node_id,
    role: spaceProject.role ?? listedProject.role,
    my_role: spaceProject.my_role ?? listedProject.my_role,
    runtime_permission: spaceProject.runtime_permission ?? listedProject.runtime_permission,
  }
}

export function channelAllowsAiStart(channel?: Channel | null): boolean {
  if (!channel) return false
  if (channel.kind !== 'ai_development') return false
  const permissions = channel.permissions
  if (!permissions) return true
  return Boolean(permissions.can_start_ai ?? permissions.canStartAi)
}

export function projectRoleCanAutoBind(role: string): boolean {
  return role === 'owner'
}

export function projectRoleLabel(role: string): string {
  if (role === 'owner') return 'Owner'
  if (role === 'admin') return 'Admin'
  if (role === 'editor') return '协作者'
  if (role === 'member') return '成员'
  if (role === 'observer') return '只读'
  return role || '未知角色'
}

export function normalizeOwnPresenceStatus(status: string): string {
  const value = clean(status).toLowerCase()
  if (value === 'idle' || value === 'dnd' || value === 'invisible' || value === 'offline') return value
  return 'online'
}

export function ownPresenceSummary(presence: UserPresenceSettings | null, status: string): string {
  const extras = [
    clean(presence?.activity ?? ''),
    clean(presence?.custom_status ?? ''),
  ].filter(Boolean)
  return [presenceLabel(status), ...extras].join(' · ')
}

export function shortNodeId(nodeId: string): string {
  const cleanId = clean(nodeId)
  if (cleanId.length <= 18) return cleanId
  return `${cleanId.slice(0, 11)}…${cleanId.slice(-6)}`
}

export async function loadAiDevelopmentTaskMessages(projectId: string, channelId: string): Promise<Message[]> {
  if (!projectId || !channelId) return []
  const data = await api.get<ChannelMessagesResponse>(
    `/api/projects/${encodeURIComponent(projectId)}/channels/${encodeURIComponent(channelId)}/messages?limit=200`,
  )
  return data.messages ?? []
}

export function conversationMessageCacheKey(projectId: string, targetUserId: string, conversationId: string): string {
  return `${projectId}::${targetUserId}::${conversationId}`
}

export function taskMessageCacheKey(projectId: string, channelId: string): string {
  return `${projectId}::${channelId}`
}

export function delay(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms))
}
