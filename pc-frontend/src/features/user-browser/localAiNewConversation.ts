import type { LocalAiMessageSnapshot, LocalAiWebSessionState } from './localAiBrowserApi'

export type LocalAiNewConversationPath = 'adapter' | 'home'

export function selectLocalAiNewConversationPath(
  session: Pick<
    LocalAiWebSessionState,
    'windowStatus' | 'loading' | 'rendererStatus' | 'semanticCacheStatus' | 'contextReady'
  > | null,
  snapshot: Pick<LocalAiMessageSnapshot, 'composerReady'> | null,
): LocalAiNewConversationPath {
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
