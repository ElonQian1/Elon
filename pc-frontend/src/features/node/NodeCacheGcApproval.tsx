import { useCallback, useEffect, useState } from 'react'
import { Check, ListChecks, LoaderCircle, RotateCcw, X } from 'lucide-react'
import { formatBytes, formatDateTime } from './nodeHelpers'
import {
  approveCacheGcPlan,
  cacheGcStatusLabel,
  createCacheGcPlan,
  fetchLatestCacheGc,
  rejectCacheGcPlan,
  type NodeCacheGcRequest,
} from './nodeCacheGc'
import styles from './NodeCacheGcApproval.module.css'

const ACTIVE = new Set<NodeCacheGcRequest['status']>(['requested', 'approved', 'executing'])
const TERMINAL = new Set<NodeCacheGcRequest['status']>(['completed', 'partial', 'failed', 'rejected', 'expired'])

export default function NodeCacheGcApproval({ nodeId, recommended }: { nodeId: string; recommended: boolean }) {
  const [request, setRequest] = useState<NodeCacheGcRequest | null>(null)
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')

  const load = useCallback(async () => {
    try {
      setRequest(await fetchLatestCacheGc(nodeId))
      setError('')
    } catch (reason) {
      setError(messageOf(reason))
    } finally {
      setLoading(false)
    }
  }, [nodeId])

  useEffect(() => {
    load()
    const timer = window.setInterval(() => {
      if (request && ACTIVE.has(request.status)) load()
    }, 5000)
    return () => window.clearInterval(timer)
  }, [load, request])

  async function createPlan() {
    setBusy(true)
    setError('')
    try { setRequest(await createCacheGcPlan(nodeId)) }
    catch (reason) { setError(messageOf(reason)) }
    finally { setBusy(false) }
  }

  async function approve() {
    if (!request?.plan) return
    const digest = request.plan.plan_digest.slice(0, 12)
    if (!window.confirm(`批准摘要 ${digest} 的缓存回收计划？目标电脑会重新扫描；只要动作、锁或活动编译发生变化，就会拒绝执行。`)) return
    setBusy(true)
    setError('')
    try { setRequest(await approveCacheGcPlan(nodeId, request.plan)) }
    catch (reason) { setError(messageOf(reason)) }
    finally { setBusy(false) }
  }

  async function reject() {
    if (!request) return
    setBusy(true)
    setError('')
    try { setRequest(await rejectCacheGcPlan(nodeId, request.request_id)) }
    catch (reason) { setError(messageOf(reason)) }
    finally { setBusy(false) }
  }

  if (loading) return <div className={styles.loading}><LoaderCircle size={14} className={styles.spinning} />读取回收审批状态...</div>

  const canCreate = !request || TERMINAL.has(request.status)
  return (
    <div className={styles.panel}>
      <div className={styles.heading}>
        <div><ListChecks size={15} aria-hidden="true" /><strong>安全回收</strong></div>
        {request && <span data-status={request.status}>{cacheGcStatusLabel(request.status)}</span>}
      </div>

      {canCreate && (
        <div className={styles.start}>
          <p>{recommended ? '当前报告建议生成一次回收预演。' : '先在目标电脑生成只读预演，页面不会收到本机路径。'}</p>
          <button type="button" onClick={createPlan} disabled={busy}>
            <RotateCcw size={14} aria-hidden="true" />生成回收预演
          </button>
        </div>
      )}

      {request?.plan && <PlanSummary request={request} />}
      {request?.result && (
        <div className={styles.result}>
          <span>删除 {request.result.removed_action_count}/{request.result.approved_action_count} 项</span>
          <span>实际回收 {formatBytes(request.result.reclaimed_bytes)}</span>
          <span>完成于 {formatDateTime(request.updated_at)}</span>
        </div>
      )}
      {request?.failure_code && !request.result && (
        <p className={styles.error}>失败代码：{request.failure_code}</p>
      )}

      {request?.status === 'plan_ready' && request.plan && (
        <div className={styles.actions}>
          <button type="button" className={styles.approve} onClick={approve} disabled={busy}>
            <Check size={14} aria-hidden="true" />批准此摘要
          </button>
          <button type="button" onClick={reject} disabled={busy}>
            <X size={14} aria-hidden="true" />取消
          </button>
        </div>
      )}
      {request?.status === 'approved' && (
        <div className={styles.actions}><button type="button" onClick={reject} disabled={busy}><X size={14} />撤销审批</button></div>
      )}
      {error && <p className={styles.error} role="alert">{error}</p>}
      <p className={styles.boundary}>服务器只保存脱敏摘要；删除始终由目标电脑按本机锁和同一摘要执行。</p>
    </div>
  )
}

function PlanSummary({ request }: { request: NodeCacheGcRequest }) {
  const plan = request.plan!
  const reasons = plan.reasons.map((item) => `${reasonLabel(item.reason)} ${item.count}`).join(' · ')
  return (
    <div className={styles.plan}>
      <div><span>预计动作</span><strong>{plan.action_count} 项</strong></div>
      <div><span>预计回收</span><strong>{formatBytes(plan.reclaim_bytes)}</strong></div>
      <div><span>活动编译</span><strong>{plan.active_writer_count} 个</strong></div>
      <div><span>摘要</span><strong title={plan.plan_digest}>{plan.plan_digest.slice(0, 12)}</strong></div>
      {reasons && <p>{reasons}</p>}
    </div>
  )
}

function reasonLabel(reason: string) {
  return ({
    'orphaned-task-worktree': '旧任务',
    'missing-workspace-recovery': '缺失工作区',
    'retired-domain': '停用域',
    'retired-shared-alias': '旧共享别名',
    'old-toolchain-epoch': '旧工具链',
    'disk-watermark': '磁盘水位',
    'disk-watermark-lru': '磁盘 LRU',
    'forced-aged-cleanup': '强制老化',
  } as Record<string, string>)[reason] ?? reason
}

function messageOf(reason: unknown): string {
  return typeof reason === 'object' && reason !== null && 'message' in reason
    ? String(reason.message)
    : '缓存回收审批操作失败'
}
