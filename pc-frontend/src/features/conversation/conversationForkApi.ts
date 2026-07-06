import { api } from '../../api/client'

export interface ConversationForkResponse {
  conversation_id: string
  source_conversation_id?: string
  source_message_id?: string
  title?: string | null
  copied_message_count?: number
}

interface ForkConversationRequest {
  message_id: string
  title?: string
}

export function forkTitleFromContent(content: string): string {
  const title = content.replace(/\s+/g, ' ').trim()
  if (!title) return '分叉会话'
  return `${title.slice(0, 24)}${title.length > 24 ? '...' : ''} · 分叉`
}

export async function forkAiConversation(
  conversationId: string,
  messageId: string,
  title?: string,
): Promise<ConversationForkResponse> {
  return api.post<ConversationForkResponse>(
    `/api/me/ai/conversations/${encodeURIComponent(conversationId)}/fork`,
    forkPayload(messageId, title),
  )
}

export async function forkProjectConversation(
  projectId: string,
  targetUserId: string,
  conversationId: string,
  messageId: string,
  title?: string,
): Promise<ConversationForkResponse> {
  return api.post<ConversationForkResponse>(
    `/api/projects/${encodeURIComponent(projectId)}/members/${encodeURIComponent(targetUserId)}/conversations/${encodeURIComponent(conversationId)}/fork`,
    forkPayload(messageId, title),
  )
}

function forkPayload(messageId: string, title?: string): ForkConversationRequest {
  return {
    message_id: messageId,
    title,
  }
}
