import { getDesktopInvoke } from '../shell/desktopShell'
import { localAiSnapshotCache } from './localAiSnapshotCache'
import { UNIFIED_AI_PROTOCOL } from './unifiedAiProtocol'
import { createLocalAiRequestId, isLocalAiRequestId } from './localAiCommandReceipt'
import type {
  LocalAiAttachment,
  LocalAiComposerControlsSnapshot,
  LocalAiFeatureNavigationSnapshot,
  LocalAiSessionDiagnostics,
  LocalAiStructuredContentPart,
  LocalAiUiManifestSnapshot,
} from './localAiBrowserProtocol'

export type {
  LocalAiAttachment,
  LocalAiComposerControlsSnapshot,
  LocalAiFeatureNavigationSnapshot,
  LocalAiSessionDiagnostics,
  LocalAiStructuredContentPart,
  LocalAiUiManifestSnapshot,
} from './localAiBrowserProtocol'

export interface LocalAiWebProvider {
  id: string
  displayName: string
  startHost: string
  loginMode: 'manual_web' | 'guest_web_system_login'
  profileScope: 'local_owner_provider'
  rendererProtocol: typeof UNIFIED_AI_PROTOCOL
  rendererStatus: 'reserved' | 'active'
  adapterActions: LocalAiAdapterAction[]
}

export interface LocalAiWebSession {
  providerId: string
  windowLabel: string
  status: 'created' | 'focused' | 'background'
  profileScope: 'local_owner_provider'
  cookieAccess: 'webview_only'
  rendererProtocol: typeof UNIFIED_AI_PROTOCOL
  rendererStatus: 'reserved' | 'active'
}

export interface LocalAiNativeChatWindow {
  providerId: string
  windowLabel: string
  status: 'created' | 'focused'
}

export interface LocalAiNativeWindowState {
  providerId: string
  windowLabel: string
  phase: 'creating' | 'loading' | 'loaded' | 'ready' | 'error' | 'closed'
  focused: boolean
  pageReady: boolean
  rootExists: boolean
  rootChildCount: number
  lastErrorCode?: 'root_empty' | 'page_runtime_error' | 'webview_navigation_error' | 'webview_create_failed' | string | null
  retryable: boolean
  updatedAtMs: number
}

export interface ClearedLocalAiWebSession {
  providerId: string
  status: 'cleared'
}

export type LocalAiBrowserWindowStatus =
  | 'opening'
  | 'loading'
  | 'ready'
  | 'minimized'
  | 'blocked'
  | 'error'
  | 'closed'

export interface LocalAiVisibleMessage {
  id: string
  role: 'user' | 'assistant'
  state: 'streaming' | 'completed'
  content: LocalAiVisibleContentPart[]
}

export type LocalAiVisibleContentPart =
  | { type: 'text'; text: string }
  | { type: 'markdown'; text: string }
  | LocalAiStructuredContentPart

export interface LocalAiMessageSnapshot {
  type: 'message_snapshot'
  title: string
  url: string
  draft: string
  messages: LocalAiVisibleMessage[]
  observedMessageCount?: number
  messageWindowStart?: number
  authenticated: boolean
  pageKind?: 'auth' | 'conversation' | 'home' | 'feature' | 'ai_mode' | 'unsupported' | 'unknown'
  loginRequired?: boolean
  composerReady: boolean
  streaming: boolean
  currentModel: string
  attachments?: LocalAiAttachment[]
  dictationActive?: boolean
  capabilities: string[]
}

export interface LocalAiConversationDirectoryItem {
  id: string
  title: string
  path: string
  active: boolean
  groupLabel: string
  projectId?: string | null
  projectTitle?: string
  projectPath?: string | null
  activityDates: string[]
}

export interface LocalAiProjectDirectoryItem {
  id: string
  title: string
  path: string
  active: boolean
}

export interface LocalAiConversationSnapshot {
  type: 'conversation_snapshot'
  conversations: LocalAiConversationDirectoryItem[]
  projects: LocalAiProjectDirectoryItem[]
}

export interface LocalAiCommandResult {
  type: 'command_result'
  action: string
  ok: boolean
  detail: string
  requestId?: string | null
}

export interface LocalAiWebSessionState {
  providerId: string
  windowLabel: string
  windowStatus: LocalAiBrowserWindowStatus
  windowVisible: boolean
  currentUrl: string
  currentHost: string
  loading: boolean
  rendererStatus: 'connecting' | 'reserved' | 'active'
  lastError?: string | null
  semanticEvent?: LocalAiMessageSnapshot | Record<string, unknown> | null
  navigationEvent?: LocalAiConversationSnapshot | Record<string, unknown> | null
  composerEvent?: LocalAiComposerControlsSnapshot | Record<string, unknown> | null
  featureEvent?: LocalAiFeatureNavigationSnapshot | Record<string, unknown> | null
  uiManifestEvent?: LocalAiUiManifestSnapshot | Record<string, unknown> | null
  commandResult?: LocalAiCommandResult | null
  diagnostics?: LocalAiSessionDiagnostics
  cacheStatus: 'empty' | 'cached' | 'live'
  semanticCacheStatus: 'empty' | 'cached' | 'live'
  navigationCacheStatus: 'empty' | 'cached' | 'live'
  cacheUpdatedAtMs: number
  updatedAtMs: number
}

export type LocalAiBrowserControlAction = 'restore' | 'background' | 'reload' | 'back' | 'home' | 'external'

export type LocalAiAdapterAction =
  | 'snapshot'
  | 'send_prompt'
  | 'stop_generation'
  | 'regenerate_response'
  | 'new_conversation'
  | 'list_conversations'
  | 'open_conversation'
  | 'open_project'
  | 'start_google_login'
  | 'list_model_options'
  | 'list_composer_tools'
  | 'collect_model_options'
  | 'collect_composer_tools'
  | 'select_model_option'
  | 'select_composer_tool'
  | 'request_attachment_upload'
  | 'open_model_selector'
  | 'open_composer_tools'
  | 'start_dictation'
  | 'cancel_dictation'
  | 'submit_dictation'
  | 'remove_attachment'
  | 'dismiss_composer_menu'
  | 'list_navigation'
  | 'collect_navigation'
  | 'select_navigation'
  | 'dismiss_navigation'
  | 'snapshot_ui_manifest'
  | 'invoke_ui_control'

type LocalAiBrowserErrorCode = 'upgrade_required' | 'desktop_required' | 'invoke_failed' | 'invoke_timeout'

const LOCAL_AI_INVOKE_TIMEOUTS = {
  capability: 4_000,
  state: 3_000,
  window: 12_000,
  action: 8_000,
  clear: 15_000,
} as const

const pendingDesktopInvokes = new Map<string, Promise<unknown>>()

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
  const providers = await invokeDesktop<LocalAiWebProvider[]>(
    'list_local_ai_web_providers',
    undefined,
    LOCAL_AI_INVOKE_TIMEOUTS.capability,
  )
  if (!Array.isArray(providers)) throw new Error('桌面壳返回了无效的 AI 网页厂商列表。')
  for (const provider of providers) normalizeProvider(provider)
  return providers
}

export async function openLocalAiWebSession(
  providerId: string,
  ownerKey: string,
  options: { showWindow?: boolean } = {},
): Promise<LocalAiWebSession> {
  assertIdentity(providerId, ownerKey)
  const session = await invokeDesktop<LocalAiWebSession>('open_local_ai_web_session', {
    providerId,
    ownerKey,
    showWindow: options.showWindow,
  }, LOCAL_AI_INVOKE_TIMEOUTS.window)
  if (session.providerId !== providerId
    || session.profileScope !== 'local_owner_provider'
    || session.cookieAccess !== 'webview_only'
    || session.rendererProtocol !== UNIFIED_AI_PROTOCOL) {
    throw new Error('桌面壳返回了不受支持的本地会话协议。')
  }
  return session
}

export async function openLocalAiNativeChatWindow(
  providerId: string,
  ownerKey: string,
): Promise<LocalAiNativeChatWindow> {
  assertIdentity(providerId, ownerKey)
  const window = await invokeDesktop<LocalAiNativeChatWindow>('open_local_ai_native_chat_window', {
    providerId,
    ownerKey,
  }, LOCAL_AI_INVOKE_TIMEOUTS.window)
  if (window.providerId !== providerId || !window.windowLabel.startsWith(`local-ai-native-${providerId}-`)) {
    throw new Error('桌面壳返回了不受支持的一龙聊天窗口。')
  }
  return window
}

export async function getLocalAiNativeWindowState(
  providerId: string,
  ownerKey: string,
): Promise<LocalAiNativeWindowState> {
  assertIdentity(providerId, ownerKey)
  const state = await invokeDesktop<LocalAiNativeWindowState>('get_local_ai_native_window_state', {
    providerId,
    ownerKey,
  }, LOCAL_AI_INVOKE_TIMEOUTS.state, `native-state:${providerId}:${ownerKey}`)
  if (state.providerId !== providerId
    || !state.windowLabel.startsWith(`local-ai-native-${providerId}-`)
    || !['creating', 'loading', 'loaded', 'ready', 'error', 'closed'].includes(state.phase)) {
    throw new Error('桌面壳返回了无效的一龙聊天窗状态。')
  }
  return state
}

export async function clearLocalAiWebSession(
  providerId: string,
  ownerKey: string,
): Promise<ClearedLocalAiWebSession> {
  assertIdentity(providerId, ownerKey)
  const cleared = await invokeDesktop<ClearedLocalAiWebSession>('clear_local_ai_web_session', {
    providerId,
    ownerKey,
  }, LOCAL_AI_INVOKE_TIMEOUTS.clear)
  localAiSnapshotCache.forget(providerId, ownerKey)
  return cleared
}

export function getCachedLocalAiWebSessionState(
  providerId: string,
  ownerKey: string,
): LocalAiWebSessionState | null {
  if (!providerId.trim() || !ownerKey.trim()) return null
  return localAiSnapshotCache.read(providerId, ownerKey)
}

export async function getLocalAiWebSessionState(
  providerId: string,
  ownerKey: string,
): Promise<LocalAiWebSessionState> {
  assertIdentity(providerId, ownerKey)
  const state = await invokeDesktop<LocalAiWebSessionState>('get_local_ai_web_session_state', {
    providerId,
    ownerKey,
  }, LOCAL_AI_INVOKE_TIMEOUTS.state, `state:${providerId}:${ownerKey}`)
  return rememberSessionState(providerId, ownerKey, state)
}

export async function controlLocalAiWebSession(
  providerId: string,
  ownerKey: string,
  action: LocalAiBrowserControlAction,
): Promise<LocalAiWebSessionState> {
  assertIdentity(providerId, ownerKey)
  const state = await invokeDesktop<LocalAiWebSessionState>('control_local_ai_web_session', {
    providerId,
    ownerKey,
    action,
  }, LOCAL_AI_INVOKE_TIMEOUTS.action)
  return rememberSessionState(providerId, ownerKey, state)
}

export async function runLocalAiWebAdapterCommand(
  providerId: string,
  ownerKey: string,
  action: LocalAiAdapterAction,
  value?: string,
  expectedDraft?: string,
): Promise<string> {
  assertIdentity(providerId, ownerKey)
  const requestId = createLocalAiRequestId()
  await invokeDesktop<void>('run_local_ai_web_adapter_command', {
    providerId,
    ownerKey,
    action,
    value,
    expectedDraft,
    requestId,
  }, LOCAL_AI_INVOKE_TIMEOUTS.action)
  return requestId
}

export function isLocalAiMessageSnapshot(value: unknown): value is LocalAiMessageSnapshot {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false
  const snapshot = value as Partial<LocalAiMessageSnapshot>
  return snapshot.type === 'message_snapshot'
    && Array.isArray(snapshot.messages)
    && typeof snapshot.authenticated === 'boolean'
    && typeof snapshot.composerReady === 'boolean'
}

export function isLocalAiConversationSnapshot(value: unknown): value is LocalAiConversationSnapshot {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false
  const snapshot = value as Partial<LocalAiConversationSnapshot>
  return snapshot.type === 'conversation_snapshot'
    && Array.isArray(snapshot.conversations)
    && Array.isArray(snapshot.projects)
}

export async function waitForLocalAiAdapterResult(
  providerId: string,
  ownerKey: string,
  action: string,
  requestId: string,
): Promise<LocalAiWebSessionState | null> {
  if (!isLocalAiRequestId(requestId)) throw new Error('本地 AI 命令回执标识无效。')
  for (let attempt = 0; attempt < 12; attempt += 1) {
    await new Promise((resolve) => window.setTimeout(resolve, 200))
    const state = await getLocalAiWebSessionState(providerId, ownerKey)
    if (state.commandResult?.action === action && state.commandResult.requestId === requestId) return state
  }
  return null
}

async function invokeDesktop<T>(
  command: string,
  args?: Record<string, unknown>,
  timeoutMs: number = LOCAL_AI_INVOKE_TIMEOUTS.action,
  coalesceKey?: string,
): Promise<T> {
  const invoke = getDesktopInvoke()
  if (!invoke) {
    throw new LocalAiBrowserError('desktop_required', '本地 AI 浏览器仅在一龙 Windows 客户端中可用。')
  }
  let timeout = 0
  try {
    let invokePromise = coalesceKey
      ? pendingDesktopInvokes.get(coalesceKey) as Promise<T> | undefined
      : undefined
    if (!invokePromise) {
      let trackedPromise: Promise<T>
      trackedPromise = invoke<T>(command, args).finally(() => {
        if (coalesceKey && pendingDesktopInvokes.get(coalesceKey) === trackedPromise) {
          pendingDesktopInvokes.delete(coalesceKey)
        }
      })
      invokePromise = trackedPromise
      if (coalesceKey) pendingDesktopInvokes.set(coalesceKey, trackedPromise)
    }
    const timeoutPromise = new Promise<never>((_, reject) => {
      timeout = window.setTimeout(() => reject(new LocalAiBrowserError(
        'invoke_timeout',
        'Win 桌面壳响应超时。窗口操作已停止等待；请关闭卡住的官方页或聊天窗后重试。',
      )), timeoutMs)
    })
    return await Promise.race([invokePromise, timeoutPromise])
  } catch (error) {
    if (error instanceof LocalAiBrowserError) throw error
    throw normalizeDesktopInvokeError(error)
  } finally {
    window.clearTimeout(timeout)
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
      '当前 Win 客户端版本不包含新版官方 AI 本地浏览器。请下载新版，完全退出旧客户端后重新打开。',
    )
  }
  const detail = raw.trim().slice(0, 240)
  return new LocalAiBrowserError(
    'invoke_failed',
    detail || 'Win 本地浏览器调用失败，请重启客户端后重试。',
  )
}

function assertIdentity(providerId: string, ownerKey: string): void {
  if (!providerId.trim()) throw new Error('缺少 AI 网页厂商标识。')
  if (!ownerKey.trim()) throw new Error('请先登录一龙账号。')
}

function rememberSessionState(
  providerId: string,
  ownerKey: string,
  state: LocalAiWebSessionState,
): LocalAiWebSessionState {
  if (state.providerId !== providerId) {
    throw new Error('桌面壳返回了错误厂商的本地会话状态。')
  }
  const cacheStatus = normalizeCacheStatus(state.cacheStatus)
  state.cacheStatus = cacheStatus
  state.semanticCacheStatus = normalizeCacheStatus(state.semanticCacheStatus, cacheStatus)
  state.navigationCacheStatus = normalizeCacheStatus(state.navigationCacheStatus, cacheStatus)
  state.cacheUpdatedAtMs = Number.isFinite(state.cacheUpdatedAtMs)
    ? Math.max(0, state.cacheUpdatedAtMs)
    : 0
  localAiSnapshotCache.remember(providerId, ownerKey, state)
  return state
}

function normalizeCacheStatus(
  value: LocalAiWebSessionState['cacheStatus'] | undefined,
  fallback: LocalAiWebSessionState['cacheStatus'] = 'live',
): LocalAiWebSessionState['cacheStatus'] {
  return value === 'empty' || value === 'cached' || value === 'live' ? value : fallback
}

function normalizeProvider(provider: LocalAiWebProvider): void {
  if (!provider?.id
    || !provider.displayName
    || !['manual_web', 'guest_web_system_login'].includes(provider.loginMode)
    || provider.profileScope !== 'local_owner_provider'
    || provider.rendererProtocol !== UNIFIED_AI_PROTOCOL) {
    throw new Error('桌面壳返回了不受支持的 AI 网页厂商协议。')
  }
  const rawActions = Array.isArray(provider.adapterActions) ? provider.adapterActions : []
  const actions = rawActions.filter((action): action is LocalAiAdapterAction => (
    typeof action === 'string' && LOCAL_AI_ADAPTER_ACTIONS.has(action as LocalAiAdapterAction)
  ))
  provider.adapterActions = actions.length ? [...new Set(actions)] : defaultAdapterActions(provider.id)
}

const LOCAL_AI_ADAPTER_ACTIONS = new Set<LocalAiAdapterAction>([
  'snapshot',
  'send_prompt',
  'stop_generation',
  'regenerate_response',
  'new_conversation',
  'list_conversations',
  'open_conversation',
  'open_project',
  'start_google_login',
  'list_model_options',
  'list_composer_tools',
  'collect_model_options',
  'collect_composer_tools',
  'select_model_option',
  'select_composer_tool',
  'request_attachment_upload',
  'open_model_selector',
  'open_composer_tools',
  'start_dictation',
  'cancel_dictation',
  'submit_dictation',
  'remove_attachment',
  'dismiss_composer_menu',
  'list_navigation',
  'collect_navigation',
  'select_navigation',
  'dismiss_navigation',
  'snapshot_ui_manifest',
  'invoke_ui_control',
])

function defaultAdapterActions(providerId: string): LocalAiAdapterAction[] {
  const shared: LocalAiAdapterAction[] = ['snapshot', 'send_prompt', 'stop_generation', 'new_conversation']
  return providerId === 'chatgpt'
    ? [
        ...shared,
        'regenerate_response',
        'list_conversations',
        'open_conversation',
        'open_project',
        'start_google_login',
        'list_model_options',
        'list_composer_tools',
        'collect_model_options',
        'collect_composer_tools',
        'select_model_option',
        'select_composer_tool',
        'request_attachment_upload',
        'open_model_selector',
        'open_composer_tools',
        'start_dictation',
        'cancel_dictation',
        'submit_dictation',
        'remove_attachment',
        'dismiss_composer_menu',
        'list_navigation',
        'collect_navigation',
        'select_navigation',
        'dismiss_navigation',
        'snapshot_ui_manifest',
        'invoke_ui_control',
      ]
    : shared
}
