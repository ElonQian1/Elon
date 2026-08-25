import { useCallback, useEffect, useRef, useState } from 'react'
import {
  controlLocalAiWebSession,
  requestLocalAiCurrentConversationRefresh,
  requestLocalAiWebSnapshot,
} from './localAiBrowserApi'
import type { LocalAiRealtimeVoiceAction } from './localAiRealtimeVoice'
import { requestReturnToAiChat } from './internalBrowserApi'
import type { AiWebChatBackend } from './useAiWebChatBackend'

// Match the APK close settlement: let the official voice overlay close first,
// then perform one private current-conversation refresh and two bounded DOM fallbacks.
const REALTIME_VOICE_TRANSCRIPT_REFRESH_GAPS_MS = [250, 750, 1_500] as const

export default function useLocalAiRealtimeVoiceControl(web: AiWebChatBackend) {
  const generation = useRef(0)
  const timer = useRef(0)
  const [transcriptSyncing, setTranscriptSyncing] = useState(false)
  const sessionIdentity = web.controller.sessionIdentity

  useEffect(() => {
    generation.current += 1
    window.clearTimeout(timer.current)
    timer.current = 0
    setTranscriptSyncing(false)
    return () => {
      generation.current += 1
      window.clearTimeout(timer.current)
    }
  }, [sessionIdentity])

  const startTranscriptRefresh = useCallback(() => {
    const request = web.officialRequest
    if (request?.providerId !== 'chatgpt') return
    const activeGeneration = ++generation.current
    let step = 0
    setTranscriptSyncing(true)
    const refresh = (): void => {
      if (activeGeneration !== generation.current) return
      requestReturnToAiChat(request)
      void controlLocalAiWebSession(request.providerId, request.ownerKey, 'background')
        .then(() => {}, () => {})
      const operation = step === 0
        ? requestLocalAiCurrentConversationRefresh(request.providerId, request.ownerKey)
        : requestLocalAiWebSnapshot(request.providerId, request.ownerKey)
      step += 1
      const delay = REALTIME_VOICE_TRANSCRIPT_REFRESH_GAPS_MS[step]
      if (delay !== undefined) {
        void operation.then(() => {}, () => {})
        timer.current = window.setTimeout(refresh, delay)
      } else {
        const settle = () => {
          if (activeGeneration === generation.current) setTranscriptSyncing(false)
        }
        void operation.then(settle, settle)
      }
    }
    timer.current = window.setTimeout(refresh, REALTIME_VOICE_TRANSCRIPT_REFRESH_GAPS_MS[0])
  }, [web.officialRequest])

  const run = useCallback(async (
    action: LocalAiRealtimeVoiceAction,
    controlId: string,
  ) => {
    if (!controlId.trim()) return null
    if (action === 'start') {
      generation.current += 1
      window.clearTimeout(timer.current)
      setTranscriptSyncing(false)
    }
    const next = await web.controller.run('invoke_ui_control', controlId)
    const result = next?.commandResult
    if (action === 'end' && result?.action === 'invoke_ui_control' && result.ok) {
      startTranscriptRefresh()
    }
    return next
  }, [startTranscriptRefresh, web.controller])

  return { run, transcriptSyncing }
}
