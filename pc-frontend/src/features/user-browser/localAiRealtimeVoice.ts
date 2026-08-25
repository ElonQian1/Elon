import type { LocalAiUiControl } from './localAiBrowserProtocol'

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
