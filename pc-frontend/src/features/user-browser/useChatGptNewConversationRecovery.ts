import { useEffect, useRef } from 'react'
import {
  controlLocalAiWebSession,
  getLocalAiWebSessionState,
  isLocalAiMessageSnapshot,
  type LocalAiWebSessionState,
} from './localAiBrowserApi'
import { chatGptNewConversationRecoveryAction } from './localAiNewConversation'
import { CHATGPT_NEW_CONVERSATION_RECOVERY_DELAYS_MS } from './localAiWebChatControllerConfig'

interface ChatGptNewConversationRecoveryOptions {
  providerId: string
  ownerKey: string
  startedAtMs: number
  baselineConversationId: string
  onState: (state: LocalAiWebSessionState) => void
  onMessage: (message: string) => void
}

export default function useChatGptNewConversationRecovery({
  providerId,
  ownerKey,
  startedAtMs,
  baselineConversationId,
  onState,
  onMessage,
}: ChatGptNewConversationRecoveryOptions) {
  const callbacks = useRef({ onState, onMessage })
  callbacks.current = { onState, onMessage }

  useEffect(() => {
    if (!startedAtMs || providerId !== 'chatgpt' || !ownerKey) return
    let active = true
    let recoveryInFlight = false
    const recover = () => {
      if (!active || recoveryInFlight) return
      recoveryInFlight = true
      void getLocalAiWebSessionState(providerId, ownerKey)
        .then(async (current) => {
          const snapshot = isLocalAiMessageSnapshot(current.semanticEvent)
            ? current.semanticEvent
            : null
          const recoveryAction = chatGptNewConversationRecoveryAction(
            current,
            snapshot,
            startedAtMs,
            baselineConversationId,
          )
          if (!active || !recoveryAction) return
          const next = await controlLocalAiWebSession(
            providerId,
            ownerKey,
            'new_conversation_home',
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
    const timers = CHATGPT_NEW_CONVERSATION_RECOVERY_DELAYS_MS.map((delay) => (
      window.setTimeout(recover, delay)
    ))
    return () => {
      active = false
      timers.forEach((timer) => window.clearTimeout(timer))
    }
  }, [baselineConversationId, ownerKey, providerId, startedAtMs])
}
