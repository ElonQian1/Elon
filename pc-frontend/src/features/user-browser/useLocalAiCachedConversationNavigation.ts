import { useCallback, useMemo, useState } from 'react'
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
  const [selection, setSelection] = useState({ sessionIdentity: '', target: '' })
  const queue = useMemo(
    () => new LocalAiConversationOpenQueue(sessionIdentity),
    [sessionIdentity],
  )

  const open = useCallback(async (conversationId: string) => {
    if (!provider || !ownerKey) return
    if (busyAction && busyAction !== 'open_cached_conversation') return
    setSelection({ sessionIdentity, target: conversationId })
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
        onMessage('正在读取会话；已有记录会先从本机显示。')
        try {
          const next = await openLocalAiCachedConversation(
            provider.id,
            ownerKey,
            request.conversationId,
          )
          if (isSessionCurrent(request.sessionIdentity) && !queue.hasPending()) onState(next)
          if (isSessionCurrent(request.sessionIdentity) && !queue.hasPending()) {
            onMessage(next.semanticEvent ? '正在后台更新会话。' : '正在读取会话正文。')
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
  return { open, readTarget: selection.sessionIdentity === sessionIdentity ? selection.target : '',
    clearReadTarget: () => setSelection({ sessionIdentity: '', target: '' }) }
}
