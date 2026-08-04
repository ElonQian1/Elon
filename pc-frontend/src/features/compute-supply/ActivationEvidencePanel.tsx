import { useCallback, useEffect, useMemo, useState } from 'react'
import { CircleCheck, FileCheck2, Plus, RefreshCw, ShieldAlert, X } from 'lucide-react'
import {
  computeActivationApi,
  type ComputeActivationEvidenceRequest,
  type ComputeActivationPreflightReport,
  type SubmitActivationEvidenceBody,
} from './computeActivationApi'
import SubmitActivationEvidenceDialog from './SubmitActivationEvidenceDialog'
import styles from './ActivationEvidencePanel.module.css'

interface Props {
  providerId: string
  poolId: string
  poolStatus: string
}

export default function ActivationEvidencePanel({ providerId, poolId, poolStatus }: Props) {
  const [requests, setRequests] = useState<ComputeActivationEvidenceRequest[]>([])
  const [selectedId, setSelectedId] = useState('')
  const [preflight, setPreflight] = useState<ComputeActivationPreflightReport | null>(null)
  const [loading, setLoading] = useState(false)
  const [writing, setWriting] = useState(false)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [cancelArmed, setCancelArmed] = useState(false)
  const [error, setError] = useState('')
  const selected = useMemo(() => requests.find((request) => request.request_id === selectedId) ?? null, [requests, selectedId])
  const hasLiveRequest = requests.some((request) => request.status === 'submitted' || request.status === 'approved')

  const load = useCallback(async () => {
    setLoading(true); setError('')
    try {
      const response = await computeActivationApi.list(providerId, poolId)
      setRequests(response.activation_evidence_requests)
      setSelectedId((current) => response.activation_evidence_requests.some((item) => item.request_id === current) ? current : response.activation_evidence_requests[0]?.request_id ?? '')
    } catch (reason) { setError(messageOf(reason, '激活证据申请读取失败')) } finally { setLoading(false) }
  }, [poolId, providerId])

  useEffect(() => { void load() }, [load])
  useEffect(() => {
    setCancelArmed(false); setPreflight(null)
    if (!selectedId) return
    let current = true
    void computeActivationApi.preflight(providerId, poolId, selectedId).then((report) => { if (current) setPreflight(report) }).catch((reason) => { if (current) setError(messageOf(reason, '激活预检读取失败')) })
    return () => { current = false }
  }, [poolId, providerId, selectedId])

  async function submit(body: SubmitActivationEvidenceBody) {
    if (writing) return
    setWriting(true); setError('')
    try { const receipt = await computeActivationApi.submit(providerId, poolId, body); await load(); setSelectedId(receipt.request.request_id); setDialogOpen(false) }
    catch (reason) { setError(messageOf(reason, '激活证据提交失败')) } finally { setWriting(false) }
  }

  async function cancel() {
    if (!selected || selected.status !== 'submitted' || writing) return
    setWriting(true); setError('')
    try { await computeActivationApi.cancel(providerId, poolId, selected.request_id, selected.request_digest); await load(); setCancelArmed(false) }
    catch (reason) { setError(messageOf(reason, '激活证据申请取消失败')) } finally { setWriting(false) }
  }

  return <section className={styles.panel}>
    <header className={styles.header}><div><h3>激活证据申请</h3><span>供给者提交，平台管理员审核</span></div><div className={styles.headerActions}><button type="button" onClick={() => void load()} disabled={loading} aria-label="刷新申请" title="刷新申请"><RefreshCw size={14} className={loading ? styles.spinning : ''} /></button><button type="button" className={styles.primaryButton} onClick={() => { setError(''); setDialogOpen(true) }} disabled={poolStatus !== 'registering' || hasLiveRequest}><Plus size={14} />提交申请</button></div></header>
    {error && !dialogOpen && <div className={styles.error}>{error}</div>}
    <div className={styles.content}>
      <div className={styles.requestList}>{requests.map((request) => <button type="button" key={request.request_id} data-active={request.request_id === selectedId} onClick={() => setSelectedId(request.request_id)}><FileCheck2 size={15} /><span><strong>{statusLabel(request.status)}</strong><small>{formatTime(request.requested_at)}</small></span></button>)}{requests.length === 0 && !loading && <div className={styles.empty}>尚未提交激活证据</div>}</div>
      <div className={styles.detail}>{selected ? <>
        <div className={styles.requestHeader}><div><span>申请 ID</span><strong>{shortId(selected.request_id)}</strong></div><span className={styles.status}>{statusLabel(selected.status)}</span></div>
        <div className={styles.refs}><div><span>节点绑定</span><code>{selected.node_binding_ref}</code></div><div><span>ReadyCapability</span><code>{shortDigest(selected.ready_capability_digest)}</code></div><div><span>路由证明</span><code>{shortDigest(selected.route_proof_digest)}</code></div><div><span>硬件观测</span><code>{shortDigest(selected.hardware_observation_digest)}</code></div></div>
        {selected.review_note && <div className={styles.reviewNote}><span>审核说明</span><p>{selected.review_note}</p></div>}
        {preflight && <div className={preflight.ready_for_activation ? styles.ready : styles.blocked}>{preflight.ready_for_activation ? <CircleCheck size={16} /> : <ShieldAlert size={16} />}<div><strong>{preflight.ready_for_activation ? '当前预检无阻断项' : `当前有 ${preflight.blockers.length} 项阻断`}</strong><span>{preflight.blockers.length ? preflight.blockers.map(blockerLabel).join('、') : '仍需管理员准备并应用激活计划'}</span></div></div>}
        {selected.status === 'submitted' && <div className={styles.cancelArea}>{cancelArmed ? <><span>确认取消这份待审核申请？历史记录仍会保留。</span><button type="button" onClick={() => void cancel()} disabled={writing}>确认取消</button><button type="button" onClick={() => setCancelArmed(false)} disabled={writing}><X size={13} />返回</button></> : <button type="button" onClick={() => setCancelArmed(true)}>取消待审核申请</button>}</div>}
      </> : <div className={styles.detailEmpty}><FileCheck2 size={21} /><span>选择一份申请查看状态</span></div>}</div>
    </div>
    <div className={styles.boundary}>提交、审核通过和预检就绪都不会自动激活资源；只有管理员受控应用精确计划才改变内部状态，且仍不发布报价、派发任务或移动资金。</div>
    {dialogOpen && <SubmitActivationEvidenceDialog poolId={poolId} busy={writing} error={error} onClose={() => setDialogOpen(false)} onSubmit={submit} />}
  </section>
}

function statusLabel(value: string) { return ({ submitted: '待审核', changes_requested: '需补充', approved: '已批准', activated: '已激活', rejected: '已拒绝', canceled: '已取消', superseded: '已废止' } as Record<string, string>)[value] ?? value }
function blockerLabel(value: string) { return ({ request_not_approved: '申请尚未批准', provider_ownership_changed: 'Provider 所有权变化', provider_version_changed: 'Provider 版本变化', provider_not_registering: 'Provider 非登记状态', provider_routing_missing: '缺少路由', verified_hardware_missing: '缺少已验证硬件', verified_at_missing: '缺少验证时间', provider_trust_tier_self_declared: '仍为自我声明信任层', provider_regions_missing: '缺少服务区域', pool_provider_changed: 'Pool 归属变化', pool_version_changed: 'Pool 版本变化', pool_not_registering: 'Pool 非登记状态', ledger_audit_unhealthy: '账本审计异常', ledger_audit_changed: '账本审计摘要变化' } as Record<string, string>)[value] ?? value }
function shortId(value: string) { return value.length <= 28 ? value : `${value.slice(0, 14)}…${value.slice(-8)}` }
function shortDigest(value: string) { return value.length <= 22 ? value : `${value.slice(0, 10)}…${value.slice(-8)}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
