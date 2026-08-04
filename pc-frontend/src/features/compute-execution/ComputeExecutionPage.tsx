import { useCallback, useEffect, useState } from 'react'
import { CircleCheck, LoaderCircle, Play, RefreshCw, Server, TriangleAlert } from 'lucide-react'
import { myComputeSettlementApi, type MyComputeProvider } from '../compute-settlement/myComputeSettlementApi'
import { type ComputeReservationReceipt } from '../compute-market/computeMarketApi'
import ActivateAttemptDialog from './ActivateAttemptDialog'
import AttemptLeasePanel from './AttemptLeasePanel'
import { computeExecutionApi, type ActivateComputeAttemptBody } from './computeExecutionApi'
import styles from './ComputeExecutionPage.module.css'

export default function ComputeExecutionPage() {
  const [providers, setProviders] = useState<MyComputeProvider[]>([])
  const [providerId, setProviderId] = useState('')
  const [candidates, setCandidates] = useState<ComputeReservationReceipt[]>([])
  const [activateCandidate, setActivateCandidate] = useState<ComputeReservationReceipt | null>(null)
  const [activeLeaseId, setActiveLeaseId] = useState('')
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')

  useEffect(() => { void myComputeSettlementApi.providers().then((items) => { setProviders(items); setProviderId((current) => items.some((item) => item.provider_id === current) ? current : items[0]?.provider_id ?? '') }).catch((reason) => setError(messageOf(reason, 'Provider 读取失败'))) }, [])

  const loadCandidates = useCallback(async () => {
    if (!providerId) { setCandidates([]); return }
    setLoading(true); setError('')
    try { setCandidates(await computeExecutionApi.candidates(providerId)) }
    catch (reason) { setError(messageOf(reason, '待激活任务读取失败')) } finally { setLoading(false) }
  }, [providerId])

  useEffect(() => { setActiveLeaseId(''); setNotice(''); void loadCandidates() }, [loadCandidates])

  async function activate(body: ActivateComputeAttemptBody) {
    if (!providerId || busy) return
    setBusy(true); setError(''); setNotice('')
    try { const receipt = await computeExecutionApi.activate(providerId, body); setActivateCandidate(null); setActiveLeaseId(receipt.lease.lease_id); setNotice('Attempt 激活回执已登记；该回执不代表平台发送了节点命令。'); await loadCandidates() }
    catch (reason) { setError(messageOf(reason, 'Attempt 激活失败')) } finally { setBusy(false) }
  }

  return <main className={styles.page}><header className={styles.header}><div><span>Provider 控制面</span><h1>算力执行</h1><p>承接已预留任务，登记外部执行器状态和 Lease 生命周期。</p></div><div><select value={providerId} onChange={(event) => { setProviderId(event.target.value); setError('') }} disabled={!providers.length}><option value="">选择 Provider</option>{providers.map((provider) => <option key={provider.provider_id} value={provider.provider_id}>{provider.display_name} · {provider.status}</option>)}</select><button type="button" onClick={() => void loadCandidates()} disabled={!providerId || loading}><RefreshCw size={14} className={loading ? styles.spinning : ''} />刷新</button></div></header>{error && !activateCandidate && <div className={styles.alert} data-tone="error"><TriangleAlert size={15} />{error}</div>}{notice && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />{notice}</div>}<div className={styles.workspace}><section className={styles.queue}><header><div><h2>待激活履约</h2><span>当前有效、尚无 Attempt 的 reserved Job</span></div><b>{candidates.length}</b></header>{loading && !candidates.length && <div className={styles.empty}><LoaderCircle size={16} className={styles.spinning} />读取候选</div>}{!loading && !candidates.length && <div className={styles.empty}>当前 Provider 没有待激活任务</div>}{candidates.map((candidate) => <div className={styles.candidate} key={candidate.reservation.reservation_id}><div className={styles.candidateIcon}><Server size={17} /></div><div><strong>{candidate.reservation.price_snapshot.sku.task_kind}</strong><span>{shortId(candidate.reservation.reservation_id)} · 至 {formatTime(candidate.reservation.expires_at)}</span><small>{candidate.reservation.reserved_capacity.map((item) => `${item.meter} ${item.quantity}`).join(' · ')}</small></div><div><strong>{formatAmount(candidate.reservation.price_snapshot.consumer_max_amount_micros)}</strong><span>rev {candidate.revision}</span></div><button type="button" onClick={() => { setError(''); setActivateCandidate(candidate) }}><Play size={14} />登记激活</button></div>)}</section><AttemptLeasePanel providerId={providerId} initialLeaseId={activeLeaseId} /></div>{activateCandidate && <ActivateAttemptDialog candidate={activateCandidate} busy={busy} error={error} onClose={() => setActivateCandidate(null)} onSubmit={activate} />}</main>
}

function formatAmount(micros: number) { return `CNY ${(micros / 1_000_000).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 6 })}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-7)}` }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
