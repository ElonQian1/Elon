import { useCallback, useEffect, useRef, useState } from 'react'
import {
  controlLocalAiWebSession,
  requestLocalAiCurrentConversationRefresh,
  requestLocalAiWebSnapshot,
  runLocalAiWebAdapterCommand,
} from './localAiBrowserApi'
import {
  findLocalAiRealtimeVoiceControls,
  readLocalAiRealtimeVoicePrivateState,
  type LocalAiRealtimeVoiceAction,
} from './localAiRealtimeVoice'
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
import {
  LOCAL_AI_REALTIME_VOICE_TRANSCRIPT_REFRESH_GAPS_MS,
  LocalAiRealtimeVoiceTranscriptRefreshFlight,
} from './localAiRealtimeVoiceTranscriptRefresh'
import { requestReturnToAiChat } from './internalBrowserApi'
import type { AiWebChatBackend } from './useAiWebChatBackend'

export default function useLocalAiRealtimeVoiceControl(web: AiWebChatBackend) {
  const transcriptRefresh = useRef(new LocalAiRealtimeVoiceTranscriptRefreshFlight())
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
  const privateStateRef = useRef(readLocalAiRealtimeVoicePrivateState(
    web.controller.sessionState?.realtimeVoiceEvent,
  ))
  const [activationStatus, setActivationStatus] = useState<'idle' | 'confirming' | 'active' | 'unconfirmed'>('idle')
  const [transcriptSyncing, setTranscriptSyncing] = useState(false)
  const [hangupStatus, setHangupStatus] = useState<'idle' | 'confirming' | 'unconfirmed'>('idle')
  const sessionIdentity = web.controller.sessionIdentity
  manifestRef.current = web.controller.uiManifest
  privateStateRef.current = readLocalAiRealtimeVoicePrivateState(
    web.controller.sessionState?.realtimeVoiceEvent,
  )

  useEffect(() => {
    transcriptRefresh.current.cancel()
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
      transcriptRefresh.current.cancel()
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
      privateDataChannelActive: privateStateRef.current.active,
      managedActive: privateStateRef.current.managedActive,
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
      privateDataChannelActive: privateStateRef.current.active,
      managedActive: privateStateRef.current.managedActive,
    })) {
      setActivationStatus('active')
    } else if (activationStatus === 'active'
      && manifest?.compatibility === 'healthy'
      && manifest.controlsTruncated !== true
      && Boolean(voice.start)) {
      setActivationStatus('idle')
    }
  }, [
    activationStatus,
    evaluateActivation,
    web.controller.sessionState?.realtimeVoiceEvent,
    web.controller.uiManifest,
  ])

  const startTranscriptRefresh = useCallback(() => {
    const request = web.officialRequest
    if (request?.providerId !== 'chatgpt') return
    const start = transcriptRefresh.current.start()
    if (!start.started) return
    window.clearTimeout(timer.current)
    setTranscriptSyncing(true)
    requestReturnToAiChat(request)
    void controlLocalAiWebSession(request.providerId, request.ownerKey, 'background')
      .then(() => {}, () => {})
    const refresh = (): void => {
      const claim = transcriptRefresh.current.claim(start.generation)
      if (claim.status !== 'run') return
      const operation = claim.action === 'private_conversation'
        ? requestLocalAiCurrentConversationRefresh(request.providerId, request.ownerKey)
        : requestLocalAiWebSnapshot(request.providerId, request.ownerKey)
      const settle = () => {
        const next = transcriptRefresh.current.settle(start.generation)
        if (next.status === 'wait') {
          timer.current = window.setTimeout(refresh, next.delayMs)
        } else if (next.status === 'done') {
          setTranscriptSyncing(false)
        }
      }
      void operation.then(settle, settle)
    }
    timer.current = window.setTimeout(
      refresh,
      LOCAL_AI_REALTIME_VOICE_TRANSCRIPT_REFRESH_GAPS_MS[0],
    )
  }, [web.officialRequest])

  useEffect(() => {
    const manifest = web.controller.uiManifest
    const voice = findLocalAiRealtimeVoiceControls(manifest?.controls ?? [])
    const current = localAiRealtimeVoiceActivationConfirmed({
      manifestHealthy: manifest?.compatibility === 'healthy',
      controlsTruncated: manifest?.controlsTruncated === true,
      voiceActive: voice.active,
      privateDataChannelActive: privateStateRef.current.active,
      managedActive: privateStateRef.current.managedActive,
    })
    const endedOnOfficialSurface = officialVoiceActive.current
      && !current
      && manifest?.compatibility === 'healthy'
      && manifest.controlsTruncated !== true
      && Boolean(voice.start)
    officialVoiceActive.current = current
    if (endedOnOfficialSurface && hangupStatus !== 'confirming') startTranscriptRefresh()
  }, [
    hangupStatus,
    startTranscriptRefresh,
    web.controller.sessionState?.realtimeVoiceEvent,
    web.controller.uiManifest,
  ])

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
        privateDataChannelActive: privateStateRef.current.active,
        managedActive: privateStateRef.current.managedActive,
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
  }, [
    evaluateHangup,
    hangupStatus,
    web.controller.sessionState?.realtimeVoiceEvent,
    web.controller.uiManifest,
  ])

  const startHangupConfirmation = useCallback(() => {
    transcriptRefresh.current.cancel()
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
      if (web.controller.draft.trim()) return null
      transcriptRefresh.current.cancel()
      activationGeneration.current += 1
      hangupGeneration.current += 1
      window.clearTimeout(timer.current)
      window.clearTimeout(activationTimer.current)
      window.clearTimeout(hangupTimer.current)
      setActivationStatus('confirming')
      setTranscriptSyncing(false)
      setHangupStatus('idle')
      const officialDraft = web.controller.snapshot?.draft ?? ''
      if (officialDraft) {
        const prepared = await web.controller.run('set_draft', '', officialDraft)
        if (prepared?.commandResult?.action !== 'set_draft' || !prepared.commandResult.ok) {
          setActivationStatus('unconfirmed')
          return prepared
        }
      }
      // Win first prepares an independent WebView2 media peer. The command only
      // arms the same-origin in-memory relay; the following official click still
      // owns upstream session creation and remains the automatic fallback.
      await web.controller.run('prepare_realtime_voice')
    }
    const next = await web.controller.run('invoke_ui_control', controlId)
    const result = next?.commandResult
    if (action === 'start') {
      if (result?.action === 'invoke_ui_control' && result.ok) {
        startActivationConfirmation()
      } else {
        await web.controller.run('control_managed_realtime_voice', 'end')
        setActivationStatus('unconfirmed')
      }
    } else {
      // Mirroring is a safe no-op when the managed peer was unavailable, so the
      // existing official mute/end controls continue to work on every fallback.
      await web.controller.run('control_managed_realtime_voice', action)
    }
    if (action === 'end' && result?.action === 'invoke_ui_control' && result.ok) {
      startHangupConfirmation()
    }
    return next
  }, [startActivationConfirmation, startHangupConfirmation, web.controller])

  return {
    run,
    activationStatus,
    transcriptSyncing,
    hangupStatus,
    privateDataChannelActive: privateStateRef.current.active,
    managedVoiceActive: privateStateRef.current.managedActive,
    managedVoicePhase: privateStateRef.current.managedPhase,
    managedMicrophoneActive: privateStateRef.current.microphoneActive,
    managedRemoteAudio: privateStateRef.current.remoteAudio,
    managedMuted: privateStateRef.current.muted,
    managedFallbackCode: privateStateRef.current.fallbackCode,
  }
}
