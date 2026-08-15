import { compactDisplayMessageContent } from '../../lib/messageDisplay'
import { formatTime } from '../../lib/utils'

export interface AiConversation {
  id: string
  title?: string
  updated_at?: string
  message_count?: number
  project_id?: string
  project_name?: string
  first_user_message?: string
}

const GENERIC_AI_TITLES = new Set(['普通聊天会话', '新对话', 'AI 对话', '一龙 AI 对话'])

export function compactConversationText(text?: string, maxLength = 28) {
  return compactDisplayMessageContent(text, maxLength)
}

export function isGenericConversationTitle(title?: string) {
  const normalized = (title ?? '').trim()
  return !normalized || GENERIC_AI_TITLES.has(normalized)
}

export function displayProjectName(name?: string) {
  const normalized = (name ?? '').trim()
  if (!normalized || GENERIC_AI_TITLES.has(normalized)) return '一龙 AI'
  return normalized
}

export function makeConversationTitle(text: string) {
  return compactConversationText(text, 32) || '新对话'
}

export function conversationTitle(conversation: AiConversation | undefined, previews: Record<string, string>) {
  if (!conversation) return '新对话'
  const title = conversation.title?.trim()
  if (title && !GENERIC_AI_TITLES.has(title)) return title
  return compactConversationText(conversation.first_user_message, 32)
    || compactConversationText(previews[conversation.id], 32)
    || title
    || '新对话'
}

export function formatHistoryAge(input?: string) {
  if (!input) return ''
  const time = new Date(input).getTime()
  if (!Number.isFinite(time)) return ''
  const diffMs = Math.max(0, Date.now() - time)
  const minute = 60 * 1000
  const hour = 60 * minute
  const day = 24 * hour
  const week = 7 * day
  const month = 30 * day
  if (diffMs < minute) return '刚刚'
  if (diffMs < hour) return `${Math.floor(diffMs / minute)}分`
  if (diffMs < day) return `${Math.floor(diffMs / hour)}小时`
  if (diffMs < week) return `${Math.floor(diffMs / day)}天`
  if (diffMs < month) return `${Math.floor(diffMs / week)}周`
  return formatTime(input)
}
