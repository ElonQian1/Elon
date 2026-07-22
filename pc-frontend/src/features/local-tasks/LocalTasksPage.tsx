import { useCallback, useEffect, useState } from 'react'
import { HardDrive, RefreshCw, WifiOff } from 'lucide-react'
import { isLocalWorkbench } from '../../api/runtime'
import { safeNodeAdminUrl } from '../../lib/utils'
import { readableTaskTitle } from '../../lib/taskTitle'
import { ensureLocalFullAccessGrant } from '../conversation/localPcRuntime'
import LocalTaskCreateForm from './LocalTaskCreateForm'
import LocalTaskDetailPanel from './LocalTaskDetailPanel'
import LocalOperationsPanel from './LocalOperationsPanel'
import {
  cancelLocalTask,
  createLocalTask,
  decideLocalTaskApproval,
  getLocalTask,
  getGlobalPublishStatus,
  listLocalTasks,
  listSelfEvolution,
  pauseSelfEvolution,
  resumeSelfEvolution,
  reviewSelfEvolution,
} from './localTaskApi'
import { normalizeGlobalPublishStatus, normalizeSelfEvolutionQueue } from './localOperationsModel'
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
  LocalTaskContinuationInput,
  LocalTaskDetail,
  LocalTaskRecord,
  GlobalPublishStatus,
  SelfEvolutionQueue,
} from './types'
import styles from './LocalTasksPage.module.css'

const LIST_POLL_MS = 5_000
const DETAIL_POLL_MS = 1_600
const INITIAL_VISIBLE_TASKS = 20
const EMPTY_EVOLUTION: SelfEvolutionQueue = {
  items: [],
  gates: {
    foreground_task_ids: [], publish_active: false, publish_status: '', publish_waiter_count: 0,
    update_active: false, resource_pressure: false,
  },
}
const EMPTY_PUBLISH: GlobalPublishStatus = {
  waiters: [], waiterCount: 0, queuePolicy: 'fifo', coalescingKey: 'kind+sha', immutableReleaseSha: true,
  batchIdentity: 'batchId+sha', stateHealth: 'unavailable', batches: [],
}

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
  const [evolution, setEvolution] = useState<SelfEvolutionQueue>(EMPTY_EVOLUTION)
  const [publish, setPublish] = useState<GlobalPublishStatus>(EMPTY_PUBLISH)
  const [visibleTaskCount, setVisibleTaskCount] = useState(INITIAL_VISIBLE_TASKS)
  const visibleTasks = tasks.slice(0, visibleTaskCount)

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

  const refreshOperations = useCallback(async () => {
    const [evolutionResult, publishResult] = await Promise.allSettled([
      listSelfEvolution(),
      getGlobalPublishStatus(),
    ])
    if (evolutionResult.status === 'fulfilled') {
      setEvolution(normalizeSelfEvolutionQueue(evolutionResult.value))
    }
    if (publishResult.status === 'fulfilled') {
      setPublish(normalizeGlobalPublishStatus(publishResult.value))
    }
  }, [])

  useEffect(() => {
    void refreshList()
    const timer = window.setInterval(() => void refreshList(true), LIST_POLL_MS)
    return () => window.clearInterval(timer)
  }, [refreshList])

  useEffect(() => {
    void refreshOperations()
    const timer = window.setInterval(() => void refreshOperations(), LIST_POLL_MS)
    return () => window.clearInterval(timer)
  }, [refreshOperations])

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

  async function handleContinue(input: LocalTaskContinuationInput): Promise<boolean> {
    if (!detail || actionKey) return false
    const contract = detail.supervision.contract
    if (!contract || !['requirement', 'resume_original'].includes(contract.task_role)) {
      setError('当前任务没有可验证的监督合同，不能直接继续。')
      return false
    }
    setActionKey('continue')
    setError('')
    setNotice('')
    try {
      const rootTaskId = contract.root_task_id || detail.task.id
      const prompt = input.mode === 'resume'
        ? `Resolve elon.resume_context.v1 for parent_task_id=${detail.task.id} and root_task_id=${rootTaskId}.`
        : input.prompt.trim()
      const createInput: LocalTaskCreateInput = {
        project_id: detail.task.project_id,
        channel_id: detail.task.channel_id || undefined,
        conversation_id: continuationConversationId(input.mode),
        workspace_path: detail.task.workspace_path,
        prompt,
        runtime_permission: 'full_access',
        supervision: {
          protocol: 'elon.desktop_pc_supervision.v1',
          supervisor: 'codex_desktop',
          task_role: 'resume_original',
          parent_task_id: detail.task.id,
          root_task_id: rootTaskId,
          acceptance_criteria: input.mode === 'resume' ? [] : input.acceptance_criteria,
          improvement_policy: contract.improvement_policy === 'observe_only'
            ? 'observe_only'
            : contract.improvement_policy === 'after_task_only'
              ? 'after_task_only'
              : 'after_task_or_unblock',
        },
        contract_revision: input.mode === 'supersede' ? {
          schema: 'elon.supervision.contract_revision.v1',
          reason: input.reason.trim(),
        } : undefined,
      }
      await ensureLocalFullAccessGrant({
        adminUrl: safeNodeAdminUrl(),
        projectId: createInput.project_id,
        projectName: createInput.project_id,
        workspacePath: createInput.workspace_path,
        runtimePermission: createInput.runtime_permission,
        useLocalRouteA: true,
      })
      const response = await createLocalTask(createInput)
      const taskId = taskIdFromCreateResponse(response)
      setNotice(input.mode === 'resume'
        ? '已按最新有效合同继续原任务。'
        : '已保存需求修订收据，并按新的完整目标开始承接。')
      await refreshList(true)
      if (taskId) setSelectedId(taskId)
      return true
    } catch (err) {
      setError(errorMessage(err, input.mode === 'resume' ? '继续任务失败。' : '需求变更承接失败。'))
      return false
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

  async function handleEvolutionAction(
    logicalId: string,
    action: 'pause' | 'resume' | 'approve' | 'reject',
  ) {
    if (actionKey) return
    setActionKey(`evolution:${logicalId}:${action}`)
    setError('')
    try {
      if (action === 'pause') await pauseSelfEvolution(logicalId)
      else if (action === 'resume') await resumeSelfEvolution(logicalId)
      else await reviewSelfEvolution(logicalId, action)
      setNotice(action === 'pause' ? '自进化正在保存现场并让路。' : action === 'resume' ? '自进化已回到低优先队列。' : action === 'approve' ? '自进化审查已通过。' : '已退回自进化结果，等待下一代继续。')
      await refreshOperations()
      await refreshList(true)
    } catch (err) {
      setError(errorMessage(err, '自进化队列操作失败。'))
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

      <LocalOperationsPanel
        evolution={evolution}
        publish={publish}
        actionKey={actionKey}
        onAction={handleEvolutionAction}
      />

      <div className={styles.workspace}>
        <aside className={styles.sidebar}>
          <LocalTaskCreateForm busy={createBusy} onCreate={handleCreate} />
          <div className={styles.listHeading}>
            <strong>最近任务</strong>
            <span>{tasks.length}</span>
          </div>
          <div className={styles.taskList} data-testid="local-task-list">
            {visibleTasks.map((task) => (
              <TaskListItem
                key={task.id}
                task={task}
                selected={task.id === selectedId}
                onSelect={() => setSelectedId(task.id)}
              />
            ))}
            {visibleTaskCount < tasks.length && (
              <button
                className={styles.loadMore}
                data-testid="local-task-list-more"
                type="button"
                onClick={() => setVisibleTaskCount((count) => Math.min(tasks.length, count + INITIAL_VISIBLE_TASKS))}
              >
                再显示 {Math.min(INITIAL_VISIBLE_TASKS, tasks.length - visibleTaskCount)} 个任务
              </button>
            )}
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
          onContinue={handleContinue}
        />
      </div>
    </div>
  )
}

function continuationConversationId(mode: LocalTaskContinuationInput['mode']): string {
  if (typeof crypto.randomUUID === 'function') return `desktop-${mode}-${crypto.randomUUID()}`
  return `desktop-${mode}-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
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
    <button className={styles.taskItem} data-testid="local-task-row" data-task-id={task.id} data-selected={selected} type="button" onClick={onSelect}>
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
