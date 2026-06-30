import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { FormEvent, KeyboardEvent } from 'react'
import { Maximize2, PlugZap, RefreshCw, Send } from 'lucide-react'
import { nodeApi } from '../node/localNodeApi'
import { clean } from '../../lib/utils'
import type { SidecarAttachResponse, SidecarOutputRecord, SidecarSession } from './types'
import styles from './AgentRunsPanel.module.css'

const POLL_INTERVAL_MS = 1200
const MAX_TERMINAL_CHARS = 80_000
const ANSI_PATTERN = /\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])/g

interface Props {
  adminUrl: string
  session: SidecarSession
}

export default function SidecarTerminalPanel({ adminUrl, session }: Props) {
  const taskId = useMemo(() => clean(session.task_id ?? session.taskId), [session])
  const cliName = clean(session.cli_name ?? session.cliName ?? 'AI CLI')
  const transport = clean(session.transport ?? 'pty')
  const [output, setOutput] = useState('')
  const [input, setInput] = useState('')
  const [offset, setOffset] = useState(0)
  const [status, setStatus] = useState('未连接')
  const [busy, setBusy] = useState(false)
  const outputRef = useRef<HTMLPreElement>(null)
  const terminalRef = useRef<HTMLDivElement>(null)
  const offsetRef = useRef(0)
  const attachBusyRef = useRef(false)
  const sizeRef = useRef({ cols: 0, rows: 0 })

  const attach = useCallback(async (force = false) => {
    if (!taskId || attachBusyRef.current) return
    attachBusyRef.current = true
    setStatus(force ? '重连中' : '同步中')
    try {
      const data = await nodeApi<SidecarAttachResponse>(
        adminUrl,
        `/api/cli-sidecars/${encodeURIComponent(taskId)}/attach?since=${offsetRef.current}&limit=300`,
        {},
        10_000,
      )
      const records = data.output_records ?? []
      if (records.length > 0) {
        setOutput((prev) => trimTerminalText(prev + records.map(formatOutputRecord).join('')))
      }
      const nextOffset = Number(data.next_offset ?? offsetRef.current)
      if (Number.isFinite(nextOffset) && nextOffset >= 0) {
        offsetRef.current = nextOffset
        setOffset(nextOffset)
      }
      setStatus(data.attached === false ? '已断开' : '已连接')
    } catch (err) {
      setStatus((err as { message?: string }).message ?? '连接失败')
    } finally {
      attachBusyRef.current = false
    }
  }, [adminUrl, taskId])

  useEffect(() => {
    offsetRef.current = 0
    setOffset(0)
    setOutput('')
    setStatus('连接中')
    void attach(true)
    const timer = window.setInterval(() => { void attach() }, POLL_INTERVAL_MS)
    return () => window.clearInterval(timer)
  }, [attach, taskId])

  useEffect(() => {
    const el = outputRef.current
    if (!el) return
    el.scrollTop = el.scrollHeight
  }, [output])

  useEffect(() => {
    const el = terminalRef.current
    if (!el || !taskId) return
    const observer = new ResizeObserver(([entry]) => {
      const rect = entry?.contentRect
      if (!rect) return
      const cols = clamp(Math.floor((rect.width - 18) / 8), 20, 160)
      const rows = clamp(Math.floor((rect.height - 18) / 18), 8, 60)
      if (sizeRef.current.cols === cols && sizeRef.current.rows === rows) return
      sizeRef.current = { cols, rows }
      void sendResize(cols, rows)
    })
    observer.observe(el)
    return () => observer.disconnect()
  }, [adminUrl, taskId])

  async function sendResize(cols: number, rows: number) {
    if (!taskId) return
    try {
      await nodeApi(
        adminUrl,
        `/api/cli-sidecars/${encodeURIComponent(taskId)}/resize`,
        { method: 'POST', body: JSON.stringify({ cols, rows }) },
        8_000,
      )
    } catch {
      // Size sync is best-effort; output polling will surface hard connection errors.
    }
  }

  async function handleSubmit(event: FormEvent) {
    event.preventDefault()
    await sendInput()
  }

  async function sendInput() {
    const text = input
    if (!taskId || !text.trim() || busy) return
    setBusy(true)
    try {
      await nodeApi(
        adminUrl,
        `/api/cli-sidecars/${encodeURIComponent(taskId)}/input`,
        { method: 'POST', body: JSON.stringify({ text: terminalInput(text) }) },
        8_000,
      )
      setInput('')
      setStatus('已发送')
      await attach(true)
    } catch (err) {
      setStatus((err as { message?: string }).message ?? '发送失败')
    } finally {
      setBusy(false)
    }
  }

  function handleKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault()
      void sendInput()
    }
  }

  const sessionId = clean(session.session_id ?? session.sessionId)

  return (
    <section className={styles.terminal}>
      <div className={styles.terminalHead}>
        <div className={styles.terminalTitle}>
          <PlugZap size={14} aria-hidden="true" />
          <strong>{cliName}</strong>
          <span>{shortId(taskId || sessionId)}</span>
        </div>
        <div className={styles.terminalActions}>
          <span title={transport}>{status}</span>
          <button type="button" title="同步尺寸" onClick={() => {
            const size = sizeRef.current
            void sendResize(size.cols || 96, size.rows || 24)
          }}>
            <Maximize2 size={13} aria-hidden="true" />
          </button>
          <button type="button" title="重连" onClick={() => { void attach(true) }}>
            <RefreshCw size={13} aria-hidden="true" />
          </button>
        </div>
      </div>
      <div ref={terminalRef} className={styles.terminalOutputWrap}>
        <pre ref={outputRef} className={styles.terminalOutput}>
          {output || '等待终端输出'}
        </pre>
      </div>
      <form className={styles.terminalForm} onSubmit={(event) => { void handleSubmit(event) }}>
        <textarea
          value={input}
          onChange={(event) => setInput(event.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="输入命令"
          rows={1}
          disabled={busy || !taskId}
        />
        <button type="submit" title="发送" disabled={busy || !taskId || !input.trim()}>
          <Send size={14} aria-hidden="true" />
        </button>
      </form>
      <div className={styles.terminalMeta}>
        <span>{transport}</span>
        <span>{offset > 0 ? `${offset} bytes` : '0 bytes'}</span>
      </div>
    </section>
  )
}

function terminalInput(value: string): string {
  return /[\r\n]$/.test(value) ? value : `${value}\r\n`
}

function formatOutputRecord(record: SidecarOutputRecord): string {
  const kind = clean(record.type ?? record.record_type)
  if (kind === 'chunk') return normalizeTerminalText(String(record.text ?? ''))
  if (kind === 'child_started') return `\n[pid ${record.child_pid ?? '?'}]\n`
  if (kind === 'exit') {
    if (record.error) return `\n[error] ${record.error}\n`
    if (record.canceled) return '\n[canceled]\n'
    return record.success === false ? '\n[failed]\n' : '\n[done]\n'
  }
  return ''
}

function normalizeTerminalText(value: string): string {
  return value
    .replace(ANSI_PATTERN, '')
    .replace(/\r\n/g, '\n')
    .replace(/\r/g, '\n')
}

function trimTerminalText(value: string): string {
  return value.length > MAX_TERMINAL_CHARS ? value.slice(-MAX_TERMINAL_CHARS) : value
}

function shortId(value: string): string {
  if (!value) return 'sidecar'
  return value.length > 18 ? `${value.slice(0, 9)}…${value.slice(-6)}` : value
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max)
}
