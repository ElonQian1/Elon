import { useCallback, useEffect, useMemo, useState } from 'react'
import { Bot, Download, Focus, PanelTopOpen, RefreshCw, RotateCw, ScanSearch, TerminalSquare } from 'lucide-react'
import {
  exportWinDiagnostics, fetchWinControlCapabilities, fetchWinDiagnostics,
  fetchWinTimeline, queueWinAction,
} from './codexControlApi'
import type { WinActionKind, WinControlCapabilities, WinControlEvent, WinLogSource } from './types'
import styles from './CodexControlPage.module.css'

const SOURCES: Array<{ id: WinLogSource; label: string }> = [
  { id: 'frontend', label: '前端' }, { id: 'rust', label: 'Rust' },
  { id: 'cli', label: 'CLI' }, { id: 'network', label: '网络' },
  { id: 'tauri', label: 'Tauri' }, { id: 'control', label: '控制' },
]

export default function CodexControlPage() {
  const [capabilities, setCapabilities] = useState<WinControlCapabilities | null>(null)
  const [events, setEvents] = useState<WinControlEvent[]>([])
  const [sources, setSources] = useState<WinLogSource[]>(SOURCES.map((item) => item.id))
  const [route, setRoute] = useState('/codex-control')
  const [diagnostics, setDiagnostics] = useState<Record<string, unknown> | null>(null)
  const [busy, setBusy] = useState('')
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')

  const refresh = useCallback(async () => {
    try {
      const [nextCapabilities, timeline] = await Promise.all([
        fetchWinControlCapabilities(), fetchWinTimeline(sources),
      ])
      setCapabilities(nextCapabilities)
      setEvents(Array.isArray(timeline.events) ? timeline.events : [])
      setError('')
    } catch (reason) {
      setError(message(reason))
    }
  }, [sources])

  useEffect(() => {
    void refresh()
    const timer = window.setInterval(() => { void refresh() }, 2_500)
    return () => window.clearInterval(timer)
  }, [refresh])

  useEffect(() => {
    fetchWinDiagnostics().then((result) => setDiagnostics(result.diagnostics ?? null)).catch(() => {})
  }, [])

  async function run(kind: WinActionKind, targetRoute?: string) {
    setBusy(kind)
    setNotice('')
    setError('')
    try {
      const action = await queueWinAction(kind, targetRoute)
      setNotice(`动作 ${action.action_id} 已排队；等待 Tauri 成功回执。`)
      await refresh()
    } catch (reason) {
      setError(message(reason))
    } finally {
      setBusy('')
    }
  }

  async function exportBundle() {
    setBusy('export')
    try {
      const result = await exportWinDiagnostics()
      setNotice(result.path ? `脱敏诊断包：${result.path}` : (result.message || '诊断包已生成'))
    } catch (reason) {
      setError(message(reason))
    } finally {
      setBusy('')
    }
  }

  const counts = useMemo(() => SOURCES.reduce<Record<string, number>>((all, source) => {
    all[source.id] = events.filter((event) => event.source === source.id).length
    return all
  }, {}), [events])

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <div className={styles.heading}>
          <span className={styles.iconBox}><TerminalSquare size={19} /></span>
          <div><span>CODEX · WIN CONTROL</span><h1>语义控制与统一日志</h1></div>
        </div>
        <div className={styles.headerActions}>
          <button type="button" onClick={() => void refresh()}><RefreshCw size={14} />刷新</button>
          <button type="button" onClick={() => void exportBundle()} disabled={busy === 'export'}><Download size={14} />导出脱敏包</button>
        </div>
      </header>

      {error && <div className={styles.error}>{error}</div>}
      {notice && <div className={styles.notice}>{notice}</div>}

      <section className={styles.statusGrid}>
        <StatusCard label="前端桥" active={!!capabilities?.frontend_available} detail="错误、路由和无正文网络元数据" />
        <StatusCard label="Tauri 宿主" active={!!capabilities?.tauri_available} detail="白名单原生命令与回执" />
        <StatusCard label="Codex MCP" active={capabilities?.schema === 'elon.win_codex_control.v1'} detail="项目绑定 · 短期 token · profile 固定" />
        <StatusCard label="敏感正文" active={false} inverted detail="Cookie、token、prompt、body 均不采集" />
      </section>

      <div className={styles.workspace}>
        <aside className={styles.controls}>
          <section>
            <h2><Bot size={15} />语义动作</h2>
            <p>排队不等于成功；必须看到 Tauri 回执。</p>
            <div className={styles.actionGrid}>
              <button type="button" onClick={() => void run('show_window')} disabled={!!busy}><PanelTopOpen size={14} />显示窗口</button>
              <button type="button" onClick={() => void run('focus_window')} disabled={!!busy}><Focus size={14} />聚焦窗口</button>
              <button type="button" onClick={() => void run('reload_page')} disabled={!!busy}><RotateCw size={14} />刷新页面</button>
              <button type="button" onClick={() => void run('capture_state')} disabled={!!busy}><ScanSearch size={14} />捕获状态</button>
              <button type="button" onClick={() => void run('open_devtools')} disabled={!!busy}>打开 DevTools</button>
              <button type="button" onClick={() => void run('close_devtools')} disabled={!!busy}>关闭 DevTools</button>
            </div>
            <label className={styles.routeField}>目标路由
              <select value={route} onChange={(event) => setRoute(event.target.value)}>
                {(capabilities?.routes ?? ['/codex-control']).map((item) => <option key={item} value={item}>{item}</option>)}
              </select>
            </label>
            <button className={styles.primary} type="button" onClick={() => void run('navigate', route)} disabled={!!busy}>导航到页面</button>
          </section>
          <section>
            <h2>诊断合同</h2>
            <Fact label="事件保留" value={`${capabilities?.retention?.events ?? '—'} 条`} />
            <Fact label="动作 TTL" value={`${Math.round((capabilities?.retention?.action_ttl_ms ?? 0) / 1000)} 秒`} />
            <Fact label="节点版本" value={diagnosticVersion(diagnostics)} />
            <Fact label="任意脚本" value="禁止" />
            <Fact label="请求正文" value="不采集" />
          </section>
        </aside>

        <main className={styles.timeline}>
          <div className={styles.timelineHeader}>
            <div><h2>统一时间线</h2><span>{events.length} 条可见事件</span></div>
            <div className={styles.filters}>
              {SOURCES.map((source) => <button
                type="button" key={source.id} data-active={sources.includes(source.id)}
                onClick={() => setSources(toggleSource(sources, source.id))}
              >{source.label}<b>{counts[source.id] ?? 0}</b></button>)}
            </div>
          </div>
          <div className={styles.eventList}>
            {events.length ? events.map((event) => <EventRow key={event.event_id} event={event} />) : <div className={styles.empty}>等待前端、Rust、CLI、网络或 Tauri 事件…</div>}
          </div>
        </main>
      </div>
    </div>
  )
}

function StatusCard({ label, active, detail, inverted = false }: { label: string; active: boolean; detail: string; inverted?: boolean }) {
  const healthy = inverted ? !active : active
  return <article className={styles.statusCard} data-active={healthy}><span>{healthy ? 'READY' : 'OFFLINE'}</span><strong>{label}</strong><p>{detail}</p></article>
}

function EventRow({ event }: { event: WinControlEvent }) {
  return <article className={styles.event} data-level={event.level}>
    <time>{formatTime(event.at_ms)}</time><span className={styles.source} data-source={event.source}>{event.source}</span>
    <div><header><strong>{event.kind}</strong><code>{event.trace_id || '—'}</code></header><p>{event.summary}</p>
      {Object.keys(event.fields ?? {}).length > 0 && <details><summary>字段</summary><pre>{JSON.stringify(event.fields, null, 2)}</pre></details>}
    </div>
  </article>
}

function Fact({ label, value }: { label: string; value: string }) { return <div className={styles.fact}><span>{label}</span><strong>{value}</strong></div> }
function formatTime(value: number) { return value ? new Date(value).toLocaleTimeString('zh-CN', { hour12: false }) : '—' }
function message(reason: unknown) { return reason instanceof Error ? reason.message : String(reason) }
function diagnosticVersion(value: Record<string, unknown> | null) {
  const runtime = value?.runtime as Record<string, unknown> | undefined
  return String(runtime?.version ?? '—')
}
function toggleSource(current: WinLogSource[], source: WinLogSource) {
  return current.includes(source) ? current.filter((item) => item !== source) : [...current, source]
}
