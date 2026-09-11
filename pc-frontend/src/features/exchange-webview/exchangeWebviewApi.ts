import { getDesktopInvoke } from '../shell/desktopShell'
import { normalizeExchangeWebviewError } from './exchangeWebviewErrors.js'

const PROVIDER_SCHEMA = 'yilong.exchange_webview.provider.v1'
const SESSION_SCHEMA = 'yilong.exchange_webview.session.v1'

export const REQUIRED_EXCHANGE_WEBVIEW_RUNTIME_VERSION = 12

export interface ExchangeWebProvider {
  schema: typeof PROVIDER_SCHEMA
  providerId: string
  displayName: string
  startHost: string
  loginMode: 'manual_web'
  profileScope: 'local_owner_provider'
  desktopRuntimeVersion: number
}

export interface ExchangeWebSession {
  schema: typeof SESSION_SCHEMA
  providerId: string
  windowLabel: string
  status: 'created' | 'focused'
  profileScope: 'local_owner_provider'
  cookieAccess: 'webview_only'
}

let openFlight: Promise<ExchangeWebSession> | null = null

export function isExchangeWebviewAvailable(): boolean {
  return getDesktopInvoke() !== null
}

export async function listExchangeWebProviders(): Promise<ExchangeWebProvider[]> {
  const invoke = requireDesktopInvoke()
  let providers: ExchangeWebProvider[]
  try {
    providers = await invoke<ExchangeWebProvider[]>('list_exchange_web_providers')
  } catch (error) {
    throw normalizeExchangeWebviewError(error)
  }
  if (!Array.isArray(providers)) throw new Error('Win 客户端返回了无效的交易所官网列表。')
  const seen = new Set<string>()
  for (const provider of providers) {
    if (provider?.schema !== PROVIDER_SCHEMA
      || !validId(provider.providerId)
      || seen.has(provider.providerId)
      || !provider.displayName?.trim()
      || !provider.startHost?.trim()
      || provider.loginMode !== 'manual_web'
      || provider.profileScope !== 'local_owner_provider') {
      throw new Error('Win 客户端返回了不受支持的交易所官网合同。')
    }
    seen.add(provider.providerId)
    if (!Number.isInteger(provider.desktopRuntimeVersion)
      || provider.desktopRuntimeVersion < REQUIRED_EXCHANGE_WEBVIEW_RUNTIME_VERSION) {
      throw new Error('当前 Win 客户端版本较旧，请更新并完全退出旧客户端后重新打开。')
    }
  }
  return providers
}

export async function openExchangeWebSession(
  providerId: string,
  ownerKey: string,
): Promise<ExchangeWebSession> {
  if (!validId(providerId) || !ownerKey.trim()) throw new Error('交易所官网会话身份无效。')
  if (openFlight) return openFlight
  const invoke = requireDesktopInvoke()
  openFlight = invoke<ExchangeWebSession>('open_exchange_web_session', { providerId, ownerKey })
    .then((session) => {
      if (session?.schema !== SESSION_SCHEMA
        || session.providerId !== providerId
        || session.profileScope !== 'local_owner_provider'
        || session.cookieAccess !== 'webview_only'
        || !['created', 'focused'].includes(session.status)) {
        throw new Error('Win 客户端返回了不受支持的交易所官网会话。')
      }
      return session
    })
    .catch((error) => {
      throw normalizeExchangeWebviewError(error)
    })
    .finally(() => { openFlight = null })
  return openFlight
}

function requireDesktopInvoke() {
  const invoke = getDesktopInvoke()
  if (!invoke) throw new Error('交易所官网 WebView 仅在一龙 Windows 客户端中可用。')
  return invoke
}

function validId(value: string): boolean {
  return /^[a-z0-9][a-z0-9-]{0,63}$/.test(value)
}
