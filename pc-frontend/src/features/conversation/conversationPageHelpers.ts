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
  const messages = data.messages ?? []
  return appendTaskRecoverySnapshots(projectId, channelId, messages)
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

interface TaskSnapshotResponse {
  task?: {
    id?: string
    status?: string
    error?: string | null
    apk_url?: string | null
    apkUrl?: string | null
  }
  pc_req_id?: string | null
  pcReqId?: string | null
  agent_id?: string | null
  agentId?: string | null
  last_event_seq?: number
  lastEventSeq?: number
  attach?: Record<string, unknown> | null
  cloud_attach?: Record<string, unknown> | null
  cloudAttach?: Record<string, unknown> | null
  resume?: Record<string, unknown> | null
  approval_state?: Record<string, unknown> | null
  approvalState?: Record<string, unknown> | null
  local_journal?: LocalJournalProbe | null
  localJournal?: LocalJournalProbe | null
}

interface LocalJournalProbe {
  status?: string
  reachable?: boolean
  pc_req_id?: string | null
  pcReqId?: string | null
  agent_id?: string | null
  agentId?: string | null
  node_display_name?: string | null
  nodeDisplayName?: string | null
  cli?: string | null
  message?: string
  error?: string | null
  snapshot?: Record<string, unknown> | null
}

const SNAPSHOT_RECOVERY_TASK_LIMIT = 4

async function appendTaskRecoverySnapshots(projectId: string, channelId: string, messages: Message[]): Promise<Message[]> {
  const taskIds = recoveryCandidateTaskIds(messages)
  if (taskIds.length === 0) return messages
  const snapshots = await Promise.all(taskIds.map(async (taskId) => {
    try {
      return await api.get<TaskSnapshotResponse>(
        `/api/projects/${encodeURIComponent(projectId)}/channels/${encodeURIComponent(channelId)}/ai-tasks/${encodeURIComponent(taskId)}/snapshot?limit=40`,
      )
    } catch {
      return null
    }
  }))
  const synthetic = snapshots
    .map((snapshot) => snapshot ? recoverySnapshotMessage(snapshot) : null)
    .filter((message): message is Message => Boolean(message))
  if (synthetic.length === 0) return messages
  const existing = new Set(messages.map((message) => message.id))
  return [
    ...messages,
    ...synthetic.filter((message) => !existing.has(message.id)),
  ]
}

function recoveryCandidateTaskIds(messages: Message[]): string[] {
  const taskIds = new Set<string>()
  const terminalIds = new Set<string>()
  const recoveryIds = new Set<string>()
  for (const message of messages) {
    const taskId = clean(message.task_id ?? message.taskId ?? '')
    if (!taskId) continue
    taskIds.add(taskId)
    const kind = clean(message.kind ?? message.role ?? '').toLowerCase()
    const status = clean(message.task_status ?? message.taskStatus ?? '').toLowerCase()
    const error = clean(message.task_error ?? message.taskError ?? '')
    if (kind === 'ai_result' || ['done', 'completed', 'success', 'failed', 'error', 'canceled', 'cancelled'].includes(status)) {
      terminalIds.add(taskId)
    }
    if (['running', 'recovering', 'interrupted'].includes(status) || /恢复|重启|journal|sidecar|CliDone|通信/.test(error)) {
      recoveryIds.add(taskId)
    }
  }
  return Array.from(taskIds)
    .filter((taskId) => recoveryIds.has(taskId) && !terminalIds.has(taskId))
    .slice(0, SNAPSHOT_RECOVERY_TASK_LIMIT)
}

function recoverySnapshotMessage(snapshot: TaskSnapshotResponse): Message | null {
  const taskId = clean(snapshot.task?.id ?? '')
  const localJournal = snapshot.local_journal ?? snapshot.localJournal ?? null
  if (!taskId || !localJournal) return null
  const journalStatus = clean(localJournal.status ?? '')
  const message = clean(localJournal.message ?? '')
  if (!journalStatus || journalStatus === 'missing_pc_req_id') return null

  const resume = snapshot.resume ?? null
  const attach = snapshot.attach ?? snapshot.cloud_attach ?? snapshot.cloudAttach ?? null
  const approvalState = snapshot.approval_state ?? snapshot.approvalState ?? null
  const nextAction = clean(resume?.next_action ?? resume?.nextAction ?? '')
  const phase = journalStatus === 'available' && nextAction !== 'continue_from_snapshot'
    ? 'connection_recovering'
    : 'resume_required'
  const pcReqId = clean(snapshot.pc_req_id ?? snapshot.pcReqId ?? localJournal.pc_req_id ?? localJournal.pcReqId ?? '')
  const agentId = clean(snapshot.agent_id ?? snapshot.agentId ?? localJournal.agent_id ?? localJournal.agentId ?? '')
  const lastEventSeq = Number(snapshot.last_event_seq ?? snapshot.lastEventSeq ?? 0)
  const content = JSON.stringify({
    type: 'runtime_status',
    phase,
    runtime: clean(localJournal.cli ?? '') || 'Codex',
    message: message || recoverySnapshotFallbackMessage(journalStatus),
    pc_req_id: pcReqId || undefined,
    agent_id: agentId || undefined,
    local_journal: localJournal,
    attach,
    resume,
    approval_state: approvalState,
  })
  return {
    id: `synthetic-local-journal-${taskId}-${journalStatus}-${lastEventSeq}`,
    kind: 'ai_progress',
    content,
    task_id: taskId,
    task_status: snapshot.task?.status,
    task_error: snapshot.task?.error ?? undefined,
    task_apk_url: snapshot.task?.apk_url ?? snapshot.task?.apkUrl ?? undefined,
    created_at: '',
    local_journal: localJournal,
    task_attach: attach,
    task_resume: resume,
    approval_state: approvalState,
    pc_req_id: pcReqId,
    agent_id: agentId,
    last_event_seq: lastEventSeq,
  }
}

function recoverySnapshotFallbackMessage(status: string): string {
  if (status === 'available') return '已读取本机 journal 恢复合同。'
  if (status === 'agent_offline_or_timeout') return '暂时不能读取本机 journal；Win 端重连后会继续恢复。'
  if (status === 'missing_agent_id') return '云端缺少节点 ID，只能使用云端快照继续。'
  return '本机 journal 恢复状态暂不可用。'
}
