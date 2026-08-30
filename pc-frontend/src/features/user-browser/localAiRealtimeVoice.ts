import type { LocalAiRealtimeVoiceStateEvent } from './localAiBrowserApi'
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
    return { observed: false, active: false }
  }
  const openChannelCount = Number(value.openChannelCount)
  return {
    observed: Number.isInteger(openChannelCount) && openChannelCount >= 0,
    active: value.active === true && openChannelCount > 0,
  }
}
