export const LOCAL_AI_REALTIME_VOICE_ACTIVATION_WATCHDOG_DELAYS_MS = [
  500, 1_000, 2_000, 4_000, 8_000, 12_000,
] as const

export interface LocalAiRealtimeVoiceActivationEvidence {
  manifestHealthy: boolean
  controlsTruncated: boolean
  voiceActive: boolean
  privateDataChannelActive: boolean
}

export function localAiRealtimeVoiceActivationConfirmed(
  evidence: LocalAiRealtimeVoiceActivationEvidence,
) {
  return evidence.privateDataChannelActive || (evidence.manifestHealthy
    && !evidence.controlsTruncated
    && evidence.voiceActive)
}

export function shouldRefreshLocalAiRealtimeVoiceActivationControls(checkIndex: number) {
  return checkIndex === 0 || checkIndex === 1 || checkIndex === 3 || checkIndex === 5
}
