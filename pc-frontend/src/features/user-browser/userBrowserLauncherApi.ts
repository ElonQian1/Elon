import { openCommerceApi } from '../open-commerce/openCommerceApi'
import { openCommerceClientApi } from '../open-commerce/openCommerceClientApi'
import type { ConsumerDiscoveryMatch } from '../open-commerce/openCommerceClientTypes'

export const CHATGPT_BROWSER_CAPABILITY = 'browser.chatgpt.session.launch'

export interface UserBrowserLaunch {
  sessionId: string
  launchUrl: string
  expiresAt?: string
}

export async function discoverChatGptBrowser(): Promise<ConsumerDiscoveryMatch | null> {
  const response = await openCommerceClientApi.discover({
    capability_key: CHATGPT_BROWSER_CAPABILITY,
    capability_kind: 'action',
    access_level: 'public',
    requester_app_id: 'pc-web',
    preferences: { categories: [], tags: [], prefer_public: true },
    limit: 10,
  })
  const matches = response.matches.filter(
    (match) => match.capability.capability_key === CHATGPT_BROWSER_CAPABILITY
      && match.capability.kind === 'action'
      && match.capability.access_level === 'public'
      && match.capability.source.kind === 'merchant_runtime',
  )
  if (matches.length > 1) {
    throw new Error('发现多个 ChatGPT 浏览器模块，请由平台管理员保留唯一可信来源。')
  }
  return matches[0] ?? null
}

export async function launchChatGptBrowser(
  match: ConsumerDiscoveryMatch,
): Promise<UserBrowserLaunch> {
  const request = {
    merchant_id: match.merchant.id,
    capability_key: CHATGPT_BROWSER_CAPABILITY,
    requester_app_id: 'pc-web',
    idempotency_key: `chatgpt-browser-${crypto.randomUUID()}`,
    input: { target: 'chatgpt' },
  }
  const prepared = await openCommerceApi.prepareActionConfirmation(request)
  const confirmed = await openCommerceApi.confirmActionConfirmation(prepared.id)
  const invocation = await openCommerceApi.invoke({
    ...request,
    action_confirmation_id: confirmed.id,
  })
  const result = asRecord(invocation.result)
  if (result.schema !== 'yilong.user_browser.launch.v1'
    || result.target !== 'chatgpt'
    || result.ticket_single_use !== true) {
    throw new Error('模块服务返回了不受支持的浏览器会话协议')
  }
  const launchUrl = requiredText(result.launch_url, '模块服务未返回浏览器入口')
  assertSafeLaunchUrl(launchUrl)
  return {
    sessionId: requiredText(result.session_id, '模块服务未返回会话编号'),
    launchUrl,
    expiresAt: optionalText(result.expires_at),
  }
}

function asRecord(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error('模块服务返回了无效的会话结果')
  }
  return value as Record<string, unknown>
}

function requiredText(value: unknown, message: string): string {
  if (typeof value !== 'string' || value.trim() === '') throw new Error(message)
  return value.trim()
}

function optionalText(value: unknown): string | undefined {
  return typeof value === 'string' && value.trim() ? value.trim() : undefined
}

function assertSafeLaunchUrl(value: string): void {
  const url = new URL(value)
  const localHttp = url.protocol === 'http:'
    && (url.hostname === 'localhost' || url.hostname === '127.0.0.1')
  if (url.protocol !== 'https:' && !localHttp) {
    throw new Error('浏览器入口必须使用 HTTPS')
  }
  if (url.username || url.password || !new URLSearchParams(url.hash.slice(1)).get('ticket')) {
    throw new Error('浏览器入口缺少安全的单次票据')
  }
}
