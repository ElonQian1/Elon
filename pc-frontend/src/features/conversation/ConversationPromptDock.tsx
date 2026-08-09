import type { KeyboardEvent, ReactNode, RefObject } from 'react'
import { SendHorizontal } from 'lucide-react'
import styles from './ConversationPromptDock.module.css'

interface ConversationPromptDockProps {
  value: string
  placeholder: string
  disabled: boolean
  submitDisabled: boolean
  sending: boolean
  busyLabel?: string
  textareaRef: RefObject<HTMLTextAreaElement>
  leading?: ReactNode
  controls?: ReactNode
  dropActive?: boolean
  onChange: (value: string) => void
  onKeyDown: (event: KeyboardEvent<HTMLTextAreaElement>) => void
  onAutoResize: () => void
}

export default function ConversationPromptDock({
  value,
  placeholder,
  disabled,
  submitDisabled,
  sending,
  busyLabel = '发送中',
  textareaRef,
  leading,
  controls,
  dropActive = false,
  onChange,
  onKeyDown,
  onAutoResize,
}: ConversationPromptDockProps) {
  const sendLabel = sending ? busyLabel : '发送'

  return (
    <div
      className={styles.inputDock}
      data-drop-active={dropActive ? 'true' : 'false'}
      data-has-leading={leading ? 'true' : 'false'}
      data-has-controls={controls ? 'true' : 'false'}
    >
      {leading}
      <textarea
        ref={textareaRef}
        className={styles.textarea}
        value={value}
        onChange={(event) => {
          onChange(event.target.value)
          onAutoResize()
        }}
        onKeyDown={onKeyDown}
        placeholder={placeholder}
        disabled={disabled}
        rows={1}
      />
      {controls && <div className={styles.controls}>{controls}</div>}
      <button
        className={styles.sendBtn}
        type="submit"
        disabled={submitDisabled}
        title={sendLabel}
        aria-label={sendLabel}
      >
        {sending ? <span className={styles.sendingMark}>...</span> : <SendHorizontal size={17} aria-hidden="true" />}
      </button>
    </div>
  )
}
