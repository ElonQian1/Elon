import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  ChartNoAxesCombined,
  CircleCheck,
  FileClock,
  Plus,
  RefreshCw,
  ShieldCheck,
  TriangleAlert,
} from 'lucide-react'
import { useAuthStore } from '../../store/auth'
import ReferenceCurveActionDialog from './ReferenceCurveActionDialog'
import ReferenceCurveSubmitDialog from './ReferenceCurveSubmitDialog'
import {
  computeReferenceCurveApi,
  type ReferenceCurveBatchDetail,
  type ReferenceCurveBatchReceipt,
  type ReferenceCurveBatchStatus,
  type ReferenceCurvePreflightReport,
  type ReferenceCurveReviewDecision,
  type SubmitReferenceCurveBody,
} from './computeReferenceCurveApi'
import styles from './ComputeReferenceCurvePage.module.css'

const FILTERS: Array<{ value: ReferenceCurveBatchStatus; label: string }> = [
  { value: 'submitted', label: '待复核' },
  { value: 'approved', label: '已批准' },
  { value: 'applied', label: '已应用' },
  { value: 'changes_requested', label: '需补充' },
  { value: 'rejected', label: '已拒绝' },
]

export default function ComputeReferenceCurvePage() {
  const user = useAuthStore((state) => state.user)
  const isAdmin = user?.role === 'admin' || user?.role === 'owner'
  const [status, setStatus] = useState<ReferenceCurveBatchStatus>('submitted')
  const [batches, setBatches] = useState<ReferenceCurveBatchReceipt[]>([])
  const [selectedId, setSelectedId] = useState('')
  const [detail, setDetail] = useState<ReferenceCurveBatchDetail | null>(null)
  const [preflight, setPreflight] = useState<ReferenceCurvePreflightReport | null>(null)
  const [submitOpen, setSubmitOpen] = useState(false)
  const [action, setAction] = useState<'review' | 'apply' | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')

  const selectedBatch = useMemo(
    () => batches.find((batch) => batch.batch_id === selectedId) ?? null,
    [batches, selectedId],
  )

  const loadForStatus = useCallback(async (
    targetStatus: ReferenceCurveBatchStatus,
    preferredBatchId = '',
  ) => {
    if (!isAdmin) return
    setLoading(true); setError('')
    try {
      const response = await computeReferenceCurveApi.list(targetStatus)
      setBatches(response)
      setSelectedId((current) => {
        if (preferredBatchId && response.some((batch) => batch.batch_id === preferredBatchId)) {
          return preferredBatchId
        }
        return response.some((batch) => batch.batch_id === current)
          ? current
          : response[0]?.batch_id ?? ''
      })
    } catch (reason) { setError(messageOf(reason, '参考价格批次读取失败')) } finally { setLoading(false) }
  }, [isAdmin])

  const load = useCallback(
    () => loadForStatus(status),
    [loadForStatus, status],
  )

  const loadDetail = useCallback(async (batchId: string) => {
    setError(''); setPreflight(null)
    try {
      const [nextDetail, nextPreflight] = await Promise.all([
        computeReferenceCurveApi.get(batchId),
        computeReferenceCurveApi.preflight(batchId),
      ])
      setDetail(nextDetail); setPreflight(nextPreflight)
    } catch (reason) { setDetail(null); setError(messageOf(reason, '参考价格批次详情读取失败')) }
  }, [])

  useEffect(() => { void load() }, [load])
  useEffect(() => {
    if (!selectedId) { setDetail(null); setPreflight(null); return }
    void loadDetail(selectedId)
  }, [loadDetail, selectedId])

  async function submit(body: SubmitReferenceCurveBody) {
    setBusy(true); setError(''); setNotice('')
    try {
      const receipt = await computeReferenceCurveApi.submit(body)
      setSubmitOpen(false); setStatus('submitted'); setNotice('参考价格批次已提交，尚未生成 Price Snapshot。')
      await loadForStatus('submitted', receipt.batch_id)
    } catch (reason) { setError(messageOf(reason, '参考价格批次提交失败')); throw reason } finally { setBusy(false) }
  }

  async function review(decision: ReferenceCurveReviewDecision, note: string | null) {
    if (!detail) return
    setBusy(true); setError(''); setNotice('')
    try {
      await computeReferenceCurveApi.review(detail, decision, note)
      setAction(null); setNotice(decision === 'approved' ? '批次已独立批准，尚未登记 Price Snapshot。' : '复核决定已保存。')
      setStatus(decision); await loadForStatus(decision, detail.batch.batch_id); await loadDetail(detail.batch.batch_id)
    } catch (reason) { setError(messageOf(reason, '批次复核失败')); throw reason } finally { setBusy(false) }
  }

  async function apply(note: string) {
    if (!detail) return
    setBusy(true); setError(''); setNotice('')
    try {
      const receipt = await computeReferenceCurveApi.apply(detail, note)
      setAction(null); setStatus('applied'); setNotice(`已原子登记 ${receipt.bindings.length} 个 Price Snapshot。`)
      await loadForStatus('applied', detail.batch.batch_id); await loadDetail(detail.batch.batch_id)
    } catch (reason) { setError(messageOf(reason, '参考价格应用失败')); throw reason } finally { setBusy(false) }
  }

  if (!isAdmin) return <main className={styles.denied}><ShieldCheck size={24} /><h1>需要平台管理员权限</h1><p>当前账号不能管理平台参考价格。</p></main>

  return <main className={styles.page}>
    <header className={styles.pageHeader}><div><span>报价治理控制</span><h1>平台参考价格</h1><p>提交、独立复核并原子登记 Offer 绑定的回退报价。</p></div><div><button type="button" onClick={() => void load()} disabled={loading}><RefreshCw size={15} className={loading ? styles.spinning : ''} />刷新</button><button type="button" className={styles.primary} onClick={() => { setError(''); setSubmitOpen(true) }}><Plus size={15} />新建批次</button></div></header>
    {error && !submitOpen && !action && <div className={styles.alert} data-tone="error"><TriangleAlert size={15} />{error}</div>}
    {notice && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />{notice}</div>}
    <div className={styles.filters} role="tablist" aria-label="批次状态">{FILTERS.map((filter) => <button type="button" role="tab" aria-selected={filter.value === status} data-active={filter.value === status} key={filter.value} onClick={() => { setStatus(filter.value); setDetail(null); setPreflight(null) }}>{filter.label}</button>)}</div>
    <section className={styles.workbench}>
      <aside className={styles.queue}><header><strong>{statusLabel(status)}</strong><span>{batches.length}</span></header>{batches.map((batch) => <button type="button" key={batch.batch_id} data-active={batch.batch_id === selectedId} onClick={() => setSelectedId(batch.batch_id)}><FileClock size={16} /><span><strong>{batch.curve_id}</strong><small>v{batch.curve_version} · {batch.entries.length} 条 · {formatTime(batch.updated_at)}</small></span></button>)}{!loading && !batches.length && <div className={styles.empty}>当前状态没有批次</div>}</aside>
      <div className={styles.detail}>{detail ? <BatchDetail detail={detail} preflight={preflight} onReview={() => setAction('review')} onApply={() => setAction('apply')} /> : selectedBatch ? <div className={styles.empty}>正在读取批次详情</div> : <div className={styles.empty}><ChartNoAxesCombined size={24} /><h2>选择一个价格批次</h2></div>}</div>
    </section>
    {submitOpen && <ReferenceCurveSubmitDialog busy={busy} error={error} onClose={() => setSubmitOpen(false)} onSubmit={submit} />}
    {action && detail && <ReferenceCurveActionDialog action={action} detail={detail} busy={busy} error={error} onClose={() => setAction(null)} onReview={review} onApply={apply} />}
  </main>
}

function BatchDetail({ detail, preflight, onReview, onApply }: { detail: ReferenceCurveBatchDetail; preflight: ReferenceCurvePreflightReport | null; onReview: () => void; onApply: () => void }) {
  const batch = detail.batch
  return <><header className={styles.batchHeader}><div><span>批次 ID</span><h2>{batch.batch_id}</h2></div><b>{statusLabel(batch.status)}</b></header>
    <div className={styles.facts}><div><span>曲线</span><strong>{batch.curve_id}</strong></div><div><span>版本</span><strong>{batch.curve_version}</strong></div><div><span>条目</span><strong>{batch.entries.length}</strong></div><div><span>提交人</span><strong>{shortId(batch.submitted_by_admin_user_id)}</strong></div></div>
    {preflight && <section className={preflight.blockers.length ? styles.blocked : styles.ready}><ShieldCheck size={18} /><div><strong>{preflight.admin_review_allowed ? '当前账号可独立复核' : preflight.admin_apply_allowed ? '当前账号可应用' : '当前操作受治理门卫限制'}</strong><span>{preflight.blockers.length ? preflight.blockers.map(blockerLabel).join('、') : '批次摘要与当前回执一致'}</span></div></section>}
    <section className={styles.digestBlock}><span>批次摘要</span><code>{batch.batch_digest}</code><span>材料摘要</span><code>{batch.batch_material_digest}</code></section>
    <section className={styles.batchEntries}><header><strong>Offer 交付窗口</strong><span>{batch.entries.length}</span></header>{batch.entries.map((entry) => <article key={entry.entry_id}><div><strong>{shortId(entry.offer_id)}</strong><span>Offer v{entry.offer_version} · 序号 {entry.ordinal}</span></div><code>{entry.entry_digest}</code></article>)}</section>
    {detail.review && <section className={styles.receipt}><CircleCheck size={17} /><div><strong>复核：{reviewLabel(detail.review.decision)}</strong><span>{shortId(detail.review.reviewed_by_admin_user_id)} · {formatTime(detail.review.reviewed_at)}</span><code>{detail.review.review_digest}</code></div></section>}
    {detail.application && <section className={styles.receipt}><CircleCheck size={17} /><div><strong>已登记 {detail.application.bindings.length} 个 Price Snapshot</strong><span>{formatTime(detail.application.applied_at)}</span><code>{detail.application.application_digest}</code></div></section>}
    <footer className={styles.actions}><span>审核不生成报价；应用只登记快照，不预留容量或移动资金。</span><div>{preflight?.admin_review_allowed && <button type="button" onClick={onReview}>独立复核</button>}{preflight?.admin_apply_allowed && <button type="button" className={styles.primary} onClick={onApply}>应用批次</button>}</div></footer>
  </>
}

function statusLabel(value: string) { return ({ submitted: '待复核', approved: '已批准', changes_requested: '需补充', rejected: '已拒绝', applied: '已应用' } as Record<string, string>)[value] ?? value }
function reviewLabel(value: string) { return ({ approved: '批准', changes_requested: '退回补充', rejected: '拒绝' } as Record<string, string>)[value] ?? value }
function blockerLabel(value: string) { return ({ current_admin_cannot_review_own_submission: '提交人不能复核自己的批次', changes_requested_requires_new_batch: '需修正材料并提交新批次', reference_curve_batch_rejected: '批次已拒绝', reference_curve_batch_already_applied: '批次已经应用' } as Record<string, string>)[value] ?? value }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function messageOf(reason: unknown, fallback: string) { return reason instanceof Error && reason.message ? reason.message : fallback }
