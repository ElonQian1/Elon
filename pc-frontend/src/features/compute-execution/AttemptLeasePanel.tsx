import { useCallback, useEffect, useState } from 'react'
import { CircleCheck, HeartPulse, LoaderCircle, OctagonX, RefreshCw, Search, TriangleAlert } from 'lucide-react'
import AbortAttemptDialog from './AbortAttemptDialog'
import RenewAttemptLeaseDialog from './RenewAttemptLeaseDialog'
import {
  computeExecutionApi,
  type AbortComputeAttemptBody,
  type ComputeAttemptActivationReceipt,
  type ComputeAttemptLeaseStateReceipt,
  type RenewComputeAttemptLeaseBody,
} from './computeExecutionApi'
import styles from './ComputeExecutionPage.module.css'

interface Props { providerId: string; initialLeaseId: string; onStateChanged: () => Promise<void> }

export default function AttemptLeasePanel({ providerId, initialLeaseId, onStateChanged }: Props) {
  const [leaseId, setLeaseId] = useState(initialLeaseId)
  const [activation, setActivation] = useState<ComputeAttemptActivationReceipt | null>(null)
  const [state, setState] = useState<ComputeAttemptLeaseStateReceipt | null>(null)
  const [renewOpen, setRenewOpen] = useState(false)
  const [abortOpen, setAbortOpen] = useState(false)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')

  const load = useCallback(async (requestedId: string) => {
    const id = requestedId.trim()
    if (!id) return
    setLoading(true); setError(''); setNotice('')
    try {
      const [nextActivation, nextState] = await Promise.all([computeExecutionApi.activation(id), computeExecutionApi.leaseState(id)])
      setLeaseId(id); setActivation(nextActivation); setState(nextState)
    } catch (reason) { setActivation(null); setState(null); setError(messageOf(reason, 'Attempt Lease 读取失败')) } finally { setLoading(false) }
  }, [])

  useEffect(() => {
    setActivation(null); setState(null); setError(''); setNotice('')
    if (initialLeaseId) { setLeaseId(initialLeaseId); void load(initialLeaseId) } else setLeaseId('')
  }, [initialLeaseId, load, providerId])

  async function renew(body: RenewComputeAttemptLeaseBody) {
    if (!state || busy) return
    setBusy(true); setError('')
    try { const receipt = await computeExecutionApi.renew(providerId, state, body); setState(receipt.state); setRenewOpen(false); await onStateChanged(); setNotice(`Lease 已续租到 revision ${receipt.state.lease_revision}。`) }
    catch (reason) { setError(messageOf(reason, 'Lease 续租失败')) } finally { setBusy(false) }
  }

  async function abort(body: AbortComputeAttemptBody) {
    if (!activation || !state || busy) return
    setBusy(true); setError('')
    try { const receipt = await computeExecutionApi.abort(providerId, state.lease.lease_id, body); setAbortOpen(false); await Promise.all([load(state.lease.lease_id), onStateChanged()]); setNotice(`Attempt 已安全中止，退回 CNY ${(receipt.budget_refunded_fen / 100).toFixed(2)}。`) }
    catch (reason) { setError(messageOf(reason, 'Attempt 中止失败')) } finally { setBusy(false) }
  }

  const sameProvider = state?.lease.provider_id === providerId
  const canRenew = Boolean(sameProvider && state && ['staging', 'running'].includes(state.lease.status) && new Date(state.lease.expires_at).getTime() > Date.now())
  const canAbort = Boolean(sameProvider && state?.lease.status === 'staging' && state.lease_revision === 1 && !state.lease.last_heartbeat_at)

  return <section className={styles.leasePanel}><header><div><h2>Attempt Lease</h2><span>按稳定 Lease ID 读取参与者可见的当前状态</span></div><div className={styles.lookup}><input value={leaseId} onChange={(event) => setLeaseId(event.target.value)} placeholder="lease_id" /><button type="button" onClick={() => void load(leaseId)} disabled={!leaseId.trim() || loading}>{loading ? <LoaderCircle size={14} className={styles.spinning} /> : <Search size={14} />}读取</button></div></header>{error && !renewOpen && !abortOpen && <div className={styles.alert} data-tone="error"><TriangleAlert size={14} />{error}</div>}{notice && <div className={styles.alert} data-tone="success"><CircleCheck size={14} />{notice}</div>}{!state && !loading && <div className={styles.empty}>登记新激活后会自动载入，也可以输入已有 Lease ID</div>}{state && activation && <div className={styles.leaseBody}><div className={styles.leaseTitle}><div><strong>{state.lease.lease_id}</strong><span>{state.lease.executor_id} · fencing {state.lease.fencing_generation}</span></div><b>{statusLabel(state.lease.status)}</b></div><div className={styles.leaseFacts}><div><span>revision</span><strong>{state.lease_revision}</strong></div><div><span>软期限</span><strong>{formatTime(state.lease.expires_at)}</strong></div><div><span>硬期限</span><strong>{formatTime(state.lease.hard_deadline_at)}</strong></div><div><span>最近心跳</span><strong>{state.lease.last_heartbeat_at ? formatTime(state.lease.last_heartbeat_at) : '尚无'}</strong></div></div><div className={styles.bindings}><span>Job {shortId(state.lease.job_id)}</span><span>Reservation {shortId(state.lease.reservation_id)}</span><span>预算 CNY {(activation.budget_reserved_fen / 100).toFixed(2)}</span></div><code>{state.lease_digest}</code><footer><button type="button" onClick={() => void load(state.lease.lease_id)} disabled={loading}><RefreshCw size={14} />刷新</button><button type="button" onClick={() => { setError(''); setRenewOpen(true) }} disabled={!canRenew}><HeartPulse size={14} />续租</button><button type="button" data-tone="danger" onClick={() => { setError(''); setAbortOpen(true) }} disabled={!canAbort}><OctagonX size={14} />无用量中止</button></footer>{!sameProvider && <div className={styles.scopeWarning}>该 Lease 不属于当前选择的 Provider，只能读取，不能执行 Provider 写操作。</div>}</div>}{renewOpen && state && <RenewAttemptLeaseDialog state={state} busy={busy} error={error} onClose={() => setRenewOpen(false)} onSubmit={renew} />}{abortOpen && activation && state && <AbortAttemptDialog activation={activation} state={state} busy={busy} error={error} onClose={() => setAbortOpen(false)} onSubmit={abort} />}</section>
}

function statusLabel(value: string) { return ({ staging: '准备中', running: '运行中', result_reported: '结果已声明', verifying: '验证中', terminal: '已终结' } as Record<string, string>)[value] ?? value }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-7)}` }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
