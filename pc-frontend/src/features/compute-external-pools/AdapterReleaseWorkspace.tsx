import { useCallback, useEffect, useMemo, useState } from 'react'
import { CircleCheck, PackageCheck, Plus, RefreshCw, TriangleAlert } from 'lucide-react'
import AdapterReleaseActionDialog from './AdapterReleaseActionDialog'
import AdapterReleaseSubmitDialog from './AdapterReleaseSubmitDialog'
import {
  externalPoolApi,
  type GovernanceDecision,
  type ReleaseDetail,
  type ReleasePreflight,
  type ReleaseStatus,
  type SubmitReleaseBody,
} from './externalPoolApi'
import styles from './ComputeExternalPoolsPage.module.css'

const FILTERS: Array<{ value: ReleaseStatus; label: string }> = [
  { value: 'submitted', label: '待复核' }, { value: 'approved', label: '已批准' },
  { value: 'staged', label: '已暂存' }, { value: 'changes_requested', label: '需补充' },
  { value: 'rejected', label: '已拒绝' },
]

export default function AdapterReleaseWorkspace() {
  const [status, setStatus] = useState<ReleaseStatus>('submitted')
  const [items, setItems] = useState<ReleaseDetail[]>([])
  const [selectedId, setSelectedId] = useState('')
  const [detail, setDetail] = useState<ReleaseDetail | null>(null)
  const [preflight, setPreflight] = useState<ReleasePreflight | null>(null)
  const [submitOpen, setSubmitOpen] = useState(false)
  const [action, setAction] = useState<'review' | 'stage' | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const selected = useMemo(() => items.find((item) => item.request.request_id === selectedId) ?? null, [items, selectedId])

  const loadForStatus = useCallback(async (target: ReleaseStatus, preferred = '') => {
    setLoading(true); setError('')
    try {
      const response = await externalPoolApi.listReleases(target)
      setItems(response)
      setSelectedId((current) => preferred && response.some((item) => item.request.request_id === preferred)
        ? preferred
        : response.some((item) => item.request.request_id === current) ? current : response[0]?.request.request_id ?? '')
    } catch (reason) { setError(messageOf(reason, 'Adapter release 列表读取失败')) } finally { setLoading(false) }
  }, [])
  const load = useCallback(() => loadForStatus(status), [loadForStatus, status])
  const loadDetail = useCallback(async (id: string) => {
    setError(''); setPreflight(null)
    try {
      const [nextDetail, nextPreflight] = await Promise.all([externalPoolApi.getRelease(id), externalPoolApi.preflightRelease(id)])
      setDetail(nextDetail); setPreflight(nextPreflight)
    } catch (reason) { setDetail(null); setError(messageOf(reason, 'Adapter release 详情读取失败')) }
  }, [])
  useEffect(() => { void load() }, [load])
  useEffect(() => { if (selectedId) void loadDetail(selectedId); else { setDetail(null); setPreflight(null) } }, [loadDetail, selectedId])

  async function submit(body: SubmitReleaseBody) {
    setBusy(true); setError(''); setNotice('')
    try {
      const receipt = await externalPoolApi.submitRelease(body)
      setSubmitOpen(false); setStatus('submitted'); setNotice('候选 release 已提交；工件尚未下载、验签或加载。')
      await loadForStatus('submitted', receipt.request_id)
    } catch (reason) { setError(messageOf(reason, 'Adapter release 提交失败')); throw reason } finally { setBusy(false) }
  }

  async function review(decision: GovernanceDecision, note: string | null) {
    if (!detail) return
    setBusy(true); setError(''); setNotice('')
    try {
      await externalPoolApi.reviewRelease(detail, decision, note)
      setAction(null); setStatus(decision); setNotice(decision === 'approved' ? '独立复核已批准；候选 release 仍未暂存。' : '复核决定已保存。')
      await loadForStatus(decision, detail.request.request_id); await loadDetail(detail.request.request_id)
    } catch (reason) { setError(messageOf(reason, 'Adapter release 复核失败')); throw reason } finally { setBusy(false) }
  }

  async function stage(note: string) {
    if (!detail) return
    setBusy(true); setError(''); setNotice('')
    try {
      await externalPoolApi.stageRelease(detail, note)
      setAction(null); setStatus('staged'); setNotice('已形成 staged admission；没有生成 Adapter registry、credential 或 route。')
      await loadForStatus('staged', detail.request.request_id); await loadDetail(detail.request.request_id)
    } catch (reason) { setError(messageOf(reason, 'Adapter release 暂存失败')); throw reason } finally { setBusy(false) }
  }

  return <section className={styles.workspace}>
    <header className={styles.workspaceHeader}><div><span>平台 Adapter 治理</span><h2>候选 release 暂存</h2></div><div><button type="button" onClick={() => void load()} disabled={loading}><RefreshCw size={14} className={loading ? styles.spinning : ''} />刷新</button><button type="button" className={styles.primary} onClick={() => { setError(''); setSubmitOpen(true) }}><Plus size={14} />新建 release</button></div></header>
    {error && !action && !submitOpen && <div className={styles.alert} data-tone="error"><TriangleAlert size={15} />{error}</div>}
    {notice && <div className={styles.alert}><CircleCheck size={15} />{notice}</div>}
    <div className={styles.filters}>{FILTERS.map((filter) => <button type="button" key={filter.value} data-active={status === filter.value} onClick={() => { setStatus(filter.value); setDetail(null); setPreflight(null) }}>{filter.label}</button>)}</div>
    <div className={styles.queueLayout}><aside className={styles.queue}><header><strong>{statusLabel(status)}</strong><span>{items.length}</span></header>{items.map((item) => <button type="button" key={item.request.request_id} data-active={item.request.request_id === selectedId} onClick={() => setSelectedId(item.request.request_id)}><PackageCheck size={16} /><span><strong>{item.request.adapter_id}</strong><small>{item.request.release_version} · {formatTime(item.request.updated_at)}</small></span></button>)}{!loading && !items.length && <div className={styles.empty}>当前状态没有 release</div>}</aside>
      <div className={styles.detail}>{detail ? <ReleaseDetailView detail={detail} preflight={preflight} onAction={setAction} /> : selected ? <div className={styles.empty}>正在读取详情</div> : <div className={styles.empty}><PackageCheck size={24} /><strong>选择一份 release request</strong></div>}</div></div>
    {submitOpen && <AdapterReleaseSubmitDialog busy={busy} error={error} onClose={() => setSubmitOpen(false)} onSubmit={submit} />}
    {action && detail && <AdapterReleaseActionDialog action={action} detail={detail} busy={busy} error={error} onClose={() => setAction(null)} onReview={review} onStage={stage} />}
  </section>
}

function ReleaseDetailView({ detail, preflight, onAction }: { detail: ReleaseDetail; preflight: ReleasePreflight | null; onAction: (value: 'review' | 'stage') => void }) {
  const request = detail.request
  return <><header className={styles.detailHeader}><div><span>Adapter release</span><h3>{request.adapter_id} · {request.release_version}</h3></div><b>{statusLabel(request.status)}</b></header>
    <div className={styles.facts}><div><span>提交人</span><strong>{shortId(request.submitted_by_admin_user_id)}</strong></div><div><span>请求 ID</span><strong>{shortId(request.request_id)}</strong></div><div><span>提交时间</span><strong>{formatTime(request.submitted_at)}</strong></div><div><span>效果</span><strong>staging only</strong></div></div>
    {preflight && <section className={preflight.blockers.length ? styles.blocked : styles.ready}><CircleCheck size={18} /><div><strong>{preflight.admin_review_allowed ? '当前管理员可独立复核' : preflight.admin_stage_allowed ? '当前管理员可暂存 release' : '当前流程无可执行写操作'}</strong><span>{preflight.blockers.length ? preflight.blockers.map(blockerLabel).join('、') : '当前摘要和账本状态一致'}</span></div></section>}
    <section className={styles.digest}><span>请求摘要</span><code>{request.request_digest}</code><span>材料摘要</span><code>{request.request_material_digest}</code></section>
    <section className={styles.secretBoundary}><PackageCheck size={17} /><div><strong>候选工件与 verifier 仍是未核验声明</strong><span>HTTP 回执不回显工件定位符或 verifier 详情；复核必须绑定当前双摘要。</span></div></section>
    {detail.review && <Receipt title={`复核：${decisionLabel(detail.review.decision)}`} meta={`${shortId(detail.review.reviewed_by_admin_user_id)} · ${formatTime(detail.review.reviewed_at)}`} digest={detail.review.review_digest} />}
    {detail.admission && <Receipt title="已形成 staged admission" meta={formatTime(detail.admission.applied_at)} digest={detail.admission.admission_digest} />}
    <footer className={styles.actions}><span>暂存不下载工件、不生成 credential、service actor 或 v213 route。</span><div>{preflight?.admin_review_allowed && <button type="button" onClick={() => onAction('review')}>独立复核</button>}{preflight?.admin_stage_allowed && <button type="button" className={styles.primary} onClick={() => onAction('stage')}>暂存 release</button>}</div></footer>
  </>
}

function Receipt({ title, meta, digest }: { title: string; meta: string; digest: string }) { return <section className={styles.receipt}><CircleCheck size={17} /><div><strong>{title}</strong><span>{meta}</span><code>{digest}</code></div></section> }
function statusLabel(value: string) { return ({ submitted: '待复核', approved: '已批准', changes_requested: '需补充', rejected: '已拒绝', staged: '已暂存' } as Record<string, string>)[value] ?? value }
function decisionLabel(value: string) { return ({ approved: '批准', changes_requested: '退回补充', rejected: '拒绝' } as Record<string, string>)[value] ?? value }
function blockerLabel(value: string) { return ({ current_admin_cannot_review_own_submission: '提交人不能复核自己的 release', changes_requested_requires_new_submission: '需修正后提交新 release', release_request_rejected: 'release 已拒绝', adapter_release_already_staged: 'release 已暂存' } as Record<string, string>)[value] ?? value }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function messageOf(reason: unknown, fallback: string) { return reason instanceof Error && reason.message ? reason.message : fallback }
