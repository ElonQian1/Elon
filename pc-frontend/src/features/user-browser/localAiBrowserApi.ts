import { getDesktopInvoke } from '../shell/desktopShell'
import { localAiSnapshotCache } from './localAiSnapshotCache'
import { UNIFIED_AI_PROTOCOL } from './unifiedAiProtocol'
import {
  createLocalAiRequestId,
  isLocalAiRequestId,
} from './localAiCommandReceipt'
import {
  LOCAL_AI_REQUIRED_DESKTOP_RUNTIME_VERSION,
  requiredLocalAiAdapterVersion,
} from './localAiAdapterCompatibility'
import { waitForLocalAiAdapterReceipts } from './localAiAdapterResultWaiter'
import type {
  LocalAiAttachment,
  LocalAiComposerControlsSnapshot,
  LocalAiFeatureNavigationSnapshot,
  LocalAiSessionDiagnostics,
  LocalAiPrivateRichRecovery,
  LocalAiStructuredContentPart,
  LocalAiUiManifestSnapshot,
} from './localAiBrowserProtocol'

export type {
  LocalAiAttachment,
  LocalAiComposerControlsSnapshot,
  LocalAiFeatureNavigationSnapshot,
  LocalAiSessionDiagnostics,
  LocalAiPrivateRichRecovery,
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
  researchCaptureStatus: 'local_raw_prelaunch'
  researchCaptureRetentionDays: number
  desktopRuntimeVersion: number
  adapterVersion: number
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

export interface ClearedLocalAiWebSession {
  providerId: string
  status: 'cleared'
}

export interface LocalAiResearchCaptureStatus {
  captureCount: number
  analyzedCaptureCount: number
  latestAnalyzedAtMs: number
  compatibility:
    | 'not_available'
    | 'truncated'
    | 'analyzer_unavailable'
    | 'parse_error'
    | 'empty_stream'
    | 'upstream_changed'
    | 'renderer_upgrade_required'
    | 'incomplete'
    | 'structure_observed'
    | 'text_compatible'
    | 'rich_compatible'
  decodedFrameCount: number
  acceptedFrameCount: number
  assistantFrameCount: number
  textLength: number
  richKinds: string[]
  contentTypes: string[]
  unsupportedRichCount: number
  completed: boolean
  truncated: boolean
  privateNetworkObservationCount: number
  privateVoiceObservationCount: number
  privateObservationLatestAtMs: number
  privateVoiceChannels: string[]
}

export interface LocalAiGuestOwnerIdentity {
  ownerKey: string
  persistence: 'native_device'
  migratedLegacy: boolean
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
  accessReason?: 'login_required' | 'rate_limited' | ''
  accessSource?: 'visible_page' | 'private_response' | ''
  composerReady: boolean
  streaming: boolean
  streamingStatus?: string
  privateStreamObserved?: boolean
  privateStreamRevision?: number
  privateStreamState?: 'idle' | 'streaming' | 'completed'
  privateTransportHealth?: LocalAiPrivateTransportHealth
  privateRichRecovery?: LocalAiPrivateRichRecovery
  currentModel: string
  attachments?: LocalAiAttachment[]
  dictationActive?: boolean
  capabilities: string[]
}

export interface LocalAiPrivateTransportHealth {
  version: number
  prefetchEnabled: boolean
  prefetchReady: boolean
  officialFresh: boolean
  cooldownRemainingMs: number
  officialLatencyMs: number
  privateLatencyMs: number
  successes: number
  failures: number
  consecutiveFailures: number
  lastOutcome: 'none' | 'success' | 'timeout' | 'auth' | 'context' | 'http' | 'network' | 'parse' | 'empty' | 'official_error'
  attemptBudgetMs: number
  sampledAtMs: number
}

export interface LocalAiConversationDirectoryItem {
  id: string
  title: string
  path: string
  active: boolean
  pinned?: boolean
  groupLabel: string
  projectId?: string | null
  projectTitle?: string
  projectPath?: string | null
  activityDates: string[]
}

export interface LocalAiConversationCollection {
  scrollerFound: boolean
  scrolled: boolean
  scrollRestored: boolean
  reachedEnd: boolean
  truncated: boolean
  timedOut: boolean
  observedCount: number
  availableCount?: number
  steps: number
  complete: boolean
  source?: 'official_partial' | 'official_complete'
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
  collection?: LocalAiConversationCollection
}

export interface LocalAiCommandResult {
  type: 'command_result'
  action: string
  ok: boolean
  detail: string
  requestId?: string | null
}

export interface LocalAiRealtimeVoiceStateEvent {
  type: 'realtime_voice_state'
  version: number
  active: boolean
  observedChannelCount: number
  openChannelCount: number
  observedFrameCount: number
  acceptedEventCount: number
  streamCount: number
  revision: number
}

export interface LocalAiAttachmentTransportEvent {
  type: 'attachment_transport'
  transportVersion: 1
  sequence: number
  state: 'armed' | 'started' | 'completed' | 'failed'
  completedCount: number
}

export function isLocalAiAttachmentTransportEvent(
  value: unknown,
): value is LocalAiAttachmentTransportEvent {
  if (!value || typeof value !== 'object') return false
  const event = value as Partial<LocalAiAttachmentTransportEvent>
  return event.type === 'attachment_transport'
    && event.transportVersion === 1
    && Number.isInteger(event.sequence)
    && Number(event.sequence) > 0
    && ['armed', 'started', 'completed', 'failed'].includes(String(event.state))
    && Number.isInteger(event.completedCount)
    && Number(event.completedCount) >= 0
    && Number(event.completedCount) <= 10
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
  composerEvents?: Record<string, LocalAiComposerControlsSnapshot | Record<string, unknown>>
  featureEvent?: LocalAiFeatureNavigationSnapshot | Record<string, unknown> | null
  interactionLive?: boolean
  interactionUpdatedAtMs?: number
  uiManifestEvent?: LocalAiUiManifestSnapshot | Record<string, unknown> | null
  realtimeVoiceEvent?: LocalAiRealtimeVoiceStateEvent | Record<string, unknown> | null
  attachmentTransportEvent?: LocalAiAttachmentTransportEvent | Record<string, unknown> | null
  commandResult?: LocalAiCommandResult | null
  commandResults?: LocalAiCommandResult[]
  diagnostics?: LocalAiSessionDiagnostics
  cacheStatus: 'empty' | 'cached' | 'live'
  semanticCacheStatus: 'empty' | 'cached' | 'live'
  navigationCacheStatus: 'empty' | 'cached' | 'live'
  localConversations?: LocalAiCachedConversation[]
  activeConversationId?: string | null
  semanticConversationAligned?: boolean
  contextReady?: boolean
  contextStatus?: 'empty' | 'cached' | 'restoring' | 'bound' | 'unbound'
  cacheUpdatedAtMs: number
  navigationUpdatedAtMs: number
  semanticUpdatedAtMs: number
  updatedAtMs: number
}

export interface LocalAiCachedConversation {
  id: string
  title: string
  active: boolean
  updatedAtMs: number
}

export type LocalAiBrowserControlAction =
  | 'restore'
  | 'background'
  | 'reload'
  | 'back'
  | 'home'
  | 'new_conversation_home'
  | 'new_conversation_reload'
  | 'external'

export type LocalAiAdapterAction =
  | 'snapshot'
  | 'refresh_current_conversation'
  | 'set_draft'
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
  | 'prepare_realtime_voice'
  | 'control_managed_realtime_voice'

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

export async function resolveNativeLocalAiGuestOwnerIdentity(
  legacyOwnerKey?: string,
): Promise<LocalAiGuestOwnerIdentity> {
  const identity = await invokeDesktop<LocalAiGuestOwnerIdentity>(
    'resolve_local_ai_guest_owner_identity',
    { legacyOwnerKey },
    LOCAL_AI_INVOKE_TIMEOUTS.capability,
    'guest-owner-identity',
  )
  if (!identity.ownerKey.startsWith('anonymous-device:')
    || identity.persistence !== 'native_device') {
    throw new Error('桌面壳返回了无效的本机游客身份。')
  }
  return identity
}

export async function openLocalAiWebSession(
  providerId: string,
  ownerKey: string,
  options: { showWindow?: boolean } = {},
): Promise<LocalAiWebSession> {
  assertIdentity(providerId, ownerKey)
  const showWindow = options.showWindow === true
  const session = await invokeDesktop<LocalAiWebSession>('open_local_ai_web_session', {
    providerId,
    ownerKey,
    showWindow,
  }, LOCAL_AI_INVOKE_TIMEOUTS.window, localAiOpenCoalesceKey(providerId, ownerKey, showWindow))
  if (session.providerId !== providerId
    || session.profileScope !== 'local_owner_provider'
    || session.cookieAccess !== 'webview_only'
    || session.rendererProtocol !== UNIFIED_AI_PROTOCOL) {
    throw new Error('桌面壳返回了不受支持的本地会话协议。')
  }
  return session
}

function localAiOpenCoalesceKey(providerId: string, ownerKey: string, showWindow: boolean) {
  return `open:${providerId}:${ownerKey}:${showWindow ? 'visible' : 'background'}`
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

export async function openLocalAiWebResearchDirectory(
  providerId: string,
  ownerKey: string,
): Promise<void> {
  assertIdentity(providerId, ownerKey)
  await invokeDesktop<void>('open_local_ai_web_research_directory', {
    providerId,
    ownerKey,
  }, LOCAL_AI_INVOKE_TIMEOUTS.action)
}

export async function getLocalAiWebResearchCaptureStatus(
  providerId: string,
  ownerKey: string,
): Promise<LocalAiResearchCaptureStatus> {
  assertIdentity(providerId, ownerKey)
  return invokeDesktop<LocalAiResearchCaptureStatus>('get_local_ai_web_research_capture_status', {
    providerId,
    ownerKey,
  }, LOCAL_AI_INVOKE_TIMEOUTS.state, `research:${providerId}:${ownerKey}`)
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

export async function openLocalAiCachedConversation(
  providerId: string,
  ownerKey: string,
  conversationId: string,
): Promise<LocalAiWebSessionState> {
  assertIdentity(providerId, ownerKey)
  if (!/^[a-f0-9]{16}$/i.test(conversationId)) throw new Error('本机会话缓存标识无效。')
  const state = await invokeDesktop<LocalAiWebSessionState>('open_local_ai_cached_conversation', {
    providerId,
    ownerKey,
    conversationId,
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

export async function requestLocalAiWebSnapshot(
  providerId: string,
  ownerKey: string,
): Promise<void> {
  assertIdentity(providerId, ownerKey)
  await invokeDesktop<void>('run_local_ai_web_adapter_command', {
    providerId,
    ownerKey,
    action: 'snapshot',
  }, LOCAL_AI_INVOKE_TIMEOUTS.state, `snapshot:${providerId}:${ownerKey}`)
}

export async function requestLocalAiCurrentConversationRefresh(
  providerId: string,
  ownerKey: string,
): Promise<void> {
  assertIdentity(providerId, ownerKey)
  await invokeDesktop<void>('run_local_ai_web_adapter_command', {
    providerId,
    ownerKey,
    action: 'refresh_current_conversation',
  }, LOCAL_AI_INVOKE_TIMEOUTS.action, `conversation-refresh:${providerId}:${ownerKey}`)
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
  return waitForLocalAiAdapterResults(providerId, ownerKey, [{ action, requestId }])
}

export async function waitForLocalAiAdapterResults(
  providerId: string,
  ownerKey: string,
  requests: ReadonlyArray<{ action: string; requestId: string }>,
): Promise<LocalAiWebSessionState | null> {
  if (!requests.length || requests.some(({ requestId }) => !isLocalAiRequestId(requestId))) {
    throw new Error('本地 AI 命令回执标识无效。')
  }
  return waitForLocalAiAdapterReceipts({
    providerId,
    requests,
    readState: () => getLocalAiWebSessionState(providerId, ownerKey),
  })
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
  state.navigationUpdatedAtMs = Number.isFinite(state.navigationUpdatedAtMs)
    ? Math.max(0, state.navigationUpdatedAtMs)
    : 0
  state.semanticUpdatedAtMs = Number.isFinite(state.semanticUpdatedAtMs)
    ? Math.max(0, state.semanticUpdatedAtMs)
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
    || provider.rendererProtocol !== UNIFIED_AI_PROTOCOL
    || provider.researchCaptureStatus !== 'local_raw_prelaunch'
    || !Number.isInteger(provider.researchCaptureRetentionDays)
    || provider.researchCaptureRetentionDays < 1) {
    throw new Error('桌面壳返回了不受支持的 AI 网页厂商协议。')
  }
  if (!Number.isInteger(provider.desktopRuntimeVersion)
      || provider.desktopRuntimeVersion < LOCAL_AI_REQUIRED_DESKTOP_RUNTIME_VERSION) {
    const current = Number.isInteger(provider.desktopRuntimeVersion)
      ? provider.desktopRuntimeVersion
      : 0
    throw new LocalAiBrowserError(
      'upgrade_required',
      `当前 Win 客户端的桌面运行时版本 ${current || '未知'}，新版界面至少需要 ${LOCAL_AI_REQUIRED_DESKTOP_RUNTIME_VERSION}。请更新并完全退出旧客户端后重新打开。`,
    )
  }
  const requiredAdapterVersion = requiredLocalAiAdapterVersion(provider.id)
  if (!Number.isInteger(provider.adapterVersion)
      || provider.adapterVersion < requiredAdapterVersion) {
    const current = Number.isInteger(provider.adapterVersion) ? provider.adapterVersion : 0
    throw new LocalAiBrowserError(
      'upgrade_required',
      `当前 Win 客户端的 ${provider.displayName} 适配器版本 ${current || '未知'}，新版界面至少需要 ${requiredAdapterVersion}。请更新并完全退出旧客户端后重新打开。`,
    )
  }
  const rawActions = Array.isArray(provider.adapterActions) ? provider.adapterActions : []
  const actions = rawActions.filter((action): action is LocalAiAdapterAction => (
    typeof action === 'string' && LOCAL_AI_ADAPTER_ACTIONS.has(action as LocalAiAdapterAction)
  ))
  provider.adapterActions = actions.length ? [...new Set(actions)] : defaultAdapterActions(provider.id)
}

const LOCAL_AI_ADAPTER_ACTIONS = new Set<LocalAiAdapterAction>([
  'snapshot',
  'refresh_current_conversation',
  'set_draft',
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
  'prepare_realtime_voice',
  'control_managed_realtime_voice',
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
        'prepare_realtime_voice',
        'control_managed_realtime_voice',
      ]
    : shared
}
