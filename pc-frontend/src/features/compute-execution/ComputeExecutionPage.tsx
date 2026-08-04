import { useCallback, useEffect, useState } from 'react'
import { CircleCheck, RefreshCw, TriangleAlert } from 'lucide-react'
import { myComputeSettlementApi, type MyComputeProvider } from '../compute-settlement/myComputeSettlementApi'
import { type ComputeReservationReceipt } from '../compute-market/computeMarketApi'
import ActivateAttemptDialog from './ActivateAttemptDialog'
import AttemptLeasePanel from './AttemptLeasePanel'
import ProviderExecutionQueue from './ProviderExecutionQueue'
import { computeExecutionApi, type ActivateComputeAttemptBody, type ComputeAttemptLeaseStateReceipt } from './computeExecutionApi'
import styles from './ComputeExecutionPage.module.css'

export default function ComputeExecutionPage() {
  const [providers, setProviders] = useState<MyComputeProvider[]>([])
  const [providerId, setProviderId] = useState('')
  const [candidates, setCandidates] = useState<ComputeReservationReceipt[]>([])
  const [leases, setLeases] = useState<ComputeAttemptLeaseStateReceipt[]>([])
  const [activateCandidate, setActivateCandidate] = useState<ComputeReservationReceipt | null>(null)
  const [activeLeaseId, setActiveLeaseId] = useState('')
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')

  useEffect(() => { void myComputeSettlementApi.providers().then((items) => { setProviders(items); setProviderId((current) => items.some((item) => item.provider_id === current) ? current : items[0]?.provider_id ?? '') }).catch((reason) => setError(messageOf(reason, 'Provider 读取失败'))) }, [])

  const loadWorkspace = useCallback(async () => {
    if (!providerId) { setCandidates([]); setLeases([]); return }
    setLoading(true); setError('')
    try { const [nextCandidates, nextLeases] = await Promise.all([computeExecutionApi.candidates(providerId), computeExecutionApi.leases(providerId)]); setCandidates(nextCandidates); setLeases(nextLeases) }
    catch (reason) { setError(messageOf(reason, '履约队列读取失败')) } finally { setLoading(false) }
  }, [providerId])

  useEffect(() => { setActiveLeaseId(''); setNotice(''); void loadWorkspace() }, [loadWorkspace])

  async function activate(body: ActivateComputeAttemptBody) {
    if (!providerId || busy) return
    setBusy(true); setError(''); setNotice('')
    try { const receipt = await computeExecutionApi.activate(providerId, body); setActivateCandidate(null); setActiveLeaseId(receipt.lease.lease_id); setNotice('Attempt 激活回执已登记；该回执不代表平台发送了节点命令。'); await loadWorkspace() }
    catch (reason) { setError(messageOf(reason, 'Attempt 激活失败')) } finally { setBusy(false) }
  }

  return <main className={styles.page}><header className={styles.header}><div><span>Provider 控制面</span><h1>算力执行</h1><p>承接已预留任务，登记外部执行器状态和 Lease 生命周期。</p></div><div><select value={providerId} onChange={(event) => { setProviderId(event.target.value); setError('') }} disabled={!providers.length}><option value="">选择 Provider</option>{providers.map((provider) => <option key={provider.provider_id} value={provider.provider_id}>{provider.display_name} · {provider.status}</option>)}</select><button type="button" onClick={() => void loadWorkspace()} disabled={!providerId || loading}><RefreshCw size={14} className={loading ? styles.spinning : ''} />刷新</button></div></header>{error && !activateCandidate && <div className={styles.alert} data-tone="error"><TriangleAlert size={15} />{error}</div>}{notice && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />{notice}</div>}<div className={styles.workspace}><ProviderExecutionQueue key={providerId} candidates={candidates} leases={leases} loading={loading} selectedLeaseId={activeLeaseId} onSelectLease={setActiveLeaseId} onActivate={(candidate) => { setError(''); setActivateCandidate(candidate) }} /><AttemptLeasePanel providerId={providerId} initialLeaseId={activeLeaseId} onStateChanged={loadWorkspace} /></div>{activateCandidate && <ActivateAttemptDialog candidate={activateCandidate} busy={busy} error={error} onClose={() => setActivateCandidate(null)} onSubmit={activate} />}</main>
}

function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
