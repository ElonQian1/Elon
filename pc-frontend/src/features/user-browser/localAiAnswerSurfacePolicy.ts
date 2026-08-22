import type { LocalAiMessageSnapshot, LocalAiWebSessionState } from './localAiBrowserApi'
import type { AiBrowserSurface } from './internalBrowserApi'

export type LocalAiAnswerRenderMode = 'official_live' | 'native_cache' | 'native'

export function selectLocalAiAnswerRenderMode(input: {
  ready: boolean
  browserSurface: AiBrowserSurface
  busy: boolean
  responseStreaming?: boolean
  session: LocalAiWebSessionState | null
  snapshot: LocalAiMessageSnapshot | null
}): LocalAiAnswerRenderMode {
  const { ready, browserSurface, busy, responseStreaming, session, snapshot } = input
  if (session?.semanticCacheStatus === 'cached') return 'native_cache'
  if (!ready || browserSurface !== 'chat' || busy || !session || !snapshot) return 'native'
  if (session.semanticCacheStatus !== 'live'
    || session.loading
    || ['opening', 'loading', 'blocked', 'error', 'closed'].includes(session.windowStatus)
    || snapshot.streaming
    || responseStreaming) return 'native'
  const completedAssistant = snapshot.messages.some(
    (message) => message.role === 'assistant' && message.state === 'completed',
  )
  return completedAssistant ? 'official_live' : 'native'
}

export function localAiAnswerSurfaceKey(
  providerId: string | undefined,
  snapshot: LocalAiMessageSnapshot | null,
) {
  const assistant = [...(snapshot?.messages ?? [])]
    .reverse()
    .find((message) => message.role === 'assistant' && message.state === 'completed')
  return assistant ? `${providerId || 'ai'}:${assistant.id}` : ''
}
