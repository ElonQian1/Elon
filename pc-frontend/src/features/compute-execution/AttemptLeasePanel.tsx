import { useCallback, useEffect, useState } from 'react'
import {
  CircleCheck, FileCheck2, Gauge, HeartPulse, LoaderCircle,
  OctagonX, RefreshCw, Search, TriangleAlert,
} from 'lucide-react'
import AbortAttemptDialog from './AbortAttemptDialog'
import DeclareUsageDialog from './DeclareUsageDialog'
import RenewAttemptLeaseDialog from './RenewAttemptLeaseDialog'
import TerminalCandidateDialog from './TerminalCandidateDialog'
import {
  computeExecutionApi,
  type AbortComputeAttemptBody,
  type ComputeAttemptActivationReceipt,
  type ComputeAttemptLeaseStateReceipt,
  type ComputeAttemptTerminalCandidateReceipt,
  type ComputeAttemptUsageTemplateReceipt,
  type DeclareComputeAttemptTerminalCandidateBody,
  type DeclareComputeAttemptUsageBody,
  type RenewComputeAttemptLeaseBody,
} from './computeExecutionApi'
import styles from './ComputeExecutionPage.module.css'

interface Props {
  providerId: string
  initialLeaseId: string
  onStateChanged: () => Promise<void>
}

export default function AttemptLeasePanel({ providerId, initialLeaseId, onStateChanged }: Props) {
  const [leaseId, setLeaseId] = useState(initialLeaseId)
  const [activation, setActivation] = useState<ComputeAttemptActivationReceipt | null>(null)
  const [state, setState] = useState<ComputeAttemptLeaseStateReceipt | null>(null)
  const [usageTemplate, setUsageTemplate] = useState<ComputeAttemptUsageTemplateReceipt | null>(null)
  const [terminalCandidate, setTerminalCandidate] = useState<ComputeAttemptTerminalCandidateReceipt | null>(null)
  const [dialog, setDialog] = useState<'renew' | 'abort' | 'usage' | 'terminal' | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')

  const load = useCallback(async (requestedId: string) => {
    const id = requestedId.trim()
    if (!id) return
    setLoading(true); setError(''); setNotice('')
    try {
      const [nextActivation, nextState] = await Promise.all([
        computeExecutionApi.activation(id), computeExecutionApi.leaseState(id),
      ])
      const sameProvider = nextState.lease.provider_id === providerId
      const liveRunning = nextState.lease.status === 'running'
        && Boolean(nextState.lease.last_heartbeat_at)
        && new Date(nextState.lease.expires_at).getTime() > Date.now()
        && new Date(nextState.lease.hard_deadline_at).getTime() > Date.now()
      setLeaseId(id); setActivation(nextActivation); setState(nextState)
      const [templateResult, candidateResult] = await Promise.allSettled([
        sameProvider && liveRunning ? computeExecutionApi.usageTemplate(providerId, id) : Promise.resolve(null),
        optionalTerminalCandidate(id),
      ])
      setUsageTemplate(templateResult.status === 'fulfilled' ? templateResult.value : null)
      setTerminalCandidate(candidateResult.status === 'fulfilled' ? candidateResult.value : null)
      const extraFailure = [templateResult, candidateResult].find((result) => result.status === 'rejected')
      if (extraFailure?.status === 'rejected') setError(messageOf(extraFailure.reason, 'Attempt 证据读取失败'))
    } catch (reason) {
      setActivation(null); setState(null); setUsageTemplate(null); setTerminalCandidate(null)
      setError(messageOf(reason, 'Attempt Lease 读取失败'))
    } finally { setLoading(false) }
  }, [providerId])

  useEffect(() => {
    setActivation(null); setState(null); setUsageTemplate(null); setTerminalCandidate(null)
    setError(''); setNotice(''); setDialog(null)
    if (initialLeaseId) { setLeaseId(initialLeaseId); void load(initialLeaseId) } else setLeaseId('')
  }, [initialLeaseId, load, providerId])

  async function renew(body: RenewComputeAttemptLeaseBody) {
    if (!state || busy) return
    setBusy(true); setError('')
    try {
      const receipt = await computeExecutionApi.renew(providerId, state, body)
      setDialog(null)
      await Promise.all([load(state.lease.lease_id), onStateChanged()])
      setNotice(`Lease 已续租到 revision ${receipt.state.lease_revision}。`)
    } catch (reason) { setError(messageOf(reason, 'Lease 续租失败')) } finally { setBusy(false) }
  }

  async function abort(body: AbortComputeAttemptBody) {
    if (!activation || !state || busy) return
    setBusy(true); setError('')
    try {
      const receipt = await computeExecutionApi.abort(providerId, state.lease.lease_id, body)
      setDialog(null)
      await Promise.all([load(state.lease.lease_id), onStateChanged()])
      setNotice(`Attempt 已安全中止，退回 CNY ${(receipt.budget_refunded_fen / 100).toFixed(2)}。`)
    } catch (reason) { setError(messageOf(reason, 'Attempt 中止失败')) } finally { setBusy(false) }
  }

  async function declareUsage(body: DeclareComputeAttemptUsageBody) {
    if (!state || busy) return
    setBusy(true); setError('')
    try {
      const receipt = await computeExecutionApi.declareUsage(providerId, state.lease.lease_id, body)
      setDialog(null); await load(state.lease.lease_id)
      setNotice(`累计用量快照 #${receipt.sequence_no} 已登记；当前仍为未验证声明。`)
    } catch (reason) { setError(messageOf(reason, '累计用量登记失败')) } finally { setBusy(false) }
  }

  async function declareTerminal(body: DeclareComputeAttemptTerminalCandidateBody) {
    if (!state || busy) return
    setBusy(true); setError('')
    try {
      const receipt = await computeExecutionApi.declareTerminalCandidate(providerId, state.lease.lease_id, body)
      setTerminalCandidate(receipt); setDialog(null)
      setNotice('Provider 终态候选已保存；Lease、容量和资金状态均未改变。')
    } catch (reason) { setError(messageOf(reason, '终态候选提交失败')) } finally { setBusy(false) }
  }

  const sameProvider = state?.lease.provider_id === providerId
  const unexpired = Boolean(state && new Date(state.lease.expires_at).getTime() > Date.now())
  const canRenew = Boolean(sameProvider && state && ['staging', 'running'].includes(state.lease.status) && unexpired)
  const canAbort = Boolean(sameProvider && state?.lease.status === 'staging' && state.lease_revision === 1 && !state.lease.last_heartbeat_at)
  const canDeclareUsage = Boolean(sameProvider && usageTemplate && state?.lease.status === 'running' && unexpired && !terminalCandidate)
  const canDeclareTerminal = Boolean(canDeclareUsage && usageTemplate?.latest_snapshot)

  return <section className={styles.leasePanel}>
    <header><div><h2>Attempt Lease</h2><span>读取当前状态、外部声明和证据边界</span></div><div className={styles.lookup}><input value={leaseId} onChange={(event) => setLeaseId(event.target.value)} placeholder="lease_id" /><button type="button" onClick={() => void load(leaseId)} disabled={!leaseId.trim() || loading}>{loading ? <LoaderCircle size={14} className={styles.spinning} /> : <Search size={14} />}读取</button></div></header>
    {error && !dialog && <div className={styles.alert} data-tone="error"><TriangleAlert size={14} />{error}</div>}
    {notice && <div className={styles.alert} data-tone="success"><CircleCheck size={14} />{notice}</div>}
    {!state && !loading && <div className={styles.empty}>从履约队列选择 Lease，也可以输入稳定 Lease ID</div>}
    {state && activation && <div className={styles.leaseBody}>
      <div className={styles.leaseTitle}><div><strong>{state.lease.lease_id}</strong><span>{state.lease.executor_id} · fencing {state.lease.fencing_generation}</span></div><b>{statusLabel(state.lease.status)}</b></div>
      <div className={styles.leaseFacts}><div><span>revision</span><strong>{state.lease_revision}</strong></div><div><span>软期限</span><strong>{formatTime(state.lease.expires_at)}</strong></div><div><span>硬期限</span><strong>{formatTime(state.lease.hard_deadline_at)}</strong></div><div><span>最近心跳</span><strong>{state.lease.last_heartbeat_at ? formatTime(state.lease.last_heartbeat_at) : '尚无'}</strong></div></div>
      <div className={styles.bindings}><span>Job {shortId(state.lease.job_id)}</span><span>Reservation {shortId(state.lease.reservation_id)}</span><span>预算 CNY {(activation.budget_reserved_fen / 100).toFixed(2)}</span></div>
      {usageTemplate?.latest_snapshot && <div className={styles.evidence}><Gauge size={14} /><span><strong>用量快照 #{usageTemplate.latest_snapshot.sequence_no}</strong><small>{shortId(usageTemplate.latest_snapshot.snapshot_id)} · provider_declared</small></span></div>}
      {terminalCandidate && <div className={styles.evidence}><FileCheck2 size={14} /><span><strong>{outcomeLabel(terminalCandidate.outcome)}候选</strong><small>{shortId(terminalCandidate.terminal_candidate_id)} · 未验证</small></span></div>}
      <code>{state.lease_digest}</code>
      <footer><button type="button" onClick={() => void load(state.lease.lease_id)} disabled={loading}><RefreshCw size={14} />刷新</button><button type="button" onClick={() => openDialog('renew', setDialog, setError)} disabled={!canRenew}><HeartPulse size={14} />续租</button><button type="button" onClick={() => openDialog('usage', setDialog, setError)} disabled={!canDeclareUsage}><Gauge size={14} />登记用量</button><button type="button" onClick={() => openDialog('terminal', setDialog, setError)} disabled={!canDeclareTerminal}><FileCheck2 size={14} />终态候选</button><button type="button" data-tone="danger" onClick={() => openDialog('abort', setDialog, setError)} disabled={!canAbort}><OctagonX size={14} />无用量中止</button></footer>
      {!sameProvider && <div className={styles.scopeWarning}>该 Lease 不属于当前选择的 Provider，只能读取，不能执行 Provider 写操作。</div>}
    </div>}
    {dialog === 'renew' && state && <RenewAttemptLeaseDialog state={state} busy={busy} error={error} onClose={() => setDialog(null)} onSubmit={renew} />}
    {dialog === 'abort' && activation && state && <AbortAttemptDialog activation={activation} state={state} busy={busy} error={error} onClose={() => setDialog(null)} onSubmit={abort} />}
    {dialog === 'usage' && usageTemplate && <DeclareUsageDialog template={usageTemplate} busy={busy} error={error} onClose={() => setDialog(null)} onSubmit={declareUsage} />}
    {dialog === 'terminal' && usageTemplate && <TerminalCandidateDialog template={usageTemplate} busy={busy} error={error} onClose={() => setDialog(null)} onSubmit={declareTerminal} />}
  </section>
}

async function optionalTerminalCandidate(leaseId: string) {
  try { return await computeExecutionApi.terminalCandidate(leaseId) }
  catch (reason) { if (isMissingCandidate(reason)) return null; throw reason }
}
function isMissingCandidate(reason: unknown) { return Boolean(reason && typeof reason === 'object' && 'status' in reason && reason.status === 400 && 'message' in reason && typeof reason.message === 'string' && reason.message.includes('尚无终态候选')) }
function openDialog(value: 'renew' | 'abort' | 'usage' | 'terminal', setDialog: (value: 'renew' | 'abort' | 'usage' | 'terminal') => void, setError: (value: string) => void) { setError(''); setDialog(value) }
function statusLabel(value: string) { return ({ staging: '准备中', running: '运行中', result_reported: '结果已声明', verifying: '验证中', terminal: '已终结' } as Record<string, string>)[value] ?? value }
function outcomeLabel(value: string) { return ({ succeeded: '成功', failed: '失败', canceled: '取消' } as Record<string, string>)[value] ?? value }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-7)}` }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
