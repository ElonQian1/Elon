import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  LocalAiConversationOpenQueue,
  type LocalAiDeferredConversationAction,
} from './localAiConversationOpenQueue'

interface Options {
  sessionIdentity: string
  busyAction: string
  dispatch: (action: LocalAiDeferredConversationAction, value: string) => Promise<unknown>
  onMessage: (message: string) => void
}

export function isLocalAiDeferredConversationAction(
  action: string,
): action is LocalAiDeferredConversationAction {
  return action === 'open_conversation' || action === 'open_project'
}

/** Serializes official WebView navigation while retaining the user's latest selection. */
export default function useLocalAiDeferredConversationNavigation({
  sessionIdentity,
  busyAction,
  dispatch,
  onMessage,
}: Options) {
  const queue = useMemo(
    () => new LocalAiConversationOpenQueue(sessionIdentity),
    [sessionIdentity],
  )
  const dispatchRef = useRef(dispatch)
  dispatchRef.current = dispatch
  const [revision, setRevision] = useState(0)

  const defer = useCallback((action: LocalAiDeferredConversationAction, value = '') => {
    const target = value.trim()
    if (!target) return false
    queue.enqueue(target, action)
    setRevision((current) => current + 1)
    onMessage('已记录最新会话目标；当前网页动作结束后会自动切换，不会停留在旧会话。')
    return true
  }, [onMessage, queue])

  useEffect(() => {
    if (busyAction || !sessionIdentity || !queue.hasPending()) return
    const request = queue.take()
    if (!request || request.sessionIdentity !== sessionIdentity || !request.action) {
      queue.finish()
      return
    }
    void dispatchRef.current(request.action, request.conversationId).finally(() => {
      if (!queue.hasPending()) queue.finish()
      setRevision((current) => current + 1)
    })
  }, [busyAction, queue, revision, sessionIdentity])

  return defer
}
