import { useCallback, useEffect, useMemo, useState } from 'react'
import { CircleCheck, KeyRound, Network, Plus, RefreshCw, ShieldAlert, TriangleAlert } from 'lucide-react'
import OnboardingActionDialog from './OnboardingActionDialog'
import OnboardingSubmitDialog from './OnboardingSubmitDialog'
import {
  externalPoolApi,
  type GovernanceDecision,
  type OnboardingDetail,
  type OnboardingPreflight,
  type OnboardingStatus,
  type SubmitOnboardingBody,
} from './externalPoolApi'
import styles from './ComputeExternalPoolsPage.module.css'

const FILTERS: Array<{ value: OnboardingStatus; label: string }> = [
  { value: 'submitted', label: '待复核' }, { value: 'approved', label: '已批准' },
  { value: 'applied', label: '已登记' }, { value: 'changes_requested', label: '需补充' },
  { value: 'rejected', label: '已拒绝' }, { value: 'canceled', label: '已取消' },
]

export default function OnboardingWorkspace({ mode }: { mode: 'owner' | 'admin' }) {
  const admin = mode === 'admin'
  const [status, setStatus] = useState<OnboardingStatus>('submitted')
  const [items, setItems] = useState<OnboardingDetail[]>([])
  const [selectedId, setSelectedId] = useState('')
  const [detail, setDetail] = useState<OnboardingDetail | null>(null)
  const [preflight, setPreflight] = useState<OnboardingPreflight | null>(null)
  const [submitOpen, setSubmitOpen] = useState(false)
  const [action, setAction] = useState<'cancel' | 'review' | 'apply' | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')

  const selected = useMemo(() => items.find((item) => item.request.request_id === selectedId) ?? null, [items, selectedId])

  const loadForStatus = useCallback(async (target: OnboardingStatus, preferred = '') => {
    setLoading(true); setError('')
    try {
      const response = admin ? await externalPoolApi.listOnboardingAdmin(target) : await externalPoolApi.listMine(target)
      setItems(response)
      setSelectedId((current) => preferred && response.some((item) => item.request.request_id === preferred)
        ? preferred
        : response.some((item) => item.request.request_id === current) ? current : response[0]?.request.request_id ?? '')
    } catch (reason) { setError(messageOf(reason, '接入申请列表读取失败')) } finally { setLoading(false) }
  }, [admin])

  const load = useCallback(() => loadForStatus(status), [loadForStatus, status])
  const loadDetail = useCallback(async (id: string) => {
    setError(''); setPreflight(null)
    try {
      const [nextDetail, nextPreflight] = admin
        ? await Promise.all([externalPoolApi.getOnboardingAdmin(id), externalPoolApi.preflightOnboardingAdmin(id)])
        : await Promise.all([externalPoolApi.getMine(id), externalPoolApi.preflightMine(id)])
      setDetail(nextDetail); setPreflight(nextPreflight)
    } catch (reason) { setDetail(null); setError(messageOf(reason, '接入申请详情读取失败')) }
  }, [admin])

  useEffect(() => { void load() }, [load])
  useEffect(() => { if (selectedId) void loadDetail(selectedId); else { setDetail(null); setPreflight(null) } }, [loadDetail, selectedId])

  async function submit(body: SubmitOnboardingBody) {
    setBusy(true); setError(''); setNotice('')
    try {
      const receipt = await externalPoolApi.submitMine(body)
      setSubmitOpen(false); setStatus('submitted'); setNotice('接入声明已提交，尚未登记 Provider 或获得路由权限。')
      await loadForStatus('submitted', receipt.request_id)
    } catch (reason) { setError(messageOf(reason, '接入申请提交失败')); throw reason } finally { setBusy(false) }
  }

  async function cancel() {
    if (!detail) return
    setBusy(true); setError(''); setNotice('')
    try {
      await externalPoolApi.cancelMine(detail.request)
      setAction(null); setStatus('canceled'); setNotice('申请已取消，没有登记 Provider。')
      await loadForStatus('canceled', detail.request.request_id); await loadDetail(detail.request.request_id)
    } catch (reason) { setError(messageOf(reason, '接入申请取消失败')); throw reason } finally { setBusy(false) }
  }

  async function review(decision: GovernanceDecision, note: string | null) {
    if (!detail) return
    setBusy(true); setError(''); setNotice('')
    try {
      await externalPoolApi.reviewOnboarding(detail, decision, note)
      setAction(null); setStatus(decision); setNotice(decision === 'approved' ? '独立复核已批准，Provider 仍未登记。' : '复核决定已保存。')
      await loadForStatus(decision, detail.request.request_id); await loadDetail(detail.request.request_id)
    } catch (reason) { setError(messageOf(reason, '接入申请复核失败')); throw reason } finally { setBusy(false) }
  }

  async function apply() {
    if (!detail) return
    setBusy(true); setError(''); setNotice('')
    try {
      await externalPoolApi.applyOnboarding(detail)
      setAction(null); setStatus('applied'); setNotice('Provider 已登记为 registering/self_declared；未激活、未建立路由。')
      await loadForStatus('applied', detail.request.request_id); await loadDetail(detail.request.request_id)
    } catch (reason) { setError(messageOf(reason, 'Provider 登记失败')); throw reason } finally { setBusy(false) }
  }

  return <section className={styles.workspace}>
    <header className={styles.workspaceHeader}><div><span>{admin ? '平台治理' : 'Provider Owner'}</span><h2>{admin ? '接入审核队列' : '我的接入申请'}</h2></div><div><button type="button" onClick={() => void load()} disabled={loading}><RefreshCw size={14} className={loading ? styles.spinning : ''} />刷新</button>{!admin && <button type="button" className={styles.primary} onClick={() => { setError(''); setSubmitOpen(true) }}><Plus size={14} />新建申请</button>}</div></header>
    {error && !action && !submitOpen && <div className={styles.alert} data-tone="error"><TriangleAlert size={15} />{error}</div>}
    {notice && <div className={styles.alert}><CircleCheck size={15} />{notice}</div>}
    <div className={styles.filters}>{FILTERS.map((filter) => <button type="button" key={filter.value} data-active={status === filter.value} onClick={() => { setStatus(filter.value); setDetail(null); setPreflight(null) }}>{filter.label}</button>)}</div>
    <div className={styles.queueLayout}><aside className={styles.queue}><header><strong>{statusLabel(status)}</strong><span>{items.length}</span></header>{items.map((item) => <button type="button" key={item.request.request_id} data-active={item.request.request_id === selectedId} onClick={() => setSelectedId(item.request.request_id)}><Network size={16} /><span><strong>{item.request.provider_id}</strong><small>{shortId(item.request.request_id)} · {formatTime(item.request.updated_at)}</small></span></button>)}{!loading && !items.length && <div className={styles.empty}>当前状态没有申请</div>}</aside>
      <div className={styles.detail}>{detail ? <OnboardingDetailView detail={detail} preflight={preflight} admin={admin} onAction={setAction} /> : selected ? <div className={styles.empty}>正在读取详情</div> : <div className={styles.empty}><Network size={24} /><strong>选择一份接入申请</strong></div>}</div></div>
    {submitOpen && <OnboardingSubmitDialog busy={busy} error={error} onClose={() => setSubmitOpen(false)} onSubmit={submit} />}
    {action && detail && <OnboardingActionDialog action={action} detail={detail} busy={busy} error={error} onClose={() => setAction(null)} onCancel={cancel} onReview={review} onApply={apply} />}
  </section>
}

function OnboardingDetailView({ detail, preflight, admin, onAction }: { detail: OnboardingDetail; preflight: OnboardingPreflight | null; admin: boolean; onAction: (value: 'cancel' | 'review' | 'apply') => void }) {
  const request = detail.request
  return <><header className={styles.detailHeader}><div><span>Provider ID</span><h3>{request.provider_id}</h3></div><b>{statusLabel(request.status)}</b></header>
    <div className={styles.facts}><div><span>申请人</span><strong>{shortId(request.provider_owner_account_id)}</strong></div><div><span>凭据引用</span><strong>{request.credential_ref_present ? '已保管（不回显）' : '未提供'}</strong></div><div><span>凭据提示</span><strong>{request.credential_hint || '—'}</strong></div><div><span>更新时间</span><strong>{formatTime(request.updated_at)}</strong></div></div>
    {preflight && <section className={preflight.blockers.length ? styles.blocked : styles.ready}>{preflight.provider_conflict ? <ShieldAlert size={18} /> : <CircleCheck size={18} />}<div><strong>{preflight.provider_conflict ? 'Provider ID 已被占用' : actionHint(preflight, admin)}</strong><span>{preflight.blockers.length ? preflight.blockers.map(blockerLabel).join('、') : '当前摘要和账本状态一致'}</span></div></section>}
    <section className={styles.digest}><span>申请摘要</span><code>{request.request_digest}</code><span>Provider 摘要</span><code>{request.target_provider_digest}</code></section>
    <section className={styles.secretBoundary}><KeyRound size={17} /><div><strong>non-bearer 凭据引用不会返回页面</strong><span>本页只有 presence 与脱敏 hint；审批不代表凭据、硬件或外部主体已验证。</span></div></section>
    {detail.review && <Receipt title={`复核：${decisionLabel(detail.review.decision)}`} meta={`${shortId(detail.review.reviewed_by_user_id)} · ${formatTime(detail.review.reviewed_at)}`} digest={detail.review.review_digest} />}
    {detail.application && <Receipt title="已登记 registering Provider" meta={formatTime(detail.application.applied_at)} digest={detail.application.application_digest} />}
    <footer className={styles.actions}><span>该流程不创建 Pool、Offer、Job、route 或结算。</span><div>{!admin && preflight?.owner_cancel_allowed && <button type="button" data-tone="danger" onClick={() => onAction('cancel')}>取消申请</button>}{admin && preflight?.admin_review_allowed && <button type="button" onClick={() => onAction('review')}>独立复核</button>}{admin && preflight?.admin_apply_allowed && <button type="button" className={styles.primary} onClick={() => onAction('apply')}>登记 Provider</button>}</div></footer>
  </>
}

function Receipt({ title, meta, digest }: { title: string; meta: string; digest: string }) { return <section className={styles.receipt}><CircleCheck size={17} /><div><strong>{title}</strong><span>{meta}</span><code>{digest}</code></div></section> }
function actionHint(value: OnboardingPreflight, admin: boolean) { if (!admin) return value.owner_cancel_allowed ? '当前申请仍可取消' : '等待平台治理或查看终态'; if (value.admin_review_allowed) return '当前管理员可独立复核'; if (value.admin_apply_allowed) return '当前管理员可登记 Provider'; return '当前流程无可执行写操作' }
function statusLabel(value: string) { return ({ submitted: '待复核', approved: '已批准', changes_requested: '需补充', rejected: '已拒绝', canceled: '已取消', applied: '已登记' } as Record<string, string>)[value] ?? value }
function decisionLabel(value: string) { return ({ approved: '批准', changes_requested: '退回补充', rejected: '拒绝' } as Record<string, string>)[value] ?? value }
function blockerLabel(value: string) { return ({ current_admin_cannot_review_own_submission: '申请人不能复核自己的申请', changes_requested_requires_new_submission: '需修正后提交新申请', request_rejected: '申请已拒绝', request_canceled: '申请已取消', provider_already_registered: 'Provider 已登记', provider_id_already_registered: 'Provider ID 已被其他记录占用' } as Record<string, string>)[value] ?? value }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function messageOf(reason: unknown, fallback: string) { return reason instanceof Error && reason.message ? reason.message : fallback }
