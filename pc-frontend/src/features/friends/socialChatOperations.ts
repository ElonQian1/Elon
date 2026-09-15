import { getAuthToken } from '../../api/client'
import { resolveApiUrl } from '../../api/runtime'
import type { ActiveConversation, SocialMessage } from './socialMessageTypes'

export const messageEndpoint = (conversation: ActiveConversation) => `/api/me/${conversation.kind === 'group' ? 'groups' : 'friends'}/${encodeURIComponent(conversation.id)}/messages`
export const isRecalled = (message: SocialMessage) => !!(message.recalled_at || message.recalledAt)
export const isPending = (message: SocialMessage) => message.id.startsWith('tmp-')
export const canRecall = (message: SocialMessage, own: boolean, now = Date.now()) => own && !isRecalled(message) && !isPending(message) && Number.isFinite(Date.parse(message.created_at)) && now - Date.parse(message.created_at) <= 60000
export const messageText = (message: SocialMessage) => message.content.trim() || (message.attachments ?? []).map(a => `[${a.kind === 'audio' ? '语音' : '附件'}] ${a.display_name || a.file_name || '附件'}`).join('\n')

export function quoteText(message: SocialMessage, author: string) {
  const preview = Array.from(messageText(message)).slice(0, 400).join('')
  const name = author.replace(/[\r\n]/g, ' ').slice(0, 80)
  return `> 引用 ${name}${(message.revision ?? 1) > 1 ? ` · 第 ${message.revision} 版` : ''}\n${preview.split('\n').map(line => `> ${line}`).join('\n')}`
}

export class SocialRequestError extends Error {
  constructor(message: string, readonly uncertain: boolean, readonly status = 0) { super(message) }
}

/** No automatic write retry: without server idempotency a timeout can still mean delivered. */
export async function socialRequest<T>(path: string, init: RequestInit = {}, timeoutMs = 15000): Promise<T> {
  const controller = new AbortController()
  const parent = init.signal
  const abort = () => controller.abort()
  if (parent?.aborted) abort()
  else parent?.addEventListener('abort', abort, { once: true })
  const timer = setTimeout(abort, timeoutMs)
  const token = getAuthToken()
  const writing = !!init.method && init.method !== 'GET'
  try {
    const response = await fetch(resolveApiUrl(path), { ...init, signal: controller.signal, cache: 'no-store',
      headers: { ...(token ? { Authorization: `Bearer ${token}` } : {}), ...init.headers } })
    if (!response.ok) {
      const body = await response.json().catch(() => ({}))
      throw new SocialRequestError(String(body.error || body.message || `请求失败（${response.status}）`), false, response.status)
    }
    if (response.status === 204) return undefined as T
    return await response.json() as T
  } catch (error) {
    if (error instanceof SocialRequestError) throw error
    throw new SocialRequestError(writing ? '操作结果未确认，请同步并核对消息后再决定是否重试' : '读取失败或超时，请重试', writing)
  } finally { clearTimeout(timer); parent?.removeEventListener('abort', abort) }
}

export function sendSocialMessage(conversation: ActiveConversation, message: Pick<SocialMessage, 'content' | 'attachments'>, signal?: AbortSignal) {
  return socialRequest<{ message: SocialMessage }>(messageEndpoint(conversation), {
    method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(message), signal,
  })
}
