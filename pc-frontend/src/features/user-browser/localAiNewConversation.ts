import type { LocalAiMessageSnapshot, LocalAiWebSessionState } from './localAiBrowserApi'

export type LocalAiNewConversationPath = 'adapter' | 'home'

export function selectLocalAiNewConversationPath(
  providerId: string,
  session: Pick<
    LocalAiWebSessionState,
    'windowStatus' | 'loading' | 'rendererStatus' | 'semanticCacheStatus' | 'contextReady'
  > | null,
  snapshot: Pick<LocalAiMessageSnapshot, 'composerReady'> | null,
): LocalAiNewConversationPath {
  // Google 的“新话题”会立即导航并重载适配器，成功回执会随旧文档一起销毁。
  // 固定回到官方 AI Mode 首页更快、更可靠，也保留同一个访客 Profile。
  if (providerId === 'google-ai-mode'
    || !session
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
