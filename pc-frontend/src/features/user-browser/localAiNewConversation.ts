import type { LocalAiMessageSnapshot, LocalAiWebSessionState } from './localAiBrowserApi'

export type LocalAiNewConversationPath = 'adapter' | 'home'
export type ChatGptNewConversationRecoveryAction = 'home' | 'reload' | null

const NEW_CONVERSATION_NATIVE_SETTLE_MS = 750

export function selectLocalAiNewConversationPath(
  _providerId: string,
  session: Pick<
    LocalAiWebSessionState,
    'windowStatus' | 'rendererStatus' | 'semanticCacheStatus' | 'contextReady'
  > | null,
  _snapshot: Pick<LocalAiMessageSnapshot, 'composerReady'> | null,
): LocalAiNewConversationPath {
  // 健康适配器会先在宿主中登记“新会话”边界，再由官网导航。这个边界是
  // 清除旧回答、保留同一访客 Profile 的关键；直接导航到同一个 /aimode URL
  // 无法区分刷新与新会话，只能作为适配器尚未就绪时的恢复路径。
  // 新建会话本身不会写入旧上下文，因此不要求旧会话的语义缓存或上下文绑定
  // 已经完成；新会话动作本身也不依赖输入框。只要同一个后台 WebView 的
  // 适配器仍处于 active，就直接复用，避免因为快照尚未同步而整页重连。
  if (!session
    || session.windowStatus === 'closed'
    || session.rendererStatus !== 'active') {
    return 'home'
  }
  return 'adapter'
}

export function googleNewConversationNeedsReload(
  session: Pick<
    LocalAiWebSessionState,
    'rendererStatus' | 'semanticCacheStatus' | 'contextReady'
  >,
  snapshot: Pick<LocalAiMessageSnapshot, 'composerReady'> | null,
): boolean {
  return session.rendererStatus !== 'active'
    || session.semanticCacheStatus !== 'live'
    || session.contextReady !== true
    || snapshot?.composerReady !== true
}

export function chatGptNewConversationRecoveryAction(
  session: Pick<
    LocalAiWebSessionState,
    | 'currentUrl'
    | 'windowStatus'
    | 'loading'
    | 'rendererStatus'
    | 'semanticCacheStatus'
    | 'contextReady'
    | 'activeConversationId'
    | 'cacheUpdatedAtMs'
    | 'semanticUpdatedAtMs'
    | 'updatedAtMs'
  > | null,
  snapshot: Pick<
    LocalAiMessageSnapshot,
    'messages' | 'composerReady' | 'authenticated' | 'loginRequired'
  > | null,
  startedAtMs: number,
  baselineConversationId: string,
  observedAtMs: number = Date.now(),
): ChatGptNewConversationRecoveryAction {
  if (localAiNewConversationNativeReady(
    session,
    snapshot,
    startedAtMs,
    baselineConversationId,
    observedAtMs,
  )) return null

  try {
    const current = new URL(session?.currentUrl || 'https://chatgpt.com/')
    return current.protocol === 'https:'
      && current.hostname === 'chatgpt.com'
      && current.pathname === '/'
      ? 'reload'
      : 'home'
  } catch {
    return 'home'
  }
}

export function localAiNewConversationContextReady(
  session: Pick<
    LocalAiWebSessionState,
    | 'rendererStatus'
    | 'semanticCacheStatus'
    | 'contextReady'
    | 'activeConversationId'
    | 'cacheUpdatedAtMs'
    | 'semanticUpdatedAtMs'
    | 'updatedAtMs'
  > | null,
  snapshot: Pick<LocalAiMessageSnapshot, 'messages'> | null,
  startedAtMs: number,
  baselineConversationId: string,
): boolean {
  return Boolean(
    startedAtMs
    && session?.rendererStatus === 'active'
    && session.semanticCacheStatus === 'live'
    && session.contextReady === true
    && session.cacheUpdatedAtMs >= startedAtMs
    && session.semanticUpdatedAtMs >= startedAtMs
    && session.updatedAtMs >= startedAtMs
    && session.activeConversationId
    && session.activeConversationId !== baselineConversationId
    && snapshot,
  )
}

export function localAiNewConversationNativeReady(
  session: Pick<
    LocalAiWebSessionState,
    | 'windowStatus'
    | 'loading'
    | 'rendererStatus'
    | 'semanticCacheStatus'
    | 'contextReady'
    | 'activeConversationId'
    | 'cacheUpdatedAtMs'
    | 'semanticUpdatedAtMs'
    | 'updatedAtMs'
  > | null,
  snapshot: Pick<
    LocalAiMessageSnapshot,
    'messages' | 'composerReady' | 'authenticated' | 'loginRequired'
  > | null,
  startedAtMs: number,
  baselineConversationId: string,
  observedAtMs: number = Date.now(),
): boolean {
  return Boolean(
    localAiNewConversationContextReady(
      session,
      snapshot,
      startedAtMs,
      baselineConversationId,
    )
    && !session?.loading
    && !['closed', 'opening', 'loading', 'blocked', 'error'].includes(session?.windowStatus || '')
    && observedAtMs >= (session?.semanticUpdatedAtMs || 0) + NEW_CONVERSATION_NATIVE_SETTLE_MS
    && snapshot?.composerReady
    && (snapshot.authenticated || !snapshot.loginRequired),
  )
}
