import { useCallback, useMemo } from 'react'
import {
  localAiBrowserErrorMessage,
  openLocalAiCachedConversation,
  type LocalAiWebProvider,
  type LocalAiWebSessionState,
} from './localAiBrowserApi'
import { LocalAiConversationOpenQueue } from './localAiConversationOpenQueue'

interface Options {
  provider: LocalAiWebProvider | undefined
  ownerKey: string
  sessionIdentity: string
  busyAction: string
  beforeOpen: () => void
  isSessionCurrent: (sessionIdentity: string) => boolean
  onBusyAction: (action: string) => void
  onMessage: (message: string) => void
  onState: (state: LocalAiWebSessionState | null) => void
}

export default function useLocalAiCachedConversationNavigation({
  provider,
  ownerKey,
  sessionIdentity,
  busyAction,
  beforeOpen,
  isSessionCurrent,
  onBusyAction,
  onMessage,
  onState,
}: Options) {
  const queue = useMemo(
    () => new LocalAiConversationOpenQueue(sessionIdentity),
    [sessionIdentity],
  )

  return useCallback(async (conversationId: string) => {
    if (!provider || !ownerKey) return
    if (busyAction && busyAction !== 'open_cached_conversation') return
    const startsDrain = queue.enqueue(conversationId)
    if (!startsDrain) {
      onMessage('已更新为最近选择的会话；当前导航完成后会立即切换，不会串入旧会话。')
      return
    }

    beforeOpen()
    onBusyAction('open_cached_conversation')
    try {
      let request = queue.take()
      while (request) {
        if (!isSessionCurrent(request.sessionIdentity)) break
        onMessage('正在从本机缓存恢复会话，并在后台连接官方上下文…')
        try {
          const next = await openLocalAiCachedConversation(
            provider.id,
            ownerKey,
            request.conversationId,
          )
          if (isSessionCurrent(request.sessionIdentity)) onState(next)
          if (isSessionCurrent(request.sessionIdentity) && !queue.hasPending()) {
            onMessage('已立即恢复本机会话缓存；官方页面正在后台同步最新内容。')
          }
        } catch (error) {
          if (isSessionCurrent(request.sessionIdentity) && !queue.hasPending()) {
            onMessage(localAiBrowserErrorMessage(error))
          }
        }
        request = queue.take()
      }
    } finally {
      queue.finish()
      if (isSessionCurrent(sessionIdentity)) onBusyAction('')
    }
  }, [beforeOpen, busyAction, isSessionCurrent, onBusyAction, onMessage, onState, ownerKey,
    provider, queue, sessionIdentity])
}
