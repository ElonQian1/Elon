import { useCallback, useEffect, useRef, useState } from 'react'
import {
  controlLocalAiWebSession,
  requestLocalAiCurrentConversationRefresh,
  requestLocalAiWebSnapshot,
  runLocalAiWebAdapterCommand,
} from './localAiBrowserApi'
import { findLocalAiRealtimeVoiceControls, type LocalAiRealtimeVoiceAction } from './localAiRealtimeVoice'
import {
  LOCAL_AI_REALTIME_VOICE_ACTIVATION_WATCHDOG_DELAYS_MS,
  localAiRealtimeVoiceActivationConfirmed,
  shouldRefreshLocalAiRealtimeVoiceActivationControls,
} from './localAiRealtimeVoiceActivation'
import {
  LOCAL_AI_REALTIME_VOICE_HANGUP_WATCHDOG_DELAYS_MS,
  beginLocalAiRealtimeVoiceHangupObservation,
  observeLocalAiRealtimeVoiceHangup,
  shouldRefreshLocalAiRealtimeVoiceHangupControls,
} from './localAiRealtimeVoiceHangup'
import { requestReturnToAiChat } from './internalBrowserApi'
import type { AiWebChatBackend } from './useAiWebChatBackend'

// Match the APK close settlement: let the official voice overlay close first,
// then perform one private current-conversation refresh and two bounded DOM fallbacks.
const REALTIME_VOICE_TRANSCRIPT_REFRESH_GAPS_MS = [250, 750, 1_500] as const

export default function useLocalAiRealtimeVoiceControl(web: AiWebChatBackend) {
  const generation = useRef(0)
  const timer = useRef(0)
  const activationGeneration = useRef(0)
  const activationTimer = useRef(0)
  const activationWatchdogIndex = useRef(0)
  const officialVoiceActive = useRef(false)
  const hangupGeneration = useRef(0)
  const hangupTimer = useRef(0)
  const hangupWatchdogIndex = useRef(0)
  const hangupObservation = useRef(beginLocalAiRealtimeVoiceHangupObservation())
  const manifestRef = useRef(web.controller.uiManifest)
  const [activationStatus, setActivationStatus] = useState<'idle' | 'confirming' | 'active' | 'unconfirmed'>('idle')
  const [transcriptSyncing, setTranscriptSyncing] = useState(false)
  const [hangupStatus, setHangupStatus] = useState<'idle' | 'confirming' | 'unconfirmed'>('idle')
  const sessionIdentity = web.controller.sessionIdentity
  manifestRef.current = web.controller.uiManifest

  useEffect(() => {
    generation.current += 1
    window.clearTimeout(timer.current)
    activationGeneration.current += 1
    window.clearTimeout(activationTimer.current)
    hangupGeneration.current += 1
    window.clearTimeout(hangupTimer.current)
    timer.current = 0
    activationTimer.current = 0
    officialVoiceActive.current = false
    hangupTimer.current = 0
    setActivationStatus('idle')
    setTranscriptSyncing(false)
    setHangupStatus('idle')
    return () => {
      generation.current += 1
      activationGeneration.current += 1
      hangupGeneration.current += 1
      window.clearTimeout(timer.current)
      window.clearTimeout(activationTimer.current)
      window.clearTimeout(hangupTimer.current)
    }
  }, [sessionIdentity])

  const evaluateActivation = useCallback((expectedGeneration: number) => {
    if (expectedGeneration !== activationGeneration.current) return false
    const manifest = manifestRef.current
    const voice = findLocalAiRealtimeVoiceControls(manifest?.controls ?? [])
    if (!localAiRealtimeVoiceActivationConfirmed({
      manifestHealthy: manifest?.compatibility === 'healthy',
      controlsTruncated: manifest?.controlsTruncated === true,
      voiceActive: voice.active,
    })) return false
    activationGeneration.current += 1
    window.clearTimeout(activationTimer.current)
    setActivationStatus('active')
    return true
  }, [])

  const startActivationConfirmation = useCallback(() => {
    const expectedGeneration = ++activationGeneration.current
    window.clearTimeout(activationTimer.current)
    activationWatchdogIndex.current = 0
    setActivationStatus('confirming')
    if (evaluateActivation(expectedGeneration)) return
    const scheduleNext = (): void => {
      const checkIndex = activationWatchdogIndex.current
      const delay = LOCAL_AI_REALTIME_VOICE_ACTIVATION_WATCHDOG_DELAYS_MS[checkIndex]
      if (delay === undefined) {
        setActivationStatus('unconfirmed')
        return
      }
      activationTimer.current = window.setTimeout(() => {
        if (expectedGeneration !== activationGeneration.current) return
        if (evaluateActivation(expectedGeneration)) return
        const request = web.officialRequest
        if (request && shouldRefreshLocalAiRealtimeVoiceActivationControls(checkIndex)) {
          void runLocalAiWebAdapterCommand(
            request.providerId, request.ownerKey, 'snapshot_ui_manifest',
          ).then(() => {}, () => {})
        }
        activationWatchdogIndex.current += 1
        scheduleNext()
      }, delay)
    }
    scheduleNext()
  }, [evaluateActivation, web.officialRequest])

  useEffect(() => {
    const manifest = web.controller.uiManifest
    const voice = findLocalAiRealtimeVoiceControls(manifest?.controls ?? [])
    if (activationStatus === 'confirming') {
      evaluateActivation(activationGeneration.current)
    } else if (localAiRealtimeVoiceActivationConfirmed({
      manifestHealthy: manifest?.compatibility === 'healthy',
      controlsTruncated: manifest?.controlsTruncated === true,
      voiceActive: voice.active,
    })) {
      setActivationStatus('active')
    } else if (activationStatus === 'active'
      && manifest?.compatibility === 'healthy'
      && manifest.controlsTruncated !== true
      && Boolean(voice.start)) {
      setActivationStatus('idle')
    }
  }, [activationStatus, evaluateActivation, web.controller.uiManifest])

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

  useEffect(() => {
    const manifest = web.controller.uiManifest
    const voice = findLocalAiRealtimeVoiceControls(manifest?.controls ?? [])
    const current = localAiRealtimeVoiceActivationConfirmed({
      manifestHealthy: manifest?.compatibility === 'healthy',
      controlsTruncated: manifest?.controlsTruncated === true,
      voiceActive: voice.active,
    })
    const endedOnOfficialSurface = officialVoiceActive.current
      && !current
      && manifest?.compatibility === 'healthy'
      && manifest.controlsTruncated !== true
      && Boolean(voice.start)
    officialVoiceActive.current = current
    if (endedOnOfficialSurface && hangupStatus !== 'confirming') startTranscriptRefresh()
  }, [hangupStatus, startTranscriptRefresh, web.controller.uiManifest])

  const evaluateHangup = useCallback((expectedGeneration: number) => {
    if (expectedGeneration !== hangupGeneration.current) return false
    const manifest = manifestRef.current
    const voice = findLocalAiRealtimeVoiceControls(manifest?.controls ?? [])
    const result = observeLocalAiRealtimeVoiceHangup(
      hangupObservation.current,
      {
        conversationPage: manifest?.pageKind === 'conversation',
        manifestHealthy: manifest?.compatibility === 'healthy',
        controlsTruncated: manifest?.controlsTruncated === true,
        startAvailable: Boolean(voice.start),
        voiceActive: voice.active,
      },
      Date.now(),
    )
    hangupObservation.current = result.observation
    if (!result.confirmed) return false
    hangupGeneration.current += 1
    window.clearTimeout(hangupTimer.current)
    setHangupStatus('idle')
    setActivationStatus('idle')
    startTranscriptRefresh()
    return true
  }, [startTranscriptRefresh])

  const scheduleHangupWatchdog = useCallback((expectedGeneration: number) => {
    const scheduleNext = (): void => {
      const checkIndex = hangupWatchdogIndex.current
      const delay = LOCAL_AI_REALTIME_VOICE_HANGUP_WATCHDOG_DELAYS_MS[checkIndex]
      if (delay === undefined) {
        setHangupStatus('unconfirmed')
        return
      }
      hangupTimer.current = window.setTimeout(() => {
        if (expectedGeneration !== hangupGeneration.current) return
        if (evaluateHangup(expectedGeneration)) return
        hangupWatchdogIndex.current += 1
        if (hangupWatchdogIndex.current >= LOCAL_AI_REALTIME_VOICE_HANGUP_WATCHDOG_DELAYS_MS.length) {
          setHangupStatus('unconfirmed')
          return
        }
        const request = web.officialRequest
        if (request && shouldRefreshLocalAiRealtimeVoiceHangupControls(checkIndex)) {
          void runLocalAiWebAdapterCommand(
            request.providerId, request.ownerKey, 'snapshot_ui_manifest',
          ).then(() => {}, () => {})
        }
        scheduleNext()
      }, delay)
    }
    scheduleNext()
  }, [evaluateHangup, web.officialRequest])

  useEffect(() => {
    if (hangupStatus === 'confirming') evaluateHangup(hangupGeneration.current)
  }, [evaluateHangup, hangupStatus, web.controller.uiManifest])

  const startHangupConfirmation = useCallback(() => {
    generation.current += 1
    window.clearTimeout(timer.current)
    setTranscriptSyncing(false)
    const expectedGeneration = ++hangupGeneration.current
    window.clearTimeout(hangupTimer.current)
    hangupWatchdogIndex.current = 0
    hangupObservation.current = beginLocalAiRealtimeVoiceHangupObservation()
    setHangupStatus('confirming')
    scheduleHangupWatchdog(expectedGeneration)
  }, [scheduleHangupWatchdog])

  const run = useCallback(async (
    action: LocalAiRealtimeVoiceAction,
    controlId: string,
  ) => {
    if (!controlId.trim()) return null
    if (action === 'start') {
      generation.current += 1
      activationGeneration.current += 1
      hangupGeneration.current += 1
      window.clearTimeout(timer.current)
      window.clearTimeout(activationTimer.current)
      window.clearTimeout(hangupTimer.current)
      setActivationStatus('confirming')
      setTranscriptSyncing(false)
      setHangupStatus('idle')
    }
    const next = await web.controller.run('invoke_ui_control', controlId)
    const result = next?.commandResult
    if (action === 'start') {
      if (result?.action === 'invoke_ui_control' && result.ok) startActivationConfirmation()
      else setActivationStatus('unconfirmed')
    }
    if (action === 'end' && result?.action === 'invoke_ui_control' && result.ok) {
      startHangupConfirmation()
    }
    return next
  }, [startActivationConfirmation, startHangupConfirmation, web.controller])

  return { run, activationStatus, transcriptSyncing, hangupStatus }
}
