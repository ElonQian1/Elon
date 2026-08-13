import styles from './AiHomeModeSwitch.module.css'

export type AiHomeMode = 'chat' | 'work'

export default function AiHomeModeSwitch({
  mode,
  onChange,
}: {
  mode: AiHomeMode
  onChange: (mode: AiHomeMode) => void
}) {
  return (
    <div className={styles.modeSwitch} aria-label="一龙 AI 模式">
      <button type="button" data-active={mode === 'chat'} onClick={() => onChange('chat')}>
        <strong>Chat</strong>
        <span>网页 AI</span>
      </button>
      <button type="button" data-active={mode === 'work'} onClick={() => onChange('work')}>
        <strong>工作</strong>
        <span>Codex</span>
      </button>
    </div>
  )
}
