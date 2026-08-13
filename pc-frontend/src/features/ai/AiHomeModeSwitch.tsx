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
    <div className={styles.modeSwitch} role="tablist" aria-label="一龙 AI 模式">
      <button
        type="button"
        role="tab"
        aria-selected={mode === 'chat'}
        data-active={mode === 'chat'}
        onClick={() => onChange('chat')}
      >
        聊天
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={mode === 'work'}
        data-active={mode === 'work'}
        onClick={() => onChange('work')}
      >
        工作
      </button>
    </div>
  )
}
