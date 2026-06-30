import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { FormEvent, KeyboardEvent } from 'react'
import { Maximize2, PlugZap, RefreshCw, Send } from 'lucide-react'
import { nodeApi } from '../node/localNodeApi'
import { clean } from '../../lib/utils'
import { normalizeTerminalChunk, parseTerminalSegments, type TerminalSegment } from './terminalAnsi'
import type { SidecarAttachResponse, SidecarOutputRecord, SidecarSession } from './types'
import styles from './AgentRunsPanel.module.css'

const POLL_INTERVAL_MS = 1200
const MAX_TERMINAL_CHARS = 80_000

interface Props {
  adminUrl: string
  session: SidecarSession
}

export default function SidecarTerminalPanel({ adminUrl, session }: Props) {
  const taskId = useMemo(() => clean(session.task_id ?? session.taskId), [session])
  const cliName = clean(session.cli_name ?? session.cliName ?? 'AI CLI')
  const transport = clean(session.transport ?? 'pty')
  const sessionState = clean(session.state ?? 'running')
  const canAttachAfterRestart = session.attachable_after_restart === true
    || session.attachableAfterRestart === true
    || session.capabilities?.terminal_attach === true
  const canApproveAfterRestart = session.approval_recoverable_after_restart === true
    || session.approvalRecoverableAfterRestart === true
    || session.capabilities?.tool_approval_recovery === true
  const [rawOutput, setRawOutput] = useState('')
  const [input, setInput] = useState('')
  const [offset, setOffset] = useState(0)
  const [status, setStatus] = useState('未连接')
  const [busy, setBusy] = useState(false)
  const [terminalSize, setTerminalSize] = useState({ cols: 0, rows: 0 })
  const outputRef = useRef<HTMLPreElement>(null)
  const terminalRef = useRef<HTMLDivElement>(null)
  const offsetRef = useRef(0)
  const attachBusyRef = useRef(false)
  const sizeRef = useRef({ cols: 0, rows: 0 })
  const segments = useMemo(() => parseTerminalSegments(rawOutput), [rawOutput])

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
        setRawOutput((prev) => trimTerminalText(prev + records.map(formatOutputRecord).join('')))
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
    setRawOutput('')
    setStatus('连接中')
    void attach(true)
    const timer = window.setInterval(() => { void attach() }, POLL_INTERVAL_MS)
    return () => window.clearInterval(timer)
  }, [attach, taskId])

  useEffect(() => {
    const el = outputRef.current
    if (!el) return
    el.scrollTop = el.scrollHeight
  }, [rawOutput])

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
      setTerminalSize({ cols, rows })
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
  const statusClass = terminalStatusClass(status, sessionState)

  return (
    <section className={styles.terminal}>
      <div className={styles.terminalHead}>
        <div className={styles.terminalTitle}>
          <PlugZap size={14} aria-hidden="true" />
          <strong>{cliName}</strong>
          <span>{shortId(taskId || sessionId)}</span>
        </div>
        <div className={styles.terminalActions}>
          <span className={statusClass} title={`${transport} · ${sessionState}`}>{statusLabel(status, sessionState)}</span>
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
      <div className={styles.terminalCapabilities}>
        <span>{transportLabel(transport)}</span>
        {canAttachAfterRestart && <span>重启可恢复</span>}
        {canApproveAfterRestart && <span>审批可恢复</span>}
      </div>
      <div ref={terminalRef} className={styles.terminalOutputWrap}>
        <pre ref={outputRef} className={styles.terminalOutput}>
          {segments.length > 0
            ? segments.map((segment, index) => (
              <span key={`${index}-${segment.text.length}`} className={segmentClassName(segment)}>
                {segment.text}
              </span>
            ))
            : '等待终端输出'}
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
        <span>{terminalSize.cols > 0 ? `${terminalSize.cols}x${terminalSize.rows}` : 'auto size'}</span>
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
  if (kind === 'chunk') return normalizeTerminalChunk(String(record.text ?? ''))
  if (kind === 'child_started') return `\n[pid ${record.child_pid ?? '?'}]\n`
  if (kind === 'exit') {
    if (record.error) return `\n[error] ${record.error}\n`
    if (record.canceled) return '\n[canceled]\n'
    return record.success === false ? '\n[failed]\n' : '\n[done]\n'
  }
  return ''
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

function statusLabel(status: string, sessionState: string): string {
  const s = clean(status)
  if (s === '已连接' && sessionState) return stateLabel(sessionState)
  return s || stateLabel(sessionState)
}

function stateLabel(state: string): string {
  const value = clean(state).toLowerCase()
  if (value === 'running') return '运行中'
  if (value === 'finished') return '已完成'
  if (value === 'failed') return '失败'
  if (value === 'canceled' || value === 'cancel_requested') return '已停止'
  return value || '同步中'
}

function terminalStatusClass(status: string, sessionState: string): string {
  const value = `${status} ${sessionState}`.toLowerCase()
  if (value.includes('失败') || value.includes('failed')) return styles.terminalStateBad
  if (value.includes('停止') || value.includes('cancel')) return styles.terminalStateWarn
  if (value.includes('运行') || value.includes('connected') || value.includes('running') || value.includes('已连接')) {
    return styles.terminalStateGood
  }
  return ''
}

function transportLabel(transport: string): string {
  const value = transport.toLowerCase()
  if (value.includes('conpty')) return '托管 ConPTY'
  if (value.includes('pty')) return '托管 PTY'
  return transport || '托管终端'
}

function segmentClassName(segment: TerminalSegment): string {
  const classes = [
    segment.bold ? styles.ansiBold : '',
    segment.dim ? styles.ansiDim : '',
    segment.underline ? styles.ansiUnderline : '',
    segment.inverse ? styles.ansiInverse : '',
    segment.fg ? styles[`ansiFg${capitalize(segment.fg)}`] : '',
    segment.bg ? styles[`ansiBg${capitalize(segment.bg)}`] : '',
  ].filter(Boolean)
  return classes.join(' ')
}

function capitalize(value: string): string {
  return value ? `${value[0].toUpperCase()}${value.slice(1)}` : value
}
