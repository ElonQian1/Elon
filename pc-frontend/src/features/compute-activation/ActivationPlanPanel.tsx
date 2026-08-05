import { useCallback, useEffect, useState } from 'react'
import { CircleCheck, LoaderCircle, Network, RefreshCw, ShieldAlert, ShieldX, UserRoundCheck } from 'lucide-react'
import { useAuthStore } from '../../store/auth'
import { type ComputeActivationEvidenceRequest } from '../compute-supply/computeActivationApi'
import ApplyActivationPlanDialog from './ApplyActivationPlanDialog'
import PrepareActivationPlanDialog from './PrepareActivationPlanDialog'
import ReviewActivationPlanDialog from './ReviewActivationPlanDialog'
import LifecycleReasonDialog from './LifecycleReasonDialog'
import {
  computeActivationAdminApi,
  type ComputeActivationApplication,
  type ComputeActivationPlan,
  type ComputeActivationPlanPreflight,
  type ComputeActivationPlanReview,
  type ComputeActivationQuarantine,
  type PrepareActivationPlanBody,
} from './computeActivationAdminApi'
import styles from './ActivationPlanPanel.module.css'

interface Props { request: ComputeActivationEvidenceRequest; onChanged: (message: string) => Promise<void> }

export default function ActivationPlanPanel({ request, onChanged }: Props) {
  const currentUserId = useAuthStore((state) => state.user?.id ?? '')
  const [plan, setPlan] = useState<ComputeActivationPlan | null>(null)
  const [review, setReview] = useState<ComputeActivationPlanReview | null>(null)
  const [preflight, setPreflight] = useState<ComputeActivationPlanPreflight | null>(null)
  const [application, setApplication] = useState<ComputeActivationApplication | null>(null)
  const [quarantine, setQuarantine] = useState<ComputeActivationQuarantine | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [prepareOpen, setPrepareOpen] = useState(false)
  const [applyOpen, setApplyOpen] = useState(false)
  const [reviewOpen, setReviewOpen] = useState(false)
  const [quarantineOpen, setQuarantineOpen] = useState(false)
  const [error, setError] = useState('')

  const load = useCallback(async () => {
    setLoading(true); setError('')
    try {
      const planResponse = await computeActivationAdminApi.plan(request.request_id)
      const nextPlan = planResponse.activation_plan
      setPlan(nextPlan)
      setReview(nextPlan ? (await computeActivationAdminApi.planReview(request.request_id)).activation_plan_review : null)
      setPreflight(nextPlan ? await computeActivationAdminApi.planPreflight(request.request_id) : null)
      const nextApplication = (await computeActivationAdminApi.application(request.request_id)).activation_application
      setApplication(nextApplication)
      setQuarantine(nextApplication ? (await computeActivationAdminApi.quarantine(request.request_id)).activation_quarantine : null)
    } catch (reason) { setError(messageOf(reason, '激活计划读取失败')) } finally { setLoading(false) }
  }, [request.request_id])

  useEffect(() => { void load() }, [load])

  async function prepare(body: PrepareActivationPlanBody) {
    if (busy) return
    setBusy(true); setError('')
    try { await computeActivationAdminApi.preparePlan(request.request_id, body); setPrepareOpen(false); await load() }
    catch (reason) { setError(messageOf(reason, '激活计划准备失败')) } finally { setBusy(false) }
  }

  async function confirmReview(note: string | null) {
    if (!plan || review || plan.prepared_by_user_id === currentUserId || busy) return
    setBusy(true); setError('')
    try {
      await computeActivationAdminApi.reviewPlan(request.request_id, `activation-plan-review:${plan.plan_digest}`, plan.plan_digest, note)
      setReviewOpen(false); await load(); await onChanged('第二人复核回执已固定；应用时服务端仍会重新审计。')
    } catch (reason) { setError(messageOf(reason, '激活计划复核失败')) } finally { setBusy(false) }
  }

  async function apply() {
    if (!plan || !preflight?.ready_for_apply || busy) return
    setBusy(true); setError('')
    try {
      await computeActivationAdminApi.applyPlan(request.request_id, `activation-apply:${plan.plan_digest}`, plan.plan_digest)
      setApplyOpen(false); await load(); await onChanged('Provider 与 Pool 已按不可变计划激活，尚未发布 Offer。')
    } catch (reason) { setError(messageOf(reason, '激活计划应用失败')) } finally { setBusy(false) }
  }

  async function isolate(reason: string) {
    if (!application || quarantine || busy) return
    setBusy(true); setError('')
    try {
      await computeActivationAdminApi.quarantineApplication(request.request_id, `activation-quarantine:${application.application_digest}`, application.application_digest, reason)
      setQuarantineOpen(false); await load(); await onChanged('Provider 与 Pool 已进入紧急隔离；原激活事实和回执保持不变。')
    } catch (failure) { setError(messageOf(failure, '激活结果隔离失败')) } finally { setBusy(false) }
  }

  if (!['approved', 'activated'].includes(request.status) && !plan) return null

  return <section className={styles.panel}>
    <header><div><Network size={17} /><div><h3>受控激活计划</h3><span>批准、准备、复核和应用是四个独立动作</span></div></div><button type="button" onClick={() => void load()} disabled={loading} title="刷新计划"><RefreshCw size={15} className={loading ? styles.spinning : ''} /></button></header>
    {error && !prepareOpen && !reviewOpen && !applyOpen && !quarantineOpen && <div className={styles.error}>{error}</div>}
    {!plan && request.status === 'approved' && <div className={styles.empty}><p>尚未固定目标 Provider revision、路由引用和已验证硬件摘要。</p><button type="button" onClick={() => { setError(''); setPrepareOpen(true) }}>准备激活计划</button></div>}
    {plan && <>
      <div className={styles.planFacts}><div><span>计划状态</span><strong>{plan.status}</strong></div><div><span>目标版本</span><strong>{plan.target_provider_policy_revision}</strong></div><div><span>信任层</span><strong>{plan.target_provider.trust_tier}</strong></div><div><span>传输协议</span><strong>{plan.target_provider.endpoint?.transport ?? '无'}</strong></div></div>
      <div className={styles.digest}><span>计划摘要</span><code>{plan.plan_digest}</code></div>
      {review ? <div className={styles.review}><UserRoundCheck size={17} /><div><strong>第二人复核已完成</strong><span>{formatTime(review.reviewed_at)} · 准备人 {shortId(review.prepared_by_user_id)} · 复核人 {shortId(review.reviewed_by_user_id)}</span>{review.review_note && <span>{review.review_note}</span>}<code>{review.review_digest}</code></div></div> : plan.status === 'prepared' && <div className={styles.reviewPending}><ShieldAlert size={17} /><div><strong>等待第二名管理员复核</strong><span>{plan.prepared_by_user_id === currentUserId ? '当前账号是计划准备人，必须换另一名管理员复核。' : '复核只固定计划证据，不会执行激活。'}</span></div>{plan.prepared_by_user_id !== currentUserId && <button type="button" onClick={() => { setError(''); setReviewOpen(true) }}>复核计划</button>}</div>}
      {preflight && <div className={preflight.ready_for_apply ? styles.ready : styles.blocked}>{preflight.ready_for_apply ? <CircleCheck size={17} /> : <ShieldAlert size={17} />}<div><strong>{preflight.ready_for_apply ? '计划可应用' : `${preflight.blockers.length} 项阻断`}</strong><span>{preflight.blockers.length ? preflight.blockers.map(blockerLabel).join('、') : '该快照不是授权；应用时服务端会重新核对'}</span></div></div>}
      {plan.status === 'prepared' && <div className={styles.actions}><span>应用后才会把 Provider 与 Pool 原子切换为 active。</span><button type="button" disabled={!preflight?.ready_for_apply || loading} onClick={() => { setError(''); setApplyOpen(true) }}>应用并激活</button></div>}
    </>}
    {application && <div className={styles.receipt}><CircleCheck size={17} /><div><strong>内部激活已提交</strong><span>{formatTime(application.applied_at)} · Offer 未发布</span><code>{application.application_digest}</code></div>{!quarantine && <button type="button" onClick={() => { setError(''); setQuarantineOpen(true) }}><ShieldX size={14} />紧急隔离</button>}</div>}
    {quarantine && <div className={styles.quarantine}><ShieldAlert size={17} /><div><strong>资源已隔离</strong><span>{formatTime(quarantine.quarantined_at)} · {quarantine.reason}</span><code>{quarantine.quarantine_digest}</code></div></div>}
    {loading && !plan && <div className={styles.loading}><LoaderCircle size={17} className={styles.spinning} />读取计划</div>}
    {prepareOpen && <PrepareActivationPlanDialog request={request} busy={busy} error={error} onClose={() => setPrepareOpen(false)} onSubmit={prepare} />}
    {reviewOpen && plan && <ReviewActivationPlanDialog plan={plan} busy={busy} error={error} onClose={() => setReviewOpen(false)} onSubmit={confirmReview} />}
    {applyOpen && plan && preflight && <ApplyActivationPlanDialog plan={plan} preflight={preflight} busy={busy} error={error} onClose={() => setApplyOpen(false)} onSubmit={apply} />}
    {quarantineOpen && application && <LifecycleReasonDialog title="紧急隔离已激活资源" description="该单向控制会把当前 Provider 与 Pool 切换为 quarantined，阻止新的候选选择；不会撤销既有合同、退款、关闭节点或删除原激活回执。" confirmLabel="确认隔离" busy={busy} error={error} onClose={() => setQuarantineOpen(false)} onSubmit={isolate} />}
  </section>
}

function blockerLabel(value: string) { return ({ plan_not_prepared: '计划非 prepared', request_not_approved: '申请不再批准', request_digest_changed: '申请摘要变化', request_binding_changed: '申请绑定变化', provider_version_changed: 'Provider 版本变化', provider_not_registering: 'Provider 非登记状态', target_provider_identity_changed: '目标身份变化', target_provider_revision_invalid: '目标版本无效', target_provider_not_ready: '目标合同未就绪', pool_provider_changed: 'Pool 归属变化', pool_version_changed: 'Pool 版本变化', pool_not_registering: 'Pool 非登记状态', ledger_audit_unhealthy: '账本审计异常', ledger_audit_changed: '账本摘要变化', plan_review_missing: '缺少第二人复核', plan_review_digest_changed: '复核摘要变化', plan_review_separation_invalid: '准备人与复核人未分离' } as Record<string, string>)[value] ?? value }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
