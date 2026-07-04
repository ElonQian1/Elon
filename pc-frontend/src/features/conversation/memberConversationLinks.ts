import type { MemberConversationEntry } from './memberConversationApi'

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

export async function copyTextToClipboard(text: string): Promise<boolean> {
  if (!text) return false
  if (fallbackCopyText(text)) return true
  try {
    await window.navigator.clipboard.writeText(text)
    return true
  } catch {
    return false
  }
}

function fallbackCopyText(text: string): boolean {
  const textarea = document.createElement('textarea')
  textarea.value = text
  textarea.setAttribute('readonly', 'true')
  textarea.style.position = 'fixed'
  textarea.style.left = '-9999px'
  document.body.appendChild(textarea)
  textarea.select()
  let copied = false
  try {
    copied = document.execCommand('copy')
  } catch {
    copied = false
  } finally {
    document.body.removeChild(textarea)
  }
  return copied
}
