import type { MemberConversationEntry } from './memberConversationApi'
import { copyTextToClipboard as copyText } from '../../lib/clipboard'

export function buildMemberConversationDeepLink(conversation: MemberConversationEntry): string {
  if (typeof window === 'undefined') return ''
  const url = new URL(window.location.href)
  if (!url.pathname.startsWith('/pc') || url.pathname === '/pc/login') {
    url.pathname = '/pc/'
  }
  url.hash = ''
  url.searchParams.set('conversation', conversation.id)
  if (conversation.project_id) url.searchParams.set('project', conversation.project_id)
  if (conversation.user_id) url.searchParams.set('member', conversation.user_id)
  return url.toString()
}

export function copyTextToClipboard(text: string): Promise<boolean> {
  return copyText(text)
}
