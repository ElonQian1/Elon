import { useEffect, useRef, useState, useCallback } from 'react'
import { nodeApi } from '../node/localNodeApi'
import { safeNodeAdminUrl, clean } from '../../lib/utils'
import SidecarTerminalPanel from './SidecarTerminalPanel'
import type { AgentRunsData, AgentRunEntry, SidecarSession } from './types'
import {
  buildAgentRunParallelOverview,
  shortRunId,
  type AgentRunParallelOverview,
  type RecoveryView,
} from './agentRunRecoveryModel'
import styles from './AgentRunsPanel.module.css'

const POLL_INTERVAL = 4500

interface Props {
  projectId: string
  workspacePath: string
  onDraftContinue?: (text: string) => void
}

export default function AgentRunsPanel({ workspacePath, onDraftContinue }: Omit<Props, 'projectId'>) {
  const adminUrl = safeNodeAdminUrl()
  const [data, setData] = useState<AgentRunsData | null>(null)
  const [loading, setLoading] = useState(false)
  const [actionState, setActionState] = useState<Record<string, string>>({})
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null)

  const load = useCallback(async (force = false) => {
    if (loading && !force) return
    setLoading(true)
    try {
      const resp = await nodeApi<unknown>(adminUrl, '/api/project-agent-runs', {
        method: 'POST',
        body: JSON.stringify({ workspace_path: workspacePath, limit: 8, event_limit: 8 }),
      })
      const r = resp as Record<string, unknown>
      setData({
        runs: Array.isArray(r.runs) ? r.runs as AgentRunsData['runs'] : [],
        activeControls: Array.isArray(r.active_controls ?? r.activeControls)
          ? (r.active_controls ?? r.activeControls) as Record<string, unknown>[]
          : [],
        sidecarSessions: Array.isArray(r.sidecar_sessions ?? r.sidecarSessions)
          ? (r.sidecar_sessions ?? r.sidecarSessions) as SidecarSession[]
          : [],
        recentTasks: Array.isArray(r.recent_tasks ?? r.recentTasks)
          ? (r.recent_tasks ?? r.recentTasks) as Record<string, unknown>[]
          : [],
        recoveryEntry: (r.recovery_entry ?? null) as Record<string, unknown> | null,
        logDir: clean(r.log_dir ?? r.logDir),
        workspacePath: clean(r.workspace_path ?? r.workspacePath),
        loadedAt: Date.now(),
      })
    } catch (err) {
      setData({ runs: [], activeControls: [], sidecarSessions: [], recentTasks: [], recoveryEntry: null, logDir: '', workspacePath, loadedAt: Date.now(), error: (err as Error).message })
    } finally {
      setLoading(false)
    }
  }, [adminUrl, workspacePath, loading])

  useEffect(() => {
    load(true)
    pollRef.current = setInterval(() => load(), POLL_INTERVAL)
    return () => { if (pollRef.current) clearInterval(pollRef.current) }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [workspacePath])

  const cancelTask = useCallback(async (taskId: string) => {
    const id = clean(taskId)
    if (!id) return
    setActionState((prev) => ({ ...prev, [id]: '正在停止…' }))
    try {
      await nodeApi(adminUrl, `/api/task-journal/${encodeURIComponent(id)}`, { method: 'POST' })
      setActionState((prev) => ({ ...prev, [id]: '已请求停止' }))
      await load(true)
    } catch (err) {
      setActionState((prev) => ({ ...prev, [id]: (err as Error).message || '停止失败' }))
    }
  }, [adminUrl, load])

  const draftContinue = useCallback(async (view: RecoveryView) => {
    const id = view.taskId || view.title
    try {
      if (onDraftContinue) {
        onDraftContinue(view.continuePrompt)
        setActionState((prev) => ({ ...prev, [id]: '已写入输入框' }))
        return
      }
      await navigator.clipboard.writeText(view.continuePrompt)
      setActionState((prev) => ({ ...prev, [id]: '继续草稿已复制' }))
    } catch (err) {
      setActionState((prev) => ({ ...prev, [id]: (err as Error).message || '复制失败' }))
    }
  }, [onDraftContinue])

  if (!data) return <div className={styles.panel}><p className={styles.status}>读取中…</p></div>
  if (data.error) return (
    <div className={styles.panel}>
      <div className={styles.panelHead}><strong>本机 Agent 运行</strong><button onClick={() => load(true)}>刷新</button></div>
      <p className={styles.status}>{data.error}</p>
    </div>
  )

  const activeSidecar = data.sidecarSessions.find(sidecarAttachable) ?? data.sidecarSessions[0]
  const overview = buildAgentRunParallelOverview({
    recoveryEntry: data.recoveryEntry,
    activeControls: data.activeControls,
    recentTasks: data.recentTasks,
    sidecarSessions: data.sidecarSessions,
    nowMs: data.loadedAt,
  })
  const hasContent = activeSidecar || overview.views.length || data.runs.length
  if (!hasContent) return (
    <div className={styles.panel}>
      <div className={styles.panelHead}><strong>本机 Agent 运行</strong><button onClick={() => load(true)}>刷新</button></div>
      <p className={styles.status}>暂无本机运行记录</p>
    </div>
  )

  return (
    <div className={styles.panel}>
      <div className={styles.panelHead}>
        <strong>本机 Agent 运行</strong>
        <button onClick={() => load(true)}>刷新</button>
      </div>
      <div className={styles.list}>
        {activeSidecar && <SidecarTerminalPanel adminUrl={adminUrl} session={activeSidecar} />}
        {overview.views.length > 0 && <ParallelOverview overview={overview} />}
        {overview.views.slice(0, 6).map((view, i) => (
          <RecoveryItem
            key={`${view.taskId || view.title}-${i}`}
            view={view}
            compact={i > 0 || overview.views.length > 1}
            actionState={actionState}
            onCancel={cancelTask}
            onDraftContinue={draftContinue}
          />
        ))}
        {data.runs.slice(0, 3).map((run, i) => <RunItem key={i} run={run} />)}
      </div>
    </div>
  )
}

function ParallelOverview({ overview }: { overview: AgentRunParallelOverview }) {
  return (
    <div className={styles.parallelOverview} aria-label="项目任务现场总览">
      <div>
        <strong>{overview.headline}</strong>
        <span>{overview.summary}</span>
      </div>
      <div className={styles.parallelCounts}>
        <span data-tone={overview.counts.active ? 'running' : undefined}>运行 {overview.counts.active}</span>
        <span data-tone={overview.counts.sidecar ? 'running' : undefined}>重接 {overview.counts.sidecar}</span>
        <span data-tone={overview.counts.recoverable ? 'running' : undefined}>继续 {overview.counts.recoverable}</span>
        <span data-tone={overview.counts.staleApproval ? 'failed' : undefined}>审批 {overview.counts.staleApproval}</span>
      </div>
    </div>
  )
}

function sidecarAttachable(session: SidecarSession): boolean {
  return session.attachable_after_restart === true
    || session.attachableAfterRestart === true
    || session.capabilities?.terminal_attach === true
}

function statusTone(status: string): string {
  const v = clean(status).toLowerCase()
  if (v === 'running') return styles.running
  if (['completed', 'done'].includes(v)) return styles.done
  if (['failed', 'error', 'canceled', 'cancelled', 'interrupted'].includes(v)) return styles.failed
  return ''
}

function statusLabel(status: string): string {
  const v = clean(status).toLowerCase()
  if (v === 'running') return '运行中'
  if (['completed', 'done'].includes(v)) return '完成'
  if (['canceled', 'cancelled', 'interrupted'].includes(v)) return '已停止'
  if (['failed', 'error'].includes(v)) return '失败'
  return v || '未知'
}

function RecoveryItem({
  view,
  actionState,
  compact,
  onCancel,
  onDraftContinue,
}: {
  view: RecoveryView
  actionState: Record<string, string>
  compact?: boolean
  onCancel: (taskId: string) => void
  onDraftContinue: (view: RecoveryView) => void
}) {
  const stateText = clean(actionState[view.taskId] ?? actionState[view.title])
  return (
    <article className={[styles.item, styles[`tone_${view.tone}`], compact ? styles.compact : ''].join(' ')}>
      <div className={styles.itemMain}>
        <div className={styles.recoveryHead}>
          <span className={styles.badge}>{view.badge}</span>
          <strong>{view.title}</strong>
        </div>
        <small>{view.summary}</small>
        {!compact && <p className={styles.recoveryDetail}>{view.detail}</p>}
        <div className={styles.recoveryStage} data-tone={view.stageTone} data-stale={view.stale ? 'true' : undefined}>
          <strong>{view.stageTitle}</strong>
          {view.stageMeta && <span>{view.stageMeta}</span>}
        </div>
        {!compact && <p className={styles.recoveryStageDetail}>{view.stageDetail}</p>}
        <div className={styles.recoveryFacts}>
          {view.facts.slice(0, compact ? 4 : 8).map((fact) => (
            <span key={`${fact.label}:${fact.value}`} data-tone={fact.tone || undefined}>
              {fact.label}：{fact.value}
            </span>
          ))}
        </div>
        {(stateText || view.canCancel || view.canContinue) && (
          <div className={styles.recoveryActions}>
            {view.canCancel && (
              <button type="button" className={styles.stopBtn} onClick={() => onCancel(view.taskId)}>
                停止
              </button>
            )}
            {view.canContinue && (
              <button type="button" className={styles.continueBtn} onClick={() => onDraftContinue(view)}>
                继续草稿
              </button>
            )}
            {stateText && <span>{stateText}</span>}
          </div>
        )}
      </div>
    </article>
  )
}

function RunItem({ run }: { run: AgentRunEntry }) {
  const runId = clean(run.run_id ?? run.runId ?? run.file_name ?? run.fileName ?? '')
  const status = clean(run.status ?? '')
  const mode = clean(run.mode ?? 'runtime')
  const tools = (run.tool_names ?? []).filter(Boolean).slice(0, 4)
  return (
    <article className={[styles.item, statusTone(status)].join(' ')}>
      <div className={styles.itemMain}>
        <span className={styles.badge}>{statusLabel(status)}</span>
        <strong>{shortRunId(runId)}</strong>
        <small>{mode}{Number(run.turn_count ?? 0) > 0 ? ` · ${run.turn_count} 轮` : ''}{Number(run.tool_count ?? 0) > 0 ? ` · ${run.tool_count} 工具` : ''}</small>
        {tools.length > 0 && (
          <div className={styles.tools}>{tools.map((t) => <span key={t}>{t}</span>)}</div>
        )}
      </div>
    </article>
  )
}
