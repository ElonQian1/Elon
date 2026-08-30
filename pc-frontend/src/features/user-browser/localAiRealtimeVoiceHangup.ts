export const LOCAL_AI_REALTIME_VOICE_HANGUP_WATCHDOG_DELAYS_MS = [
  1_000, 1_000, 2_000, 3_000, 5_000, 8_000, 15_000, 25_000, 30_000, 30_000,
] as const

export interface LocalAiRealtimeVoiceHangupEvidence {
  conversationPage: boolean
  manifestHealthy: boolean
  controlsTruncated: boolean
  startAvailable: boolean
  voiceActive: boolean
  privateDataChannelActive: boolean
}

export interface LocalAiRealtimeVoiceHangupObservation {
  stableSinceMs: number
  stableObservations: number
}

export function beginLocalAiRealtimeVoiceHangupObservation(): LocalAiRealtimeVoiceHangupObservation {
  return { stableSinceMs: 0, stableObservations: 0 }
}

export function observeLocalAiRealtimeVoiceHangup(
  previous: LocalAiRealtimeVoiceHangupObservation,
  evidence: LocalAiRealtimeVoiceHangupEvidence,
  observedAtMs: number,
) {
  const ended = evidence.conversationPage
    && evidence.manifestHealthy
    && !evidence.controlsTruncated
    && evidence.startAvailable
    && !evidence.voiceActive
    && !evidence.privateDataChannelActive
  if (!ended) {
    return {
      observation: beginLocalAiRealtimeVoiceHangupObservation(),
      confirmed: false,
    }
  }
  const stableSinceMs = previous.stableSinceMs || observedAtMs
  const stableObservations = previous.stableObservations + 1
  return {
    observation: { stableSinceMs, stableObservations },
    confirmed: stableObservations >= 2 && observedAtMs - stableSinceMs >= 2_000,
  }
}

export function shouldRefreshLocalAiRealtimeVoiceHangupControls(checkIndex: number) {
  return checkIndex === 0 || (checkIndex + 1) % 2 === 0
}
