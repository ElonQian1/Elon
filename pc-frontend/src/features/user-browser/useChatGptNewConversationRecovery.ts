import { useEffect, useRef } from 'react'
import {
  controlLocalAiWebSession,
  getLocalAiWebSessionState,
  isLocalAiMessageSnapshot,
  runLocalAiWebAdapterCommand,
  waitForLocalAiAdapterResult,
  type LocalAiWebSessionState,
} from './localAiBrowserApi'
import {
  chatGptNewConversationRecoveryAction,
  chatGptNewConversationResetControlAction,
} from './localAiNewConversation'
import { CHATGPT_NEW_CONVERSATION_RECOVERY_DELAYS_MS } from './localAiWebChatControllerConfig'

interface ChatGptNewConversationRecoveryOptions {
  providerId: string
  ownerKey: string
  startedAtMs: number
  baselineConversationId: string
  suspended: boolean
  onState: (state: LocalAiWebSessionState) => void
  onMessage: (message: string) => void
}

export default function useChatGptNewConversationRecovery({
  providerId,
  ownerKey,
  startedAtMs,
  baselineConversationId,
  suspended,
  onState,
  onMessage,
}: ChatGptNewConversationRecoveryOptions) {
  const callbacks = useRef({ onState, onMessage })
  callbacks.current = { onState, onMessage }

  useEffect(() => {
    if (!startedAtMs || providerId !== 'chatgpt' || !ownerKey || suspended) return
    let active = true
    let recoveryInFlight = false
    const recover = () => {
      if (!active || recoveryInFlight) return
      recoveryInFlight = true
      void getLocalAiWebSessionState(providerId, ownerKey)
        .then(async (initial) => {
          let current = initial
          let snapshot = isLocalAiMessageSnapshot(current.semanticEvent)
            ? current.semanticEvent
            : null
          let recoveryAction = chatGptNewConversationRecoveryAction(
            current,
            snapshot,
            startedAtMs,
            baselineConversationId,
          )
          if (!active || !recoveryAction) return
          if (current.loading || current.rendererStatus !== 'active') return

          // A navigation/reload only restores the official new-chat surface. Guest ChatGPT
          // can restore the previous root conversation from its persistent profile, so retry
          // the semantic new-chat control once the adapter is live before navigating again.
          try {
            const requestId = await runLocalAiWebAdapterCommand(
              providerId,
              ownerKey,
              'new_conversation',
            )
            const retried = await waitForLocalAiAdapterResult(
              providerId,
              ownerKey,
              'new_conversation',
              requestId,
            )
            if (!active) return
            if (retried?.commandResult?.action === 'new_conversation'
              && retried.commandResult.ok) {
              callbacks.current.onState(retried)
              callbacks.current.onMessage(
                'ChatGPT 官网空白页已确认，正在等待本机实时快照绑定；提前输入的消息仍会安全排队。',
              )
              return
            }
            if (retried) {
              current = retried
              snapshot = isLocalAiMessageSnapshot(current.semanticEvent)
                ? current.semanticEvent
                : null
              recoveryAction = chatGptNewConversationRecoveryAction(
                current,
                snapshot,
                startedAtMs,
                baselineConversationId,
              )
              if (!active || !recoveryAction) return
            }
          } catch {
            // A failed semantic retry falls through to the bounded host reset below.
          }

          const next = await controlLocalAiWebSession(
            providerId,
            ownerKey,
            chatGptNewConversationResetControlAction(current.currentUrl),
          )
          if (!active) return
          callbacks.current.onState(next)
          callbacks.current.onMessage(recoveryAction === 'reload'
            ? 'ChatGPT 新会话首页未完成初始化，已在后台自动重载。'
            : 'ChatGPT 仍停留在上一会话，已在后台自动切回新会话首页。')
        })
        .catch(() => {
          // 总超时会安全恢复草稿，瞬时 WebView2 错误不升级为阻断。
        })
        .finally(() => { recoveryInFlight = false })
    }
    const elapsed = Math.max(0, Date.now() - startedAtMs)
    const pendingDelays = CHATGPT_NEW_CONVERSATION_RECOVERY_DELAYS_MS
      .filter((delay) => delay > elapsed)
      .map((delay) => delay - elapsed)
    const delays = elapsed >= CHATGPT_NEW_CONVERSATION_RECOVERY_DELAYS_MS[0]
      ? [0, ...pendingDelays]
      : pendingDelays
    const timers = delays.map((delay) => window.setTimeout(recover, delay))
    return () => {
      active = false
      timers.forEach((timer) => window.clearTimeout(timer))
    }
  }, [baselineConversationId, ownerKey, providerId, startedAtMs, suspended])
}
