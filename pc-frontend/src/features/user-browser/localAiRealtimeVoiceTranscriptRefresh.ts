export const LOCAL_AI_REALTIME_VOICE_TRANSCRIPT_REFRESH_GAPS_MS = [
  250,
  750,
  1_500,
] as const

export type LocalAiRealtimeVoiceTranscriptRefreshAction = 'private_conversation' | 'snapshot'

export type LocalAiRealtimeVoiceTranscriptRefreshClaim =
  | { status: 'run'; action: LocalAiRealtimeVoiceTranscriptRefreshAction }
  | { status: 'busy' | 'stale' | 'done' }

export type LocalAiRealtimeVoiceTranscriptRefreshSettlement =
  | { status: 'wait'; delayMs: number }
  | { status: 'done' | 'stale' }

/**
 * Serializes the bounded transcript settlement used after the official realtime
 * voice surface closes. Duplicate close signals join the active sequence and an
 * old request can never schedule work for a newer WebView session.
 */
export class LocalAiRealtimeVoiceTranscriptRefreshFlight {
  private generation = 0
  private active = false
  private inFlight = false
  private step = 0

  start(): { generation: number; started: boolean } {
    if (this.active) return { generation: this.generation, started: false }
    this.generation += 1
    this.active = true
    this.inFlight = false
    this.step = 0
    return { generation: this.generation, started: true }
  }

  cancel(): number {
    this.generation += 1
    this.active = false
    this.inFlight = false
    this.step = 0
    return this.generation
  }

  claim(generation: number): LocalAiRealtimeVoiceTranscriptRefreshClaim {
    if (generation !== this.generation) return { status: 'stale' }
    if (!this.active || this.step >= LOCAL_AI_REALTIME_VOICE_TRANSCRIPT_REFRESH_GAPS_MS.length) {
      return { status: 'done' }
    }
    if (this.inFlight) return { status: 'busy' }
    this.inFlight = true
    return {
      status: 'run',
      action: this.step === 0 ? 'private_conversation' : 'snapshot',
    }
  }

  settle(generation: number): LocalAiRealtimeVoiceTranscriptRefreshSettlement {
    if (generation !== this.generation || !this.active || !this.inFlight) {
      return { status: 'stale' }
    }
    this.inFlight = false
    this.step += 1
    if (this.step >= LOCAL_AI_REALTIME_VOICE_TRANSCRIPT_REFRESH_GAPS_MS.length) {
      this.active = false
      return { status: 'done' }
    }
    return {
      status: 'wait',
      delayMs: LOCAL_AI_REALTIME_VOICE_TRANSCRIPT_REFRESH_GAPS_MS[this.step],
    }
  }
}
