export interface LocalAiRealtimeVoiceStateEvent {
  type: 'realtime_voice_state'
  version: number
  active: boolean
  observedChannelCount: number
  openChannelCount: number
  observedFrameCount: number
  acceptedEventCount: number
  streamCount: number
  revision: number
  managedPhase?: 'idle' | 'requesting_microphone' | 'creating_offer' | 'armed'
    | 'applying_answer' | 'connecting' | 'active' | 'failed' | 'closed'
  managedActive?: boolean
  microphoneActive?: boolean
  remoteAudio?: boolean
  muted?: boolean
  routeBound?: boolean
  fallbackCode?: string
  lifecycleRevision?: number
}
