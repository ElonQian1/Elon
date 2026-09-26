import { getDesktopInvoke } from '../shell/desktopShell'
import { normalizeExchangeWebviewError } from './exchangeWebviewErrors.js'

const PROVIDER_SCHEMA = 'yilong.exchange_webview.provider.v1'
const SESSION_SCHEMA = 'yilong.exchange_webview.session.v1'
const OBSERVATION_SCHEMA = 'yilong.exchange_webview.observation.v1'

export const REQUIRED_EXCHANGE_WEBVIEW_RUNTIME_VERSION = 12
/** Desktop generation that ships the read-only observer bridge and its two commands. */
export const EXCHANGE_OBSERVATION_RUNTIME_VERSION = 14
export const EXCHANGE_ADAPTER_ACTIONS = ['refresh', 'detail', 'report', 'wallet', 'inspect'] as const
export type ExchangeAdapterAction = typeof EXCHANGE_ADAPTER_ACTIONS[number]

export interface ExchangeWebObservation {
  schema: typeof OBSERVATION_SCHEMA
  windowOpen: boolean
  adapterReady: boolean
  documentToken: string | null
  identity: Record<string, unknown> | null
  list: Record<string, unknown> | null
  details: Record<string, Record<string, unknown>>
  reports: Record<string, Record<string, unknown>>
  wallet: Record<string, unknown> | null
  diagnostic: Record<string, unknown> | null
  unavailableAtMs: number
  commandResults: Record<string, unknown>[]
  lastError: string | null
  updatedAtMs: number
  tradingEnabled: false
}

export interface ExchangeWebProvider {
  schema: typeof PROVIDER_SCHEMA
  providerId: string
  displayName: string
  startHost: string
  loginMode: 'manual_web'
  profileScope: 'local_owner_provider'
  desktopRuntimeVersion: number
  backgroundOpenSupported?: boolean
}

export interface ExchangeWebSession {
  schema: typeof SESSION_SCHEMA
  providerId: string
  windowLabel: string
  status: 'created' | 'focused' | 'background'
  profileScope: 'local_owner_provider'
  cookieAccess: 'webview_only'
}

const openFlights = new Map<string, Promise<ExchangeWebSession>>()

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
  showWindow = true,
): Promise<ExchangeWebSession> {
  if (!validId(providerId) || !ownerKey.trim()) throw new Error('交易所官网会话身份无效。')
  const flightKey = JSON.stringify([providerId, ownerKey, showWindow])
  const existing = openFlights.get(flightKey)
  if (existing) return existing
  const invoke = requireDesktopInvoke()
  const flight = (async () => {
    if (!showWindow) {
      const providers = await listExchangeWebProviders()
      if (!providers.some(p => p.providerId === providerId && p.backgroundOpenSupported === true)) {
        throw new Error('当前 Win 宿主尚不支持后台打开币安，请更新并重启后读取。')
      }
    }
    return invoke<ExchangeWebSession>('open_exchange_web_session', { providerId, ownerKey, showWindow })
  })()
    .then((session) => {
      if (session?.schema !== SESSION_SCHEMA
        || session.providerId !== providerId
        || session.profileScope !== 'local_owner_provider'
        || session.cookieAccess !== 'webview_only'
        || !(showWindow ? ['created', 'focused'] : ['background']).includes(session.status)) {
        throw new Error('Win 客户端返回了不受支持的交易所官网会话。')
      }
      return session
    })
    .catch((error) => {
      throw normalizeExchangeWebviewError(error)
    })
    .finally(() => { openFlights.delete(flightKey) })
  openFlights.set(flightKey, flight)
  return flight
}

export async function getExchangeWebObservation(
  providerId: string,
  ownerKey: string,
): Promise<ExchangeWebObservation> {
  if (!validId(providerId) || !ownerKey.trim()) throw new Error('交易所官网会话身份无效。')
  const invoke = requireDesktopInvoke()
  let value: ExchangeWebObservation
  try {
    value = await invoke<ExchangeWebObservation>('get_exchange_web_observation', { providerId, ownerKey })
  } catch (error) {
    throw normalizeExchangeWebviewError(error)
  }
  if (value?.schema !== OBSERVATION_SCHEMA
    || typeof value.windowOpen !== 'boolean'
    || typeof value.adapterReady !== 'boolean'
    || value.tradingEnabled !== false
    || !Number.isSafeInteger(value.updatedAtMs)
    || !Array.isArray(value.commandResults)) {
    throw new Error('Win 客户端返回了不受支持的交易所观察合同。')
  }
  return value
}

export async function runExchangeWebAdapterCommand(
  providerId: string,
  ownerKey: string,
  action: ExchangeAdapterAction,
  value?: string,
): Promise<void> {
  if (!validId(providerId) || !ownerKey.trim()) throw new Error('交易所官网会话身份无效。')
  if (!EXCHANGE_ADAPTER_ACTIONS.includes(action)) throw new Error('不支持的交易所只读动作。')
  const invoke = requireDesktopInvoke()
  try {
    await invoke('run_exchange_web_adapter_command', { providerId, ownerKey, action, value: value ?? null, requestId: null })
  } catch (error) {
    throw normalizeExchangeWebviewError(error)
  }
}

function requireDesktopInvoke() {
  const invoke = getDesktopInvoke()
  if (!invoke) throw new Error('交易所官网 WebView 仅在一龙 Windows 客户端中可用。')
  return invoke
}

function validId(value: string): boolean {
  return /^[a-z0-9][a-z0-9-]{0,63}$/.test(value)
}
