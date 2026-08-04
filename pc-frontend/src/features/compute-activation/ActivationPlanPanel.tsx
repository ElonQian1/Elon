import { useCallback, useEffect, useState } from 'react'
import { CircleCheck, LoaderCircle, Network, RefreshCw, ShieldAlert } from 'lucide-react'
import { type ComputeActivationEvidenceRequest } from '../compute-supply/computeActivationApi'
import ApplyActivationPlanDialog from './ApplyActivationPlanDialog'
import PrepareActivationPlanDialog from './PrepareActivationPlanDialog'
import {
  computeActivationAdminApi,
  type ComputeActivationApplication,
  type ComputeActivationPlan,
  type ComputeActivationPlanPreflight,
  type PrepareActivationPlanBody,
} from './computeActivationAdminApi'
import styles from './ActivationPlanPanel.module.css'

interface Props { request: ComputeActivationEvidenceRequest; onChanged: (message: string) => Promise<void> }

export default function ActivationPlanPanel({ request, onChanged }: Props) {
  const [plan, setPlan] = useState<ComputeActivationPlan | null>(null)
  const [preflight, setPreflight] = useState<ComputeActivationPlanPreflight | null>(null)
  const [application, setApplication] = useState<ComputeActivationApplication | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [prepareOpen, setPrepareOpen] = useState(false)
  const [applyOpen, setApplyOpen] = useState(false)
  const [error, setError] = useState('')

  const load = useCallback(async () => {
    setLoading(true); setError('')
    try {
      const planResponse = await computeActivationAdminApi.plan(request.request_id)
      const nextPlan = planResponse.activation_plan
      setPlan(nextPlan)
      setPreflight(nextPlan ? await computeActivationAdminApi.planPreflight(request.request_id) : null)
      setApplication((await computeActivationAdminApi.application(request.request_id)).activation_application)
    } catch (reason) { setError(messageOf(reason, '激活计划读取失败')) } finally { setLoading(false) }
  }, [request.request_id])

  useEffect(() => { void load() }, [load])

  async function prepare(body: PrepareActivationPlanBody) {
    if (busy) return
    setBusy(true); setError('')
    try { await computeActivationAdminApi.preparePlan(request.request_id, body); setPrepareOpen(false); await load() }
    catch (reason) { setError(messageOf(reason, '激活计划准备失败')) } finally { setBusy(false) }
  }

  async function apply() {
    if (!plan || !preflight?.ready_for_apply || busy) return
    setBusy(true); setError('')
    try {
      await computeActivationAdminApi.applyPlan(request.request_id, `activation-apply:${plan.plan_id}:${plan.plan_digest.slice(0, 12)}`, plan.plan_digest)
      setApplyOpen(false); await load(); await onChanged('Provider 与 Pool 已按不可变计划激活，尚未发布 Offer。')
    } catch (reason) { setError(messageOf(reason, '激活计划应用失败')) } finally { setBusy(false) }
  }

  if (!['approved', 'activated'].includes(request.status) && !plan) return null

  return <section className={styles.panel}>
    <header><div><Network size={17} /><div><h3>受控激活计划</h3><span>批准、准备和应用是三个独立动作</span></div></div><button type="button" onClick={() => void load()} disabled={loading} title="刷新计划"><RefreshCw size={15} className={loading ? styles.spinning : ''} /></button></header>
    {error && !prepareOpen && !applyOpen && <div className={styles.error}>{error}</div>}
    {!plan && request.status === 'approved' && <div className={styles.empty}><p>尚未固定目标 Provider revision、路由引用和已验证硬件摘要。</p><button type="button" onClick={() => { setError(''); setPrepareOpen(true) }}>准备激活计划</button></div>}
    {plan && <>
      <div className={styles.planFacts}><div><span>计划状态</span><strong>{plan.status}</strong></div><div><span>目标版本</span><strong>{plan.target_provider_policy_revision}</strong></div><div><span>信任层</span><strong>{plan.target_provider.trust_tier}</strong></div><div><span>传输协议</span><strong>{plan.target_provider.endpoint?.transport ?? '无'}</strong></div></div>
      <div className={styles.digest}><span>计划摘要</span><code>{plan.plan_digest}</code></div>
      {preflight && <div className={preflight.ready_for_apply ? styles.ready : styles.blocked}>{preflight.ready_for_apply ? <CircleCheck size={17} /> : <ShieldAlert size={17} />}<div><strong>{preflight.ready_for_apply ? '计划可应用' : `${preflight.blockers.length} 项阻断`}</strong><span>{preflight.blockers.length ? preflight.blockers.map(blockerLabel).join('、') : '该快照不是授权；应用时服务端会重新核对'}</span></div></div>}
      {plan.status === 'prepared' && <div className={styles.actions}><span>应用后才会把 Provider 与 Pool 原子切换为 active。</span><button type="button" disabled={!preflight?.ready_for_apply || loading} onClick={() => { setError(''); setApplyOpen(true) }}>应用并激活</button></div>}
    </>}
    {application && <div className={styles.receipt}><CircleCheck size={17} /><div><strong>内部激活已提交</strong><span>{formatTime(application.applied_at)} · Offer 未发布</span><code>{application.application_digest}</code></div></div>}
    {loading && !plan && <div className={styles.loading}><LoaderCircle size={17} className={styles.spinning} />读取计划</div>}
    {prepareOpen && <PrepareActivationPlanDialog request={request} busy={busy} error={error} onClose={() => setPrepareOpen(false)} onSubmit={prepare} />}
    {applyOpen && plan && preflight && <ApplyActivationPlanDialog plan={plan} preflight={preflight} busy={busy} error={error} onClose={() => setApplyOpen(false)} onSubmit={apply} />}
  </section>
}

function blockerLabel(value: string) { return ({ plan_not_prepared: '计划非 prepared', request_not_approved: '申请不再批准', request_digest_changed: '申请摘要变化', request_binding_changed: '申请绑定变化', provider_version_changed: 'Provider 版本变化', provider_not_registering: 'Provider 非登记状态', target_provider_identity_changed: '目标身份变化', target_provider_revision_invalid: '目标版本无效', target_provider_not_ready: '目标合同未就绪', pool_provider_changed: 'Pool 归属变化', pool_version_changed: 'Pool 版本变化', pool_not_registering: 'Pool 非登记状态', ledger_audit_unhealthy: '账本审计异常', ledger_audit_changed: '账本摘要变化' } as Record<string, string>)[value] ?? value }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
