import type {
  LocalAiAdapterAction,
  LocalAiMessageSnapshot,
  LocalAiWebProvider,
  LocalAiWebSessionState,
} from './localAiBrowserApi'

export type LocalAiClientState = 'desktop_required' | 'checking' | 'ready' | 'upgrade_required' | 'error'

export type LocalAiUserPhase =
  | 'client_checking'
  | 'client_unavailable'
  | 'official_closed'
  | 'official_loading'
  | 'official_blocked'
  | 'official_error'
  | 'adapter_waiting'
  | 'login_required'
  | 'provider_unavailable'
  | 'ready_guest'
  | 'ready_authenticated'
  | 'streaming'

export interface LocalAiUserFeature {
  id: 'native_chat' | 'new_conversation' | 'citations' | 'conversation_history' | 'attachments'
  label: string
  supported: boolean
  active: boolean
}

export interface LocalAiUserState {
  phase: LocalAiUserPhase
  tone: 'ready' | 'loading' | 'attention' | 'error' | 'muted'
  badge: string
  title: string
  detail: string
  officialOpen: boolean
  officialVisible: boolean
  adapterConnected: boolean
  authenticated: boolean
  guestMode: boolean
  degraded: boolean
  fallbackRecommended: boolean
  canSend: boolean
  canStop: boolean
  canNewConversation: boolean
  canConversationHistory: boolean
  canStartGoogleLogin: boolean
  features: LocalAiUserFeature[]
}

export function deriveLocalAiUserState(
  clientState: LocalAiClientState,
  provider: LocalAiWebProvider | undefined,
  session: LocalAiWebSessionState | null,
  snapshot: LocalAiMessageSnapshot | null,
): LocalAiUserState {
  const officialOpen = Boolean(session && session.windowStatus !== 'closed')
  const officialVisible = Boolean(session?.windowVisible)
  const authenticated = Boolean(snapshot?.authenticated)
  const guestMode = Boolean(snapshot?.composerReady && !authenticated && !snapshot?.loginRequired)
  const supportedActions = new Set<LocalAiAdapterAction>(provider?.adapterActions ?? [])
  const liveCapabilities = new Set(snapshot?.capabilities ?? [])
  const accountReady = authenticated || guestMode
  const adapterConnected = Boolean(snapshot && session?.rendererStatus === 'active')
  const livePageReady = Boolean(
    officialOpen
      && !session?.loading
      && !['opening', 'loading', 'blocked', 'error'].includes(session?.windowStatus || '')
      && session?.semanticCacheStatus !== 'cached',
  )
  const canSend = Boolean(
    adapterConnected
      && livePageReady
      && snapshot?.composerReady
      && accountReady
      && supportedActions.has('send_prompt'),
  )
  const canNewConversation = Boolean(
    canSend
      && supportedActions.has('new_conversation')
      && liveCapabilities.has('new_conversation'),
  )
  const canConversationHistory = Boolean(
    livePageReady
      && authenticated
      && supportedActions.has('list_conversations')
      && (liveCapabilities.has('conversation_history') || liveCapabilities.has('conversation_list')),
  )
  const canStartGoogleLogin = Boolean(
    officialOpen
      && !authenticated
      && supportedActions.has('start_google_login'),
  )

  const shared = {
    officialOpen,
    officialVisible,
    adapterConnected,
    authenticated,
    guestMode,
    canSend,
    canStop: Boolean(snapshot?.streaming && supportedActions.has('stop_generation')),
    canNewConversation,
    canConversationHistory,
    canStartGoogleLogin,
    features: featureMatrix(provider, liveCapabilities, {
      canSend,
      canNewConversation,
      canConversationHistory,
    }),
  }

  if (clientState === 'checking') {
    return result('client_checking', 'loading', '检查中', '正在检查本地网页 AI', '正在读取当前 Win 客户端的厂商与原生界面能力。', true, false, shared)
  }
  if (clientState !== 'ready' || !provider) {
    const upgrade = clientState === 'upgrade_required'
    return result(
      'client_unavailable',
      'error',
      upgrade ? '需更新' : '不可用',
      upgrade ? '当前 Win 客户端需要更新' : '本地网页 AI 暂不可用',
      upgrade ? '更新并完全退出旧客户端后重试；官方网页仍可在系统浏览器使用。' : '请在 Windows 客户端重新检查，或暂时使用官方网页。',
      true,
      true,
      shared,
    )
  }
  if (!officialOpen) {
    return result('official_closed', 'muted', '未打开', `尚未打开 ${provider.displayName}`, loginInstruction(provider), true, false, shared)
  }
  if (session?.windowStatus === 'blocked') {
    return result('official_blocked', 'error', '已拦截', '官方页导航已被安全拦截', session.lastError || '页面离开了允许的官方域名；请回到厂商主页或使用系统浏览器。', true, true, shared)
  }
  if (session?.windowStatus === 'error' || session?.lastError) {
    return result('official_error', 'error', '异常', '官方页面需要处理', session?.lastError || '请显示官方窗口确认错误，必要时刷新或使用系统浏览器。', true, true, shared)
  }
  if (session?.loading || ['opening', 'loading'].includes(session?.windowStatus || '')) {
    return result('official_loading', 'loading', '加载中', `正在连接 ${provider.displayName}`, '官方页面正在加载；登录、地区和真人验证仍由厂商页面决定。', true, false, shared)
  }
  if (!adapterConnected) {
    return result('adapter_waiting', 'attention', '等待同步', '官方页已打开，原生界面仍在连接', '可以显示官方窗口继续操作；一龙只等待页面可见语义，不读取网络请求或 Cookie。', true, true, shared)
  }
  if (snapshot?.streaming) {
    return result('streaming', 'ready', '回答中', `${provider.displayName} 正在回答`, '回答和公开来源正在同步到一龙聊天界面。', false, false, shared)
  }
  if (snapshot?.loginRequired) {
    return result('login_required', 'attention', '需登录', `请在 ${provider.displayName} 官方窗口登录`, '登录、真人验证和账号选择必须由本人在官方窗口完成；完成后本页会自动更新。', true, false, shared)
  }
  if (!snapshot?.composerReady) {
    if (provider.loginMode !== 'guest_web_system_login' && !authenticated) {
      return result('login_required', 'attention', '需登录', `请在 ${provider.displayName} 官方窗口登录`, '当前官方页没有访客输入框；登录、真人验证和账号选择必须由本人完成。', true, false, shared)
    }
    const googleDetail = '当前页面没有可用输入框；Google AI 模式可能尚未对当前地区、语言或账号开放。'
    return result(
      'provider_unavailable',
      'attention',
      '只读降级',
      `${provider.displayName} 官方页可见，但原生输入暂不可用`,
      guestMode ? googleDetail : '官方页面结构或账号状态暂不支持原生输入；可显示官方窗口继续使用。',
      true,
      true,
      shared,
    )
  }
  if (guestMode) {
    return result('ready_guest', 'ready', '访客可用', `${provider.displayName} 已接入一龙界面`, guestDetail(provider), false, false, shared)
  }
  return result('ready_authenticated', 'ready', '已登录', `${provider.displayName} 已接入一龙界面`, '官方账号已在本机窗口就绪，可以使用当前支持的原生聊天能力。', false, false, shared)
}

function result(
  phase: LocalAiUserPhase,
  tone: LocalAiUserState['tone'],
  badge: string,
  title: string,
  detail: string,
  degraded: boolean,
  fallbackRecommended: boolean,
  shared: Omit<LocalAiUserState, 'phase' | 'tone' | 'badge' | 'title' | 'detail' | 'degraded' | 'fallbackRecommended'>,
): LocalAiUserState {
  return { phase, tone, badge, title, detail, degraded, fallbackRecommended, ...shared }
}

function loginInstruction(provider: LocalAiWebProvider): string {
  return provider.loginMode === 'guest_web_system_login'
    ? '先打开 Google 官方页确认 AI 模式在当前地区与账号可用；登录按官方要求在系统浏览器完成。'
    : '先连接 ChatGPT 官方页；官网提供访客输入框时可直接聊天，需要历史或增强能力时再由本人登录。'
}

function guestDetail(provider: LocalAiWebProvider): string {
  return provider.loginMode === 'guest_web_system_login'
    ? '当前使用官方访客能力；系统浏览器登录不会把 Cookie 复制到本地窗口。'
    : '当前使用 ChatGPT 官方访客能力；需要历史、项目或增强能力时再在官方窗口登录。'
}

function featureMatrix(
  provider: LocalAiWebProvider | undefined,
  live: Set<string>,
  active: Pick<LocalAiUserState, 'canSend' | 'canNewConversation' | 'canConversationHistory'>,
): LocalAiUserFeature[] {
  const actions = new Set<LocalAiAdapterAction>(provider?.adapterActions ?? [])
  const features: LocalAiUserFeature[] = [
    { id: 'native_chat', label: '原生收发', supported: actions.has('send_prompt'), active: active.canSend },
    { id: 'new_conversation', label: '新建对话', supported: actions.has('new_conversation'), active: active.canNewConversation },
    { id: 'conversation_history', label: '历史与项目', supported: actions.has('list_conversations'), active: active.canConversationHistory },
    { id: 'citations', label: '公开来源', supported: provider?.id === 'google-ai-mode' || live.has('citations'), active: live.has('citations') },
    { id: 'attachments', label: '附件', supported: actions.has('list_composer_tools'), active: live.has('attachments') },
  ]
  return features.filter((feature) => feature.supported)
}
