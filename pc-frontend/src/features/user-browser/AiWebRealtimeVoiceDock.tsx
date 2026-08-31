import { AudioLines, Mic, MicOff, PhoneOff, Radio, ShieldCheck } from 'lucide-react'
import styles from './AiWebRealtimeVoiceDock.module.css'

interface Props {
  visible: boolean
  statusText: string
  managed: boolean
  connected: boolean
  microphoneActive: boolean
  remoteAudio: boolean
  privateTranscript: boolean
  muted: boolean
  canToggleMute: boolean
  canEnd: boolean
  busy: boolean
  hangupConfirming: boolean
  onToggleMute: () => void
  onEnd: () => void
}

export default function AiWebRealtimeVoiceDock(props: Props) {
  if (!props.visible) return null
  return (
    <aside className={styles.dock} data-connected={props.connected} aria-live="polite" aria-label="实时语音控制">
      <span className={styles.orb} aria-hidden="true">
        <AudioLines size={20} />
        <i /><i /><i />
      </span>
      <span className={styles.summary}>
        <b>{props.managed ? 'Win 托管实时语音' : 'ChatGPT 实时语音'}</b>
        <small>{props.statusText}</small>
      </span>
      <span className={styles.evidence} aria-label="语音连接状态">
        <span data-ok={props.microphoneActive}><Mic size={12} />麦克风</span>
        <span data-ok={props.remoteAudio}><Radio size={12} />远端音频</span>
        <span data-ok={props.privateTranscript}><ShieldCheck size={12} />私有转写</span>
      </span>
      <span className={styles.actions}>
        {props.canToggleMute && (
          <button type="button" onClick={props.onToggleMute} disabled={props.busy} aria-label={props.muted ? '取消静音' : '静音'}>
            {props.muted ? <Mic size={17} /> : <MicOff size={17} />}
            <span>{props.muted ? '取消静音' : '静音'}</span>
          </button>
        )}
        {props.canEnd && (
          <button type="button" data-danger onClick={props.onEnd} disabled={props.busy || props.hangupConfirming}>
            <PhoneOff size={17} />
            <span>{props.hangupConfirming ? '确认中' : '挂断'}</span>
          </button>
        )}
      </span>
    </aside>
  )
}
