import { useEffect, useRef, useState, useCallback } from 'react'
import { nodeApi } from '../node/localNodeApi'
import { safeNodeAdminUrl, clean } from '../../lib/utils'
import type { AgentRunsData, AgentRunEntry } from './types'
import styles from './AgentRunsPanel.module.css'

const POLL_INTERVAL = 4500

interface Props {
  projectId: string
  workspacePath: string
}

export default function AgentRunsPanel({ workspacePath }: Omit<Props, 'projectId'>) {
  const adminUrl = safeNodeAdminUrl()
  const [data, setData] = useState<AgentRunsData | null>(null)
  const [loading, setLoading] = useState(false)
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
        recentTasks: Array.isArray(r.recent_tasks ?? r.recentTasks)
          ? (r.recent_tasks ?? r.recentTasks) as Record<string, unknown>[]
          : [],
        recoveryEntry: (r.recovery_entry ?? null) as Record<string, unknown> | null,
        logDir: clean(r.log_dir ?? r.logDir),
        workspacePath: clean(r.workspace_path ?? r.workspacePath),
        loadedAt: Date.now(),
      })
    } catch (err) {
      setData({ runs: [], activeControls: [], recentTasks: [], recoveryEntry: null, logDir: '', workspacePath, loadedAt: Date.now(), error: (err as Error).message })
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

  if (!data) return <div className={styles.panel}><p className={styles.status}>读取中…</p></div>
  if (data.error) return (
    <div className={styles.panel}>
      <div className={styles.panelHead}><strong>本机 Agent 运行</strong><button onClick={() => load(true)}>刷新</button></div>
      <p className={styles.status}>{data.error}</p>
    </div>
  )

  const hasContent = data.recoveryEntry || data.activeControls.length || data.recentTasks.length || data.runs.length
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
        {data.recoveryEntry && <RecoveryItem entry={data.recoveryEntry} />}
        {data.activeControls.map((ctrl, i) => <ControlItem key={i} control={ctrl} />)}
        {data.recentTasks.slice(0, 3).map((task, i) => <RecentTaskItem key={i} task={task} />)}
        {data.runs.slice(0, 3).map((run, i) => <RunItem key={i} run={run} />)}
      </div>
    </div>
  )
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

function shortRunId(value: string): string {
  const text = clean(value) || 'agent run'
  if (text.length <= 24) return text
  return `${text.slice(0, 12)}…${text.slice(-8)}`
}

function RecoveryItem({ entry }: { entry: Record<string, unknown> }) {
  const taskId = clean(entry.task_id ?? entry.taskId ?? '')
  const cliName = clean(entry.cli_name ?? entry.cliName ?? 'agent')
  const action = clean(entry.recommended_action ?? entry.recommendedAction ?? '').toLowerCase()
  return (
    <article className={[styles.item, styles.running].join(' ')}>
      <div className={styles.itemMain}>
        <span className={styles.badge}>推荐恢复</span>
        <strong>{shortRunId(taskId || cliName)}</strong>
        <small>{cliName} · {action === 'continue_from_snapshot' ? '基于快照继续' : '等待或停止'}</small>
      </div>
    </article>
  )
}

function ControlItem({ control }: { control: Record<string, unknown> }) {
  const taskId = clean(control.task_id ?? control.taskId ?? control.run_handle_id ?? control.runHandleId ?? '')
  const cliName = clean(control.cli_name ?? control.cliName ?? 'agent')
  const route = clean(control.route ?? 'local-runtime')
  return (
    <article className={[styles.item, styles.running].join(' ')}>
      <div className={styles.itemMain}>
        <span className={styles.badge}>运行中</span>
        <strong>{shortRunId(taskId || route)}</strong>
        <small>{cliName} · {route}</small>
      </div>
    </article>
  )
}

function RecentTaskItem({ task }: { task: Record<string, unknown> }) {
  const taskId = clean(task.task_id ?? task.taskId ?? task.req_id ?? task.reqId ?? '')
  const status = clean(task.status ?? '')
  const cliName = clean(task.cli_name ?? task.cliName ?? 'agent')
  return (
    <article className={[styles.item, statusTone(status)].join(' ')}>
      <div className={styles.itemMain}>
        <span className={styles.badge}>{statusLabel(status)}</span>
        <strong>{shortRunId(taskId || cliName)}</strong>
        <small>{cliName}</small>
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
