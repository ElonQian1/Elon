import { useEffect, useMemo, useRef, useState } from 'react'
import { Check, ChevronDown, Copy, GitFork, ThumbsDown, ThumbsUp } from 'lucide-react'
import { copyRichTextToClipboard, copyTextToClipboard, sanitizedRichHtmlFromElement } from '../../lib/clipboard'
import styles from './MessageActions.module.css'

export type MessageFeedbackValue = 'up' | 'down' | null
type CopyStatus = 'idle' | 'markdown' | 'rich' | 'failed'

interface MessageActionsProps {
  content: string
  messageKey: string
  storageScope: string
  align?: 'left' | 'right'
  richCopySourceId?: string
  onFeedbackChange?: (value: MessageFeedbackValue) => void
  onFork?: () => void | Promise<void>
}

const FEEDBACK_PREFIX = 'elon.pc.messageFeedback'

export default function MessageActions({
  content,
  messageKey,
  storageScope,
  align = 'left',
  richCopySourceId,
  onFeedbackChange,
  onFork,
}: MessageActionsProps) {
  const text = content.trim()
  const [copyStatus, setCopyStatus] = useState<CopyStatus>('idle')
  const [copyMenuOpen, setCopyMenuOpen] = useState(false)
  const [forking, setForking] = useState(false)
  const [feedback, setFeedback] = useState<MessageFeedbackValue>(null)
  const copyMenuRef = useRef<HTMLDivElement>(null)

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
    if (copyStatus === 'idle') return
    const timer = window.setTimeout(() => setCopyStatus('idle'), 1400)
    return () => window.clearTimeout(timer)
  }, [copyStatus])

  useEffect(() => {
    if (!copyMenuOpen) return

    function handlePointerDown(event: PointerEvent) {
      if (copyMenuRef.current?.contains(event.target as Node)) return
      setCopyMenuOpen(false)
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') setCopyMenuOpen(false)
    }

    document.addEventListener('pointerdown', handlePointerDown)
    document.addEventListener('keydown', handleKeyDown)
    return () => {
      document.removeEventListener('pointerdown', handlePointerDown)
      document.removeEventListener('keydown', handleKeyDown)
    }
  }, [copyMenuOpen])

  if (!text) return null

  async function copyMarkdown() {
    const ok = await copyTextToClipboard(text)
    setCopyStatus(ok ? 'markdown' : 'failed')
    setCopyMenuOpen(false)
  }

  async function copyRichText() {
    const html = sanitizedRichHtmlFromElement(richCopySourceId ? document.getElementById(richCopySourceId) : null)
    const result = await copyRichTextToClipboard(html, text)
    setCopyStatus(result === 'rich' ? 'rich' : result === 'text' ? 'markdown' : 'failed')
    setCopyMenuOpen(false)
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

  async function handleFork() {
    if (!onFork || forking) return
    setForking(true)
    try {
      await onFork()
    } catch (err) {
      if (typeof window !== 'undefined') {
        window.alert(err instanceof Error ? err.message : '分叉会话失败')
      }
    } finally {
      setForking(false)
    }
  }

  const containerClassName = [
    styles.actions,
    align === 'right' ? styles.right : styles.left,
  ].join(' ')
  const copied = copyStatus === 'markdown' || copyStatus === 'rich'
  const copyTitle = copyStatus === 'markdown'
    ? '已复制为 Markdown'
    : copyStatus === 'rich'
      ? '已复制为富文本'
      : copyStatus === 'failed'
        ? '复制失败'
        : '复制'

  return (
    <div className={containerClassName} role="group" aria-label="消息操作">
      <div className={styles.copyMenuWrap} ref={copyMenuRef}>
        <button
          className={[styles.button, copied ? styles.copied : copyStatus === 'failed' ? styles.copyFailed : ''].filter(Boolean).join(' ')}
          type="button"
          title={copyTitle}
          aria-label={copyTitle}
          aria-haspopup="menu"
          aria-expanded={copyMenuOpen}
          onClick={() => setCopyMenuOpen((open) => !open)}
        >
          {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
          <ChevronDown className={styles.chevron} aria-hidden="true" />
        </button>
        {copyMenuOpen && (
          <div className={styles.copyMenu} role="menu">
            <button type="button" role="menuitem" onClick={copyMarkdown}>复制为 Markdown</button>
            <button type="button" role="menuitem" onClick={copyRichText}>复制为富文本</button>
          </div>
        )}
      </div>
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
      {onFork && (
        <button
          className={styles.button}
          type="button"
          title={forking ? '正在分叉...' : '分叉会话'}
          aria-label={forking ? '正在分叉会话' : '从此处开始分叉会话'}
          aria-busy={forking}
          disabled={forking}
          onClick={handleFork}
        >
          <GitFork aria-hidden="true" />
        </button>
      )}
    </div>
  )
}

function normalizeStorageSegment(value: string): string {
  return value
    .trim()
    .replace(/[^a-zA-Z0-9._:-]+/g, '_')
    .slice(0, 120) || 'message'
}

export function messageCopySourceId(storageScope: string, messageKey: string): string {
  return `message-copy-${normalizeStorageSegment(storageScope)}-${normalizeStorageSegment(messageKey)}`
}
