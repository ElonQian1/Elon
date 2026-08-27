import { useEffect, useRef, useState } from 'react'
import type { LocalAiMessageSnapshot } from './localAiBrowserApi'
import {
  beginOptimisticLocalAiSend,
  beginPendingLocalAiResponse,
  type PendingLocalAiResponse,
} from './localAiOptimisticSend'
import type { QueuedLocalAiSend } from './localAiWebChatControllerConfig'
import { normalizeLocalAiResponsePrompt } from './localAiResponseTracking'

export default function useLocalAiAccessRecovery(
  identity: string,
  snapshot: LocalAiMessageSnapshot | null,
  pendingResponses: PendingLocalAiResponse[],
  expectedPrompt: string,
  clearPendingResponses: (value: PendingLocalAiResponse[]) => void,
  cancelResponseRefresh: () => void,
  setMessage: (value: string) => void,
) {
  const [prompt, setPrompt] = useState('')
  const handled = useRef(false)
  const loginRequired = Boolean(snapshot?.loginRequired || snapshot?.accessReason === 'login_required')
  const blocked = Boolean(loginRequired || snapshot?.accessReason === 'rate_limited')

  useEffect(() => {
    setPrompt('')
    handled.current = false
  }, [identity])

  useEffect(() => {
    if (!blocked) {
      handled.current = false
      return
    }
    if (handled.current && pendingResponses.length === 0) return
    const retained = pendingResponses[pendingResponses.length - 1]?.prompt || expectedPrompt
    if (normalizeLocalAiResponsePrompt(retained)) setPrompt(retained.trim())
    if (pendingResponses.length) clearPendingResponses([])
    cancelResponseRefresh()
    handled.current = true
    setMessage(loginRequired
      ? '已识别 ChatGPT 官方登录提示；一龙已停止等待空白回复，并保留上一条问题。'
      : '已识别 ChatGPT 官方请求受限；一龙已停止等待空白回复，并保留上一条问题。')
  }, [blocked, cancelResponseRefresh, clearPendingResponses, expectedPrompt, loginRequired, pendingResponses, setMessage])

  return { blocked, prompt, dismiss: () => setPrompt('') }
}

export function createLocalAiAccessRetry(
  sessionIdentity: string,
  draftIdentity: string,
  prompt: string,
  id: string,
) {
  const pending = beginOptimisticLocalAiSend([], [], prompt, id)
  if (!pending) return null
  const queued: QueuedLocalAiSend = {
    prompt: pending.prompt,
    expectedDraft: '',
    pending,
    sessionIdentity,
    draftIdentity,
    queueReason: 'direct',
    queuedAtMs: Date.now(),
  }
  return { pending, response: beginPendingLocalAiResponse(pending), queued }
}
