import { useCallback, useEffect, useMemo, useState } from 'react'
import { CircleCheck, FileCheck2, RefreshCw, ShieldAlert, ShieldCheck, TriangleAlert } from 'lucide-react'
import { useAuthStore } from '../../store/auth'
import { type ComputeActivationEvidenceRequest, type ComputeActivationPreflightReport } from '../compute-supply/computeActivationApi'
import ActivationReviewDialog from './ActivationReviewDialog'
import {
  computeActivationAdminApi,
  type ActivationRequestStatus,
  type ActivationReviewDecision,
} from './computeActivationAdminApi'
import styles from './ComputeActivationAdminPage.module.css'

const FILTERS: Array<{ value: ActivationRequestStatus; label: string }> = [
  { value: 'submitted', label: '待审核' }, { value: 'approved', label: '已批准' },
  { value: 'changes_requested', label: '需补充' }, { value: 'activated', label: '已激活' },
  { value: 'rejected', label: '已拒绝' }, { value: 'superseded', label: '已废止' },
  { value: 'canceled', label: '已取消' },
]

export default function ComputeActivationAdminPage() {
  const user = useAuthStore((state) => state.user)
  const isAdmin = user?.role === 'admin' || user?.role === 'owner'
  const [status, setStatus] = useState<ActivationRequestStatus>('submitted')
  const [requests, setRequests] = useState<ComputeActivationEvidenceRequest[]>([])
  const [selectedId, setSelectedId] = useState('')
  const [preflight, setPreflight] = useState<ComputeActivationPreflightReport | null>(null)
  const [loading, setLoading] = useState(false)
  const [reviewing, setReviewing] = useState(false)
  const [reviewOpen, setReviewOpen] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const selected = useMemo(() => requests.find((request) => request.request_id === selectedId) ?? null, [requests, selectedId])

  const load = useCallback(async () => {
    if (!isAdmin) return
    setLoading(true); setError('')
    try {
      const response = await computeActivationAdminApi.list(status)
      setRequests(response.activation_evidence_requests)
      setSelectedId((current) => response.activation_evidence_requests.some((item) => item.request_id === current) ? current : response.activation_evidence_requests[0]?.request_id ?? '')
    } catch (reason) { setError(messageOf(reason, '激活证据审核队列读取失败')) } finally { setLoading(false) }
  }, [isAdmin, status])

  useEffect(() => { void load() }, [load])
  useEffect(() => {
    setPreflight(null)
    if (!isAdmin || !selectedId) return
    let current = true
    void computeActivationAdminApi.preflight(selectedId).then((report) => { if (current) setPreflight(report) }).catch((reason) => { if (current) setError(messageOf(reason, '激活预检读取失败')) })
    return () => { current = false }
  }, [isAdmin, selectedId])

  async function review(decision: ActivationReviewDecision, note: string | null) {
    if (!selected || selected.status !== 'submitted' || reviewing) return
    setReviewing(true); setError(''); setNotice('')
    try {
      await computeActivationAdminApi.review(selected.request_id, selected.request_digest, decision, note)
      setReviewOpen(false); setNotice(`申请已${reviewLabel(decision)}，未执行资源激活。`); await load()
    } catch (reason) { setError(messageOf(reason, '激活证据审核失败')) } finally { setReviewing(false) }
  }

  if (!isAdmin) return <main className={styles.denied}><ShieldCheck size={24} /><h1>需要平台管理员权限</h1><p>当前账号不能审核算力激活证据。</p></main>

  return <main className={styles.page}>
    <header className={styles.header}><div><span className={styles.eyebrow}>资源信任控制</span><h1>算力激活审核</h1><p>审核供给者证据；审核与实际激活保持分离。</p></div><button type="button" onClick={() => void load()} disabled={loading}><RefreshCw size={15} className={loading ? styles.spinning : ''} />刷新</button></header>
    {error && !reviewOpen && <div className={styles.alert} data-tone="error"><TriangleAlert size={15} />{error}</div>}
    {notice && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />{notice}</div>}
    <div className={styles.filters} role="tablist" aria-label="申请状态">{FILTERS.map((filter) => <button type="button" role="tab" aria-selected={filter.value === status} data-active={filter.value === status} key={filter.value} onClick={() => setStatus(filter.value)}>{filter.label}</button>)}</div>
    <section className={styles.workbench}>
      <aside className={styles.queue}><header><strong>申请队列</strong><span>{requests.length}</span></header>{requests.map((request) => <button type="button" key={request.request_id} data-active={request.request_id === selectedId} onClick={() => setSelectedId(request.request_id)}><FileCheck2 size={16} /><span><strong>{shortId(request.provider_id)}</strong><small>{shortId(request.pool_id)} · {formatTime(request.requested_at)}</small></span></button>)}{requests.length === 0 && !loading && <div className={styles.empty}>当前状态没有申请</div>}</aside>
      <div className={styles.detail}>{selected ? <>
        <header className={styles.requestHeader}><div><span>申请 ID</span><h2>{selected.request_id}</h2></div><span className={styles.status}>{statusLabel(selected.status)}</span></header>
        <div className={styles.facts}><div><span>Provider</span><strong>{shortId(selected.provider_id)}</strong></div><div><span>Pool</span><strong>{shortId(selected.pool_id)}</strong></div><div><span>Provider Revision</span><strong>{selected.expected_provider_policy_revision}</strong></div><div><span>Pool Epoch / Revision</span><strong>{selected.expected_capacity_epoch} / {selected.expected_pool_revision}</strong></div></div>
        <section className={styles.evidence}><h3>证据引用</h3><div><span>节点绑定</span><code>{selected.node_binding_ref}</code></div><div><span>ReadyCapability</span><code>{selected.ready_capability_digest}</code></div><div><span>路由证明</span><code>{selected.route_proof_digest}</code></div><div><span>硬件观测</span><code>{selected.hardware_observation_digest}</code></div><div><span>账本审计</span><code>{selected.ledger_audit_digest}</code></div></section>
        {selected.review_note && <section className={styles.note}><h3>审核说明</h3><p>{selected.review_note}</p></section>}
        {preflight && <section className={preflight.ready_for_activation ? styles.ready : styles.blocked}>{preflight.ready_for_activation ? <CircleCheck size={18} /> : <ShieldAlert size={18} />}<div><strong>{preflight.ready_for_activation ? '当前预检无阻断项' : `当前有 ${preflight.blockers.length} 项阻断`}</strong><span>{preflight.blockers.length ? preflight.blockers.map(blockerLabel).join('、') : '审核后仍需准备并应用不可变计划'}</span></div></section>}
        <footer className={styles.detailFooter}><span>审核只改变申请状态，不触发资源激活。</span>{selected.status === 'submitted' && <button type="button" onClick={() => { setError(''); setReviewOpen(true) }}><ShieldCheck size={15} />审核申请</button>}</footer>
      </> : <div className={styles.detailEmpty}><FileCheck2 size={24} /><h2>选择一份申请</h2></div>}</div>
    </section>
    {reviewOpen && selected && <ActivationReviewDialog request={selected} busy={reviewing} error={error} onClose={() => setReviewOpen(false)} onSubmit={review} />}
  </main>
}

function statusLabel(value: string) { return ({ submitted: '待审核', changes_requested: '需补充', approved: '已批准', activated: '已激活', rejected: '已拒绝', canceled: '已取消', superseded: '已废止' } as Record<string, string>)[value] ?? value }
function blockerLabel(value: string) { return ({ request_not_approved: '申请尚未批准', provider_ownership_changed: 'Provider 所有权变化', provider_version_changed: 'Provider 版本变化', provider_not_registering: 'Provider 非登记状态', provider_routing_missing: '缺少路由', verified_hardware_missing: '缺少已验证硬件', verified_at_missing: '缺少验证时间', provider_trust_tier_self_declared: '仍为自我声明信任层', provider_regions_missing: '缺少服务区域', pool_provider_changed: 'Pool 归属变化', pool_version_changed: 'Pool 版本变化', pool_not_registering: 'Pool 非登记状态', ledger_audit_unhealthy: '账本审计异常', ledger_audit_changed: '账本审计摘要变化' } as Record<string, string>)[value] ?? value }
function reviewLabel(value: ActivationReviewDecision) { return ({ approved: '批准', changes_requested: '退回补充', rejected: '拒绝' } as Record<string, string>)[value] }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
