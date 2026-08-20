import type { LocalAiMessageSnapshot, LocalAiWebSessionState } from './localAiBrowserApi'

export type LocalAiNewConversationPath = 'adapter' | 'home'

export function selectLocalAiNewConversationPath(
  _providerId: string,
  session: Pick<
    LocalAiWebSessionState,
    'windowStatus' | 'loading' | 'rendererStatus' | 'semanticCacheStatus' | 'contextReady'
  > | null,
  snapshot: Pick<LocalAiMessageSnapshot, 'composerReady'> | null,
): LocalAiNewConversationPath {
  // 健康适配器会先在宿主中登记“新会话”边界，再由官网导航。这个边界是
  // 清除旧回答、保留同一访客 Profile 的关键；直接导航到同一个 /aimode URL
  // 无法区分刷新与新会话，只能作为适配器尚未就绪时的恢复路径。
  if (!session
    || session.windowStatus === 'closed'
    || session.loading
    || session.rendererStatus !== 'active'
    || session.semanticCacheStatus !== 'live'
    || session.contextReady === false
    || !snapshot?.composerReady) {
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
