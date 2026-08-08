import { getDesktopInvoke } from '../shell/desktopShell'
import { UNIFIED_AI_PROTOCOL } from './unifiedAiProtocol'

export interface LocalAiWebProvider {
  id: string
  displayName: string
  startHost: string
  loginMode: 'manual_web'
  profileScope: 'local_owner_provider'
  rendererProtocol: typeof UNIFIED_AI_PROTOCOL
  rendererStatus: 'reserved' | 'active'
}

export interface LocalAiWebSession {
  providerId: string
  windowLabel: string
  status: 'created' | 'focused'
  profileScope: 'local_owner_provider'
  cookieAccess: 'webview_only'
  rendererProtocol: typeof UNIFIED_AI_PROTOCOL
  rendererStatus: 'reserved' | 'active'
}

export interface ClearedLocalAiWebSession {
  providerId: string
  status: 'cleared'
}

type LocalAiBrowserErrorCode = 'upgrade_required' | 'desktop_required' | 'invoke_failed'

class LocalAiBrowserError extends Error {
  constructor(readonly code: LocalAiBrowserErrorCode, message: string) {
    super(message)
    this.name = 'LocalAiBrowserError'
  }
}

export function isLocalAiBrowserAvailable(): boolean {
  return getDesktopInvoke() !== null
}

export async function listLocalAiWebProviders(): Promise<LocalAiWebProvider[]> {
  const providers = await invokeDesktop<LocalAiWebProvider[]>('list_local_ai_web_providers')
  if (!Array.isArray(providers)) throw new Error('桌面壳返回了无效的 AI 网页厂商列表。')
  for (const provider of providers) assertProvider(provider)
  return providers
}

export async function openLocalAiWebSession(
  providerId: string,
  ownerKey: string,
): Promise<LocalAiWebSession> {
  assertIdentity(providerId, ownerKey)
  const session = await invokeDesktop<LocalAiWebSession>('open_local_ai_web_session', {
    providerId,
    ownerKey,
  })
  if (session.providerId !== providerId
    || session.profileScope !== 'local_owner_provider'
    || session.cookieAccess !== 'webview_only'
    || session.rendererProtocol !== UNIFIED_AI_PROTOCOL) {
    throw new Error('桌面壳返回了不受支持的本地会话协议。')
  }
  return session
}

export async function clearLocalAiWebSession(
  providerId: string,
  ownerKey: string,
): Promise<ClearedLocalAiWebSession> {
  assertIdentity(providerId, ownerKey)
  return invokeDesktop<ClearedLocalAiWebSession>('clear_local_ai_web_session', {
    providerId,
    ownerKey,
  })
}

async function invokeDesktop<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const invoke = getDesktopInvoke()
  if (!invoke) {
    throw new LocalAiBrowserError('desktop_required', '本地 AI 浏览器仅在一龙 Windows 客户端中可用。')
  }
  try {
    return await invoke<T>(command, args)
  } catch (error) {
    throw normalizeDesktopInvokeError(error)
  }
}

export function isLocalAiBrowserUpgradeRequired(error: unknown): boolean {
  return error instanceof LocalAiBrowserError && error.code === 'upgrade_required'
}

export function localAiBrowserErrorMessage(error: unknown): string {
  return error instanceof Error && error.message
    ? error.message
    : 'Win 本地浏览器调用失败，请重启客户端后重试。'
}

function normalizeDesktopInvokeError(error: unknown): LocalAiBrowserError {
  const raw = error instanceof Error
    ? error.message
    : typeof error === 'string'
      ? error
      : ''
  const incompatible = /command.+not found|unknown command|not allowed|allowlist|permission denied/i.test(raw)
  if (incompatible) {
    return new LocalAiBrowserError(
      'upgrade_required',
      '当前 Win 客户端版本不包含 ChatGPT 本地浏览器。请下载新版，完全退出旧客户端后重新打开。',
    )
  }
  return new LocalAiBrowserError('invoke_failed', 'Win 本地浏览器调用失败，请重启客户端后重试。')
}

function assertIdentity(providerId: string, ownerKey: string): void {
  if (!providerId.trim()) throw new Error('缺少 AI 网页厂商标识。')
  if (!ownerKey.trim()) throw new Error('请先登录一龙账号。')
}

function assertProvider(provider: LocalAiWebProvider): void {
  if (!provider?.id
    || !provider.displayName
    || provider.loginMode !== 'manual_web'
    || provider.profileScope !== 'local_owner_provider'
    || provider.rendererProtocol !== UNIFIED_AI_PROTOCOL) {
    throw new Error('桌面壳返回了不受支持的 AI 网页厂商协议。')
  }
}
