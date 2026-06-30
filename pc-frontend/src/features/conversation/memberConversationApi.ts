import { api } from '../../api/client'
import type { User } from '../../store/auth'
import type { Message, ProjectMember } from './types'

export interface MemberConversationTarget {
  userId: string
  account: string
  avatarDataUrl?: string | null
}

export interface MemberConversationEntry {
  id: string
  project_id?: string
  user_id?: string
  user_account?: string
  title?: string | null
  status?: string
  is_public?: boolean
  message_count?: number
  task_count?: number
  last_message?: string | null
  last_message_role?: string | null
  last_message_at?: string | null
  last_task_status?: string | null
  created_at?: string
  updated_at?: string
}

export type MemberConversationMessage = Message & {
  conversation_id?: string | null
  conversationId?: string | null
  sender_name?: string | null
  senderName?: string | null
  outgoing?: boolean
}

export function targetFromUser(user: User | null): MemberConversationTarget | null {
  if (!user?.id) return null
  return {
    userId: user.id,
    account: user.nickname || user.account || user.id,
    avatarDataUrl: user.avatar_data_url,
  }
}

export function targetFromProjectMember(member: ProjectMember): MemberConversationTarget {
  return {
    userId: member.user_id,
    account: member.account || member.user_id,
    avatarDataUrl: member.avatar_data_url,
  }
}

export function sameConversationTarget(
  left: MemberConversationTarget | null,
  right: MemberConversationTarget | null,
): boolean {
  return !!left && !!right && left.userId === right.userId
}

export function targetDisplayName(target: MemberConversationTarget | null): string {
  return target?.account || '成员'
}

export async function listMemberConversations(
  projectId: string,
  targetUserId: string,
): Promise<MemberConversationEntry[]> {
  const data = await api.get<{ conversations?: MemberConversationEntry[] }>(
    `/api/projects/${encodeURIComponent(projectId)}/members/${encodeURIComponent(targetUserId)}/conversations?limit=50`,
  )
  return data.conversations ?? []
}

export async function listMemberConversationMessages(
  projectId: string,
  targetUserId: string,
  conversationId: string,
): Promise<MemberConversationMessage[]> {
  const data = await api.get<{ messages?: MemberConversationMessage[] }>(
    `/api/projects/${encodeURIComponent(projectId)}/members/${encodeURIComponent(targetUserId)}/conversations/${encodeURIComponent(conversationId)}/messages?limit=120`,
  )
  return data.messages ?? []
}

export async function sendMemberConversationDiscussion(
  projectId: string,
  targetUserId: string,
  conversationId: string,
  content: string,
): Promise<MemberConversationMessage> {
  const data = await api.post<{ message: MemberConversationMessage }>(
    `/api/projects/${encodeURIComponent(projectId)}/members/${encodeURIComponent(targetUserId)}/conversations/${encodeURIComponent(conversationId)}/messages`,
    { content },
  )
  return data.message
}
