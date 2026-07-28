import { clean } from '../../lib/utils'
import { presenceLabel } from './memberUtils'
import type { Channel, Message, Project, UserPresenceSettings } from './types'

const DIRECT_PC_CLI_STORAGE_KEY = 'elon_pc_project_direct_pc_cli'

export function initialDirectPcCliFromStorage(storage?: Storage | null): boolean {
  try {
    return storage?.getItem(DIRECT_PC_CLI_STORAGE_KEY) === '1'
  } catch {
    return false
  }
}

export function persistDirectPcCliSelection(storage: Storage | null | undefined, enabled: boolean): void {
  try {
    if (enabled) {
      storage?.setItem(DIRECT_PC_CLI_STORAGE_KEY, '1')
    } else {
      storage?.removeItem(DIRECT_PC_CLI_STORAGE_KEY)
    }
  } catch {
    // Ignore blocked storage; the selected value still works for the current session.
  }
}

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

export function delay(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms))
}

export function sameMessageList(left: Message[], right: Message[]): boolean {
  if (left === right) return true
  if (left.length !== right.length) return false
  for (let index = 0; index < left.length; index += 1) {
    if (messageFingerprint(left[index]) !== messageFingerprint(right[index])) return false
  }
  return true
}

function messageFingerprint(message: Message | undefined): string {
  if (!message) return ''
  return [
    clean(message.id),
    clean(message.kind ?? message.role ?? (message as Record<string, unknown>).message_kind ?? ''),
    clean(message.task_id ?? message.taskId ?? ''),
    clean(message.task_status ?? message.taskStatus ?? ''),
    clean(message.created_at),
    clean(message.content ?? message.text ?? ''),
  ].join('|')
}
