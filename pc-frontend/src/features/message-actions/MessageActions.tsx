import { useEffect, useId, useMemo, useState } from 'react'
import { createPortal } from 'react-dom'
import { Check, Copy, Maximize2, ThumbsDown, ThumbsUp, X } from 'lucide-react'
import styles from './MessageActions.module.css'

export type MessageFeedbackValue = 'up' | 'down' | null

interface MessageActionsProps {
  content: string
  messageKey: string
  storageScope: string
  align?: 'left' | 'right'
  onFeedbackChange?: (value: MessageFeedbackValue) => void
}

const FEEDBACK_PREFIX = 'elon.pc.messageFeedback'

export default function MessageActions({
  content,
  messageKey,
  storageScope,
  align = 'left',
  onFeedbackChange,
}: MessageActionsProps) {
  const text = content.trim()
  const [copied, setCopied] = useState(false)
  const [expanded, setExpanded] = useState(false)
  const [feedback, setFeedback] = useState<MessageFeedbackValue>(null)
  const dialogTitleId = useId()

  const storageKey = useMemo(() => {
    const scope = normalizeStorageSegment(storageScope)
    const key = normalizeStorageSegment(messageKey)
    return `${FEEDBACK_PREFIX}.${scope}.${key}`
  }, [messageKey, storageScope])

  useEffect(() => {
    if (typeof window === 'undefined') return
    const stored = window.localStorage.getItem(storageKey)
    setFeedback(stored === 'up' || stored === 'down' ? stored : null)
  }, [storageKey])

  useEffect(() => {
    if (!copied) return
    const timer = window.setTimeout(() => setCopied(false), 1200)
    return () => window.clearTimeout(timer)
  }, [copied])

  if (!text) return null

  async function handleCopy() {
    const ok = await copyText(text)
    setCopied(ok)
  }

  function handleFeedback(next: Exclude<MessageFeedbackValue, null>) {
    const value = feedback === next ? null : next
    setFeedback(value)
    if (typeof window !== 'undefined') {
      if (value) window.localStorage.setItem(storageKey, value)
      else window.localStorage.removeItem(storageKey)
    }
    onFeedbackChange?.(value)
  }

  const containerClassName = [
    styles.actions,
    align === 'right' ? styles.right : styles.left,
  ].join(' ')

  return (
    <>
      <div className={containerClassName} role="group" aria-label="消息操作">
        <button
          className={[styles.button, copied ? styles.copied : ''].filter(Boolean).join(' ')}
          type="button"
          title={copied ? '已复制' : '复制'}
          aria-label={copied ? '已复制' : '复制消息'}
          onClick={handleCopy}
        >
          {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
        </button>
        <button
          className={[styles.button, feedback === 'up' ? styles.activePositive : ''].filter(Boolean).join(' ')}
          type="button"
          title="赞"
          aria-label="赞"
          aria-pressed={feedback === 'up'}
          onClick={() => handleFeedback('up')}
        >
          <ThumbsUp aria-hidden="true" />
        </button>
        <button
          className={[styles.button, feedback === 'down' ? styles.activeNegative : ''].filter(Boolean).join(' ')}
          type="button"
          title="踩"
          aria-label="踩"
          aria-pressed={feedback === 'down'}
          onClick={() => handleFeedback('down')}
        >
          <ThumbsDown aria-hidden="true" />
        </button>
        <button
          className={styles.button}
          type="button"
          title="展开"
          aria-label="展开消息"
          onClick={() => setExpanded(true)}
        >
          <Maximize2 aria-hidden="true" />
        </button>
      </div>
      {expanded && typeof document !== 'undefined' && createPortal(
        <div className={styles.backdrop} role="presentation" onClick={() => setExpanded(false)}>
          <section
            className={styles.dialog}
            role="dialog"
            aria-modal="true"
            aria-labelledby={dialogTitleId}
            onClick={(event) => event.stopPropagation()}
          >
            <header className={styles.dialogHeader}>
              <h2 className={styles.dialogTitle} id={dialogTitleId}>完整消息</h2>
              <button
                className={styles.button}
                type="button"
                title="关闭"
                aria-label="关闭完整消息"
                onClick={() => setExpanded(false)}
              >
                <X aria-hidden="true" />
              </button>
            </header>
            <pre className={styles.dialogContent}>{text}</pre>
          </section>
        </div>,
        document.body,
      )}
    </>
  )
}

async function copyText(text: string): Promise<boolean> {
  if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text)
      return true
    } catch {
      // Fall back to a temporary textarea when clipboard permissions are denied.
    }
  }

  if (typeof document === 'undefined') return false

  const textarea = document.createElement('textarea')
  textarea.value = text
  textarea.setAttribute('readonly', '')
  textarea.style.position = 'fixed'
  textarea.style.left = '-9999px'
  document.body.appendChild(textarea)
  textarea.select()

  try {
    return document.execCommand('copy')
  } finally {
    document.body.removeChild(textarea)
  }
}

function normalizeStorageSegment(value: string): string {
  return value
    .trim()
    .replace(/[^a-zA-Z0-9._:-]+/g, '_')
    .slice(0, 120) || 'message'
}
