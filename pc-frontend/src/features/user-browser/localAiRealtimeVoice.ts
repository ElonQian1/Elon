import type { LocalAiRealtimeVoiceStateEvent } from './localAiRealtimeVoiceEvent'
import type { LocalAiUiControl } from './localAiBrowserProtocol'

export type LocalAiRealtimeVoiceAction = 'start' | 'mute' | 'unmute' | 'end'

export interface LocalAiRealtimeVoiceControls {
  start?: LocalAiUiControl
  mute?: LocalAiUiControl
  unmute?: LocalAiUiControl
  end?: LocalAiUiControl
  active: boolean
}

const VOICE_LABEL = /voice|call|语音|通话|挂断/i

export function findLocalAiRealtimeVoiceControls(
  controls: readonly LocalAiUiControl[],
): LocalAiRealtimeVoiceControls {
  const start = controls.find((control) => control.semantic === 'voice_mode')
  const mute = controls.find((control) => control.semantic === 'voice_mute')
  const unmute = controls.find((control) => control.semantic === 'voice_unmute')
  const active = Boolean(mute || unmute)
  const end = controls.find((control) => (
    control.semantic === 'close'
      && (VOICE_LABEL.test(control.label) || (active && control.region === 'overlay'))
  ))
  return { start, mute, unmute, end, active: active || Boolean(end) }
}

export function readLocalAiRealtimeVoicePrivateState(
  value: LocalAiRealtimeVoiceStateEvent | Record<string, unknown> | null | undefined,
) {
  if (!value || value.type !== 'realtime_voice_state' || value.version !== 1) {
    if (!value || value.type !== 'realtime_voice_state' || value.version !== 2) {
      return emptyPrivateState()
    }
  }
  const openChannelCount = Number(value.openChannelCount)
  const managedPhase = typeof value.managedPhase === 'string' && MANAGED_PHASES.has(value.managedPhase)
    ? value.managedPhase : 'idle'
  return {
    observed: Number.isInteger(openChannelCount) && openChannelCount >= 0,
    active: openChannelCount > 0,
    managedObserved: value.version >= 2,
    managedPhase,
    managedActive: value.managedActive === true && managedPhase === 'active',
    microphoneActive: value.microphoneActive === true,
    remoteAudio: value.remoteAudio === true,
    muted: value.muted === true,
    routeBound: value.routeBound === true,
    fallbackCode: typeof value.fallbackCode === 'string' ? value.fallbackCode : '',
  }
}

const MANAGED_PHASES = new Set([
  'idle', 'requesting_microphone', 'creating_offer', 'armed', 'applying_answer',
  'connecting', 'active', 'failed', 'closed',
])

function emptyPrivateState() {
  return {
    observed: false,
    active: false,
    managedObserved: false,
    managedPhase: 'idle',
    managedActive: false,
    microphoneActive: false,
    remoteAudio: false,
    muted: false,
    routeBound: false,
    fallbackCode: '',
  }
}
