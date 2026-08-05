import { useCallback, useEffect, useState } from 'react'
import { CircleCheck, LoaderCircle, RefreshCw, RotateCcw, ShieldAlert, UserRoundCheck } from 'lucide-react'
import { useAuthStore } from '../../store/auth'
import ApplyActivationRecoveryDialog from './ApplyActivationRecoveryDialog'
import PrepareActivationRecoveryDialog from './PrepareActivationRecoveryDialog'
import ReviewActivationRecoveryDialog from './ReviewActivationRecoveryDialog'
import {
  computeActivationAdminApi,
  type ComputeActivationQuarantine,
  type ComputeActivationRecoveryApplication,
  type ComputeActivationRecoveryPlan,
  type ComputeActivationRecoveryPreflight,
  type ComputeActivationRecoveryReview,
  type PrepareActivationRecoveryPlanBody,
} from './computeActivationAdminApi'
import styles from './ActivationPlanPanel.module.css'

interface Props {
  requestId: string
  quarantine: ComputeActivationQuarantine
  onChanged: (message: string) => Promise<void>
}

export default function ActivationRecoveryPanel({ requestId, quarantine, onChanged }: Props) {
  const currentUserId = useAuthStore((state) => state.user?.id ?? '')
  const [plan, setPlan] = useState<ComputeActivationRecoveryPlan | null>(null)
  const [review, setReview] = useState<ComputeActivationRecoveryReview | null>(null)
  const [preflight, setPreflight] = useState<ComputeActivationRecoveryPreflight | null>(null)
  const [application, setApplication] = useState<ComputeActivationRecoveryApplication | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [prepareOpen, setPrepareOpen] = useState(false)
  const [reviewOpen, setReviewOpen] = useState(false)
  const [applyOpen, setApplyOpen] = useState(false)
  const [error, setError] = useState('')

  const load = useCallback(async () => {
    setLoading(true)
    setError('')
    try {
      const nextPlan = (await computeActivationAdminApi.recoveryPlan(requestId)).activation_recovery_plan
      setPlan(nextPlan)
      setReview(nextPlan ? (await computeActivationAdminApi.recoveryReview(requestId)).activation_recovery_review : null)
      setPreflight(nextPlan?.status === 'prepared' ? await computeActivationAdminApi.recoveryPreflight(requestId) : null)
      setApplication((await computeActivationAdminApi.recoveryApplication(requestId)).activation_recovery_application)
    } catch (reason) {
      setError(messageOf(reason, '隔离恢复状态读取失败'))
    } finally {
      setLoading(false)
    }
  }, [requestId])

  useEffect(() => { void load() }, [load])

  async function prepare(body: PrepareActivationRecoveryPlanBody) {
    if (busy) return
    setBusy(true)
    setError('')
    try {
      await computeActivationAdminApi.prepareRecoveryPlan(requestId, body)
      setPrepareOpen(false)
      await load()
      await onChanged('隔离恢复计划已固定；尚未恢复 Provider、Pool 或旧 Offer。')
    } catch (reason) {
      setError(messageOf(reason, '隔离恢复计划准备失败'))
    } finally {
      setBusy(false)
    }
  }

  async function confirmReview(note: string | null) {
    if (!plan || review || plan.prepared_by_user_id === currentUserId || busy) return
    setBusy(true)
    setError('')
    try {
      await computeActivationAdminApi.reviewRecoveryPlan(
        requestId,
        `activation-recovery-review:${plan.plan_digest}`,
        plan.plan_digest,
        note,
      )
      setReviewOpen(false)
      await load()
      await onChanged('第二名管理员已固定恢复复核回执；应用时仍会重新审计。')
    } catch (reason) {
      setError(messageOf(reason, '隔离恢复计划复核失败'))
    } finally {
      setBusy(false)
    }
  }

  async function apply() {
    if (!plan || !preflight?.ready_for_apply || busy) return
    setBusy(true)
    setError('')
    try {
      await computeActivationAdminApi.applyRecoveryPlan(
        requestId,
        `activation-recovery-apply:${plan.plan_digest}`,
        plan.plan_digest,
      )
      setApplyOpen(false)
      await load()
      await onChanged('Provider 与 Pool 已按恢复计划重新激活；旧 Offer 未恢复。')
    } catch (reason) {
      setError(messageOf(reason, '隔离恢复计划应用失败'))
    } finally {
      setBusy(false)
    }
  }

  return <section className={styles.recoverySection}>
    <header className={styles.recoveryHeader}>
      <div><RotateCcw size={17} /><div><h4>隔离恢复</h4><span>准备、第二人复核和应用彼此分离</span></div></div>
      <button type="button" onClick={() => void load()} disabled={loading} title="刷新恢复状态"><RefreshCw size={14} className={loading ? styles.spinning : ''} /></button>
    </header>
    {error && !prepareOpen && !reviewOpen && !applyOpen && <div className={styles.error}>{error}</div>}
    {!plan && !application && <div className={styles.empty}><p>修复路由或硬件证据后，可准备一份绑定当前隔离摘要的新 Provider 版本。</p><button type="button" onClick={() => { setError(''); setPrepareOpen(true) }}>准备恢复计划</button></div>}
    {plan && <>
      <div className={styles.planFacts}><div><span>恢复状态</span><strong>{plan.status}</strong></div><div><span>目标版本</span><strong>{plan.target_provider_policy_revision}</strong></div><div><span>信任层</span><strong>{plan.target_provider.trust_tier}</strong></div><div><span>证据引用</span><strong>{plan.evidence_refs.length}</strong></div></div>
      <div className={styles.digest}><span>恢复计划摘要</span><code>{plan.plan_digest}</code></div>
      {review ? <div className={styles.review}><UserRoundCheck size={17} /><div><strong>恢复计划已完成第二人复核</strong><span>{formatTime(review.reviewed_at)} · 准备人 {shortId(review.prepared_by_user_id)} · 复核人 {shortId(review.reviewed_by_user_id)}</span>{review.review_note && <span>{review.review_note}</span>}<code>{review.review_digest}</code></div></div> : plan.status === 'prepared' && <div className={styles.reviewPending}><ShieldAlert size={17} /><div><strong>等待第二名管理员复核</strong><span>{plan.prepared_by_user_id === currentUserId ? '当前账号是恢复计划准备人，请换另一名管理员。' : '复核只固定计划摘要，不会解除隔离。'}</span></div>{plan.prepared_by_user_id !== currentUserId && <button type="button" onClick={() => { setError(''); setReviewOpen(true) }}>复核恢复计划</button>}</div>}
      {preflight && <div className={preflight.ready_for_apply ? styles.ready : styles.blocked}>{preflight.ready_for_apply ? <CircleCheck size={17} /> : <ShieldAlert size={17} />}<div><strong>{preflight.ready_for_apply ? '恢复计划可应用' : `${preflight.blockers.length} 项阻断`}</strong><span>{preflight.blockers.length ? preflight.blockers.map(recoveryBlockerLabel).join('、') : '旧 active Offer 已清退；应用时服务端仍会重新核对'}</span></div></div>}
      {plan.status === 'prepared' && <div className={styles.actions}><span>当前 active Offer：{preflight?.active_offer_count ?? '读取中'}。恢复不会重新发布旧 Offer。</span><button type="button" disabled={!preflight?.ready_for_apply || loading} onClick={() => { setError(''); setApplyOpen(true) }}>应用恢复</button></div>}
    </>}
    {application && <div className={styles.recoveryReceipt}><CircleCheck size={17} /><div><strong>隔离恢复已应用</strong><span>{formatTime(application.applied_at)} · Provider revision {application.recovered_provider_policy_revision} · Offer 未恢复</span><code>{application.application_digest}</code></div></div>}
    {loading && !plan && !application && <div className={styles.loading}><LoaderCircle size={17} className={styles.spinning} />读取恢复状态</div>}
    {prepareOpen && <PrepareActivationRecoveryDialog quarantine={quarantine} busy={busy} error={error} onClose={() => setPrepareOpen(false)} onSubmit={prepare} />}
    {reviewOpen && plan && <ReviewActivationRecoveryDialog plan={plan} busy={busy} error={error} onClose={() => setReviewOpen(false)} onSubmit={confirmReview} />}
    {applyOpen && plan && preflight && <ApplyActivationRecoveryDialog plan={plan} preflight={preflight} busy={busy} error={error} onClose={() => setApplyOpen(false)} onSubmit={apply} />}
  </section>
}

function recoveryBlockerLabel(value: string) { return ({ recovery_plan_not_prepared: '恢复计划非 prepared', quarantine_digest_changed: '隔离摘要变化', quarantine_binding_changed: '隔离绑定变化', provider_version_changed: 'Provider 版本变化', provider_not_quarantined: 'Provider 不在隔离状态', target_provider_identity_changed: '目标身份变化', target_provider_revision_invalid: '目标版本无效', target_provider_not_ready: '目标合同未就绪', pool_provider_changed: 'Pool 归属变化', pool_version_changed: 'Pool 版本变化', pool_not_quarantined: 'Pool 不在隔离状态', active_offers_remaining: '仍有 active Offer', plan_review_missing: '缺少第二人复核', plan_review_digest_changed: '复核摘要变化', plan_review_separation_invalid: '准备人与复核人未分离' } as Record<string, string>)[value] ?? value }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
