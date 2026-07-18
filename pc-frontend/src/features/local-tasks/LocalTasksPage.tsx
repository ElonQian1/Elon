import { useCallback, useEffect, useState } from 'react'
import { HardDrive, RefreshCw, WifiOff } from 'lucide-react'
import { isLocalWorkbench } from '../../api/runtime'
import { safeNodeAdminUrl } from '../../lib/utils'
import { readableTaskTitle } from '../../lib/taskTitle'
import { ensureLocalFullAccessGrant } from '../conversation/localPcRuntime'
import LocalTaskCreateForm from './LocalTaskCreateForm'
import LocalTaskDetailPanel from './LocalTaskDetailPanel'
import {
  cancelLocalTask,
  createLocalTask,
  decideLocalTaskApproval,
  getLocalTask,
  listLocalTasks,
} from './localTaskApi'
import {
  localTaskStatus,
  mergeLocalTaskDetail,
  normalizeLocalTaskDetail,
  normalizeLocalTaskList,
  pendingSyncCountFromList,
  syncStateLabel,
  taskIdFromCreateResponse,
} from './localTaskModel'
import type {
  LocalTaskApproval,
  LocalTaskApprovalDecision,
  LocalTaskCreateInput,
  LocalTaskDetail,
  LocalTaskRecord,
} from './types'
import styles from './LocalTasksPage.module.css'

const LIST_POLL_MS = 5_000
const DETAIL_POLL_MS = 1_600

export default function LocalTasksPage() {
  const [tasks, setTasks] = useState<LocalTaskRecord[]>([])
  const [selectedId, setSelectedId] = useState(() => new URLSearchParams(location.search).get('task') ?? '')
  const [detail, setDetail] = useState<LocalTaskDetail | null>(null)
  const [listLoading, setListLoading] = useState(true)
  const [detailLoading, setDetailLoading] = useState(false)
  const [createBusy, setCreateBusy] = useState(false)
  const [actionKey, setActionKey] = useState('')
  const [pendingSync, setPendingSync] = useState(0)
  const [listStale, setListStale] = useState(false)
  const [error, setError] = useState('')
  const [detailError, setDetailError] = useState('')
  const [notice, setNotice] = useState('')

  const refreshList = useCallback(async (quiet = false) => {
    if (!quiet) setListLoading(true)
    try {
      const response = await listLocalTasks(50)
      const next = normalizeLocalTaskList(response)
      setTasks(next)
      setPendingSync(pendingSyncCountFromList(response))
      setListStale(false)
      setSelectedId((current) => current || next[0]?.id || '')
      if (!quiet) setError('')
    } catch (err) {
      setListStale(true)
      if (!quiet) setError(errorMessage(err, '无法读取本机任务；请确认一龙 PC 节点正在运行。'))
    } finally {
      if (!quiet) setListLoading(false)
    }
  }, [])

  useEffect(() => {
    void refreshList()
    const timer = window.setInterval(() => void refreshList(true), LIST_POLL_MS)
    return () => window.clearInterval(timer)
  }, [refreshList])

  useEffect(() => {
    if (!selectedId) {
      setDetail(null)
      return
    }
    let cancelled = false
    let busy = false
    let since = 0
    setDetail(null)
    setDetailError('')
    setDetailLoading(true)

    async function refreshDetail() {
      if (busy || cancelled) return
      busy = true
      try {
        const response = await getLocalTask(selectedId, since, 200)
        if (cancelled) return
        const incoming = normalizeLocalTaskDetail(response)
        since = Math.max(since, incoming.last_event_seq)
        setDetail((current) => mergeLocalTaskDetail(current, incoming))
        setDetailError('')
      } catch (err) {
        if (!cancelled) setDetailError(errorMessage(err, '无法读取本机任务详情。'))
      } finally {
        busy = false
        if (!cancelled) setDetailLoading(false)
      }
    }

    void refreshDetail()
    const timer = window.setInterval(refreshDetail, DETAIL_POLL_MS)
    return () => {
      cancelled = true
      window.clearInterval(timer)
    }
  }, [selectedId])

  useEffect(() => {
    const url = new URL(location.href)
    if (selectedId) url.searchParams.set('task', selectedId)
    else url.searchParams.delete('task')
    window.history.replaceState(window.history.state, '', url)
  }, [selectedId])

  async function handleCreate(input: LocalTaskCreateInput): Promise<boolean> {
    setCreateBusy(true)
    setError('')
    setNotice('')
    try {
      const grantResult = await ensureLocalFullAccessGrant({
        adminUrl: safeNodeAdminUrl(),
        projectId: input.project_id,
        projectName: input.project_id,
        workspacePath: input.workspace_path,
        runtimePermission: input.runtime_permission,
        useLocalRouteA: true,
      })
      const response = await createLocalTask(input)
      const taskId = taskIdFromCreateResponse(response)
      setNotice(grantResult === 'granted'
        ? '已保存本机目录授权并启动 Codex；即使云端断线，本页仍会从本机节点读取进度。'
        : '本机 Codex 任务已启动；即使云端断线，本页仍会从本机节点读取进度。')
      await refreshList(true)
      if (taskId) setSelectedId(taskId)
      return true
    } catch (err) {
      setError(errorMessage(err, '本机任务启动失败。'))
      return false
    } finally {
      setCreateBusy(false)
    }
  }

  async function handleCancel() {
    if (!selectedId || actionKey) return
    setActionKey('cancel')
    setError('')
    try {
      await cancelLocalTask(selectedId)
      setNotice('已向本机 Codex 发送停止请求。')
      await refreshList(true)
    } catch (err) {
      setError(errorMessage(err, '停止本机任务失败。'))
    } finally {
      setActionKey('')
    }
  }

  async function handleDecision(approval: LocalTaskApproval, decision: LocalTaskApprovalDecision) {
    if (!selectedId || actionKey) return
    setActionKey(`approval:${approval.approval_id}`)
    setError('')
    try {
      await decideLocalTaskApproval(selectedId, approval.approval_id, decision)
      setNotice(decision === 'approve' ? '已在本机批准工具操作。' : '已在本机拒绝工具操作。')
    } catch (err) {
      setError(errorMessage(err, '提交本机工具审批失败。'))
    } finally {
      setActionKey('')
    }
  }

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <div className={styles.heading}>
          <span className={styles.iconBox}><HardDrive size={19} aria-hidden="true" /></span>
          <div>
            <div className={styles.kicker}>Offline-first · Local Codex</div>
            <h1>本机任务</h1>
          </div>
        </div>
        <div className={styles.headerActions}>
          <span className={styles.pendingBadge} data-pending={pendingSync > 0}>
            {pendingSync > 0 ? `${pendingSync} 个待同步` : '暂无待同步记录'}
          </span>
          <button type="button" onClick={() => void refreshList()} disabled={listLoading}>
            <RefreshCw size={15} className={listLoading ? styles.spin : ''} aria-hidden="true" />
            刷新
          </button>
        </div>
      </header>

      {!isLocalWorkbench() && (
        <div className={styles.modeNotice}>
          <WifiOff size={15} aria-hidden="true" />
          当前是云端工作台；本页仍通过浏览器直连 127.0.0.1 的一龙 PC 节点。
        </div>
      )}
      {error && <div className={styles.errorNotice}>{error}</div>}
      {!error && detailError && <div className={styles.errorNotice}>{detailError}</div>}
      {!error && !detailError && listStale && (
        <div className={styles.errorNotice}>本机节点连接暂时中断，当前显示上次成功读取的任务列表。</div>
      )}
      {notice && <div className={styles.successNotice}>{notice}</div>}

      <div className={styles.workspace}>
        <aside className={styles.sidebar}>
          <LocalTaskCreateForm busy={createBusy} onCreate={handleCreate} />
          <div className={styles.listHeading}>
            <strong>最近任务</strong>
            <span>{tasks.length}</span>
          </div>
          <div className={styles.taskList}>
            {tasks.map((task) => (
              <TaskListItem
                key={task.id}
                task={task}
                selected={task.id === selectedId}
                onSelect={() => setSelectedId(task.id)}
              />
            ))}
            {listLoading && !tasks.length && <p className={styles.listEmpty}>正在读取本机 journal…</p>}
            {!listLoading && !tasks.length && <p className={styles.listEmpty}>还没有本机任务，从上方启动第一个。</p>}
          </div>
        </aside>
        <LocalTaskDetailPanel
          detail={detail}
          loading={detailLoading}
          actionKey={actionKey}
          onCancel={handleCancel}
          onDecision={handleDecision}
        />
      </div>
    </div>
  )
}

function TaskListItem({
  task,
  selected,
  onSelect,
}: {
  task: LocalTaskRecord
  selected: boolean
  onSelect: () => void
}) {
  const status = localTaskStatus(task.status)
  return (
    <button className={styles.taskItem} data-selected={selected} type="button" onClick={onSelect}>
      <div>
        <strong>{readableTaskTitle(task.prompt)}</strong>
        <span>{task.workspace_path || '本机工作目录'}</span>
      </div>
      <div className={styles.taskItemMeta}>
        <em data-tone={status.tone}>{status.label}</em>
        <span>{syncStateLabel(task.sync_state)}</span>
        <span>{new Intl.NumberFormat('zh-CN').format(task.token_usage.total_tokens)} tokens</span>
      </div>
    </button>
  )
}

function errorMessage(error: unknown, fallback: string): string {
  if (error instanceof DOMException && error.name === 'AbortError') return '本机节点请求超时，请确认节点仍在运行。'
  if (error instanceof Error && error.message.trim()) return error.message
  return fallback
}
