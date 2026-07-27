import { Check, Copy } from 'lucide-react'
import { useState } from 'react'

interface Props {
  threadId?: string
}

export default function CodexThreadBadge({ threadId = '' }: Props) {
  const [copied, setCopied] = useState(false)
  const value = threadId.trim().replace(/^codex:\/\/threads\//i, '')
  if (!value) return null

  async function copy() {
    try {
      await navigator.clipboard.writeText(value)
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1400)
    } catch {
      setCopied(false)
    }
  }

  return (
    <div
      data-codex-thread-id={value}
      title={`codex://threads/${value}`}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        width: 'fit-content',
        maxWidth: '100%',
        margin: '3px 0 8px',
        color: 'var(--conversation-text-muted)',
        fontSize: 12,
      }}
    >
      <span>Codex 本机会话</span>
      <code style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
        {value}
      </code>
      <button
        type="button"
        onClick={copy}
        aria-label="复制 Codex 本机会话 ID"
        title="复制 Codex 本机会话 ID"
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          border: 0,
          padding: 2,
          color: 'inherit',
          background: 'transparent',
          cursor: 'pointer',
        }}
      >
        {copied ? <Check size={13} /> : <Copy size={13} />}
      </button>
    </div>
  )
}
