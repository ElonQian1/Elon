import { useCallback, useEffect, useState } from 'react'
import { CircleCheck, RefreshCw, ShieldAlert, TriangleAlert } from 'lucide-react'
import { myComputeSettlementApi, type MyComputeProvider } from '../compute-settlement/myComputeSettlementApi'
import { type ComputeReservationReceipt } from '../compute-market/computeMarketApi'
import AttemptLeasePanel from './AttemptLeasePanel'
import ProviderExecutionQueue from './ProviderExecutionQueue'
import { computeExecutionApi, type ComputeAttemptLeaseStateReceipt } from './computeExecutionApi'
import styles from './ComputeExecutionPage.module.css'

export default function ComputeExecutionPage() {
  const [providers, setProviders] = useState<MyComputeProvider[]>([])
  const [providerId, setProviderId] = useState('')
  const [candidates, setCandidates] = useState<ComputeReservationReceipt[]>([])
  const [leases, setLeases] = useState<ComputeAttemptLeaseStateReceipt[]>([])
  const [activeLeaseId, setActiveLeaseId] = useState('')
  const [loading, setLoading] = useState(false)
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

  return <main className={styles.page}><header className={styles.header}><div><span>Provider 控制面</span><h1>算力执行</h1><p>读取履约队列，登记 Provider 声明和终态候选。</p></div><div><select value={providerId} onChange={(event) => { setProviderId(event.target.value); setError('') }} disabled={!providers.length}><option value="">选择 Provider</option>{providers.map((provider) => <option key={provider.provider_id} value={provider.provider_id}>{provider.display_name} · {provider.status}</option>)}</select><button type="button" onClick={() => void loadWorkspace()} disabled={!providerId || loading}><RefreshCw size={14} className={loading ? styles.spinning : ''} />刷新</button></div></header><div className={styles.alert} data-tone="boundary"><ShieldAlert size={15} />Start、Renew 与 no-start Abort 只能由认证 Gateway 推进，当前 PC 工作台不会调用失败关闭的人工写入口。</div>{error && <div className={styles.alert} data-tone="error"><TriangleAlert size={15} />{error}</div>}{notice && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />{notice}</div>}<div className={styles.workspace}><ProviderExecutionQueue key={providerId} candidates={candidates} leases={leases} loading={loading} selectedLeaseId={activeLeaseId} onSelectLease={setActiveLeaseId} /><AttemptLeasePanel providerId={providerId} initialLeaseId={activeLeaseId} /></div></main>
}

function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
