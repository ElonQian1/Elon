import { useCallback, useEffect, useState } from 'react'
import { CircleCheck, FileSignature, LoaderCircle, RefreshCw, TriangleAlert } from 'lucide-react'
import IssueExecutionReceiptDialog from './IssueExecutionReceiptDialog'
import {
  computeExecutionReceiptApi,
  type ComputeAttemptExecutionReceiptEnvelope,
  type ComputePendingExecutionReceiptCandidate,
  type IssueComputeAttemptExecutionReceiptBody,
} from './computeExecutionReceiptApi'
import styles from './ComputeExecutionReceiptPage.module.css'

export default function ComputeExecutionReceiptPage() {
  const [candidates, setCandidates] = useState<ComputePendingExecutionReceiptCandidate[]>([])
  const [selected, setSelected] = useState<ComputePendingExecutionReceiptCandidate | null>(null)
  const [latest, setLatest] = useState<ComputeAttemptExecutionReceiptEnvelope | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [dialogError, setDialogError] = useState('')

  const loadCandidates = useCallback(async () => {
    setLoading(true)
    setError('')
    try {
      setCandidates(await computeExecutionReceiptApi.listPending())
    } catch (reason) {
      setError(messageOf(reason, '待签发回执读取失败'))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { void loadCandidates() }, [loadCandidates])

  async function issue(body: IssueComputeAttemptExecutionReceiptBody) {
    if (!selected || busy) return
    setBusy(true)
    setDialogError('')
    try {
      const envelope = await computeExecutionReceiptApi.issue(selected, body)
      setLatest(envelope)
      setSelected(null)
      await loadCandidates()
    } catch (reason) {
      setDialogError(messageOf(reason, 'Execution Receipt 签发失败'))
    } finally {
      setBusy(false)
    }
  }

  return (
    <main className={styles.page}>
      <header className={styles.pageHeader}>
        <div><span>管理员控制面</span><h1>执行回执</h1><p>accepted Verification 的不可变执行事实</p></div>
        <button type="button" onClick={() => void loadCandidates()} disabled={loading} title="刷新待签发回执"><RefreshCw size={15} className={loading ? styles.spinning : ''} />刷新</button>
      </header>

      {error && <div className={styles.alert} data-tone="error"><TriangleAlert size={15} />{error}</div>}
      {latest && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />已签发 Execution Receipt · {shortId(latest.receipt.receipt_id)}</div>}

      <section className={styles.queue}>
        <header><div><FileSignature size={17} /><h2>待签发验证决定</h2></div><span>{candidates.length}</span></header>
        {loading && !candidates.length && <div className={styles.empty}><LoaderCircle size={17} className={styles.spinning} />读取候选</div>}
        {!loading && !candidates.length && <div className={styles.empty}>暂无待签发 Execution Receipt</div>}
        <div className={styles.candidateList}>
          {candidates.map((item) => {
            const verification = item.verification_decision
            const terminal = item.terminal_candidate
            return (
              <article className={styles.candidate} key={verification.verification_decision_id}>
                <header>
                  <div><b>accepted</b><span>{formatTime(verification.decided_at)}</span></div>
                  <button type="button" onClick={() => { setDialogError(''); setSelected(item) }}>签发回执</button>
                </header>
                <div className={styles.facts}>
                  <div><span>Job</span><strong>{shortId(verification.job_id)}</strong></div>
                  <div><span>Provider</span><strong>{shortId(verification.provider_id)}</strong></div>
                  <div><span>执行结果</span><strong>{outcomeLabel(terminal.outcome)}</strong></div>
                  <div><span>输出工件</span><strong>{terminal.result_artifacts.length}</strong></div>
                </div>
                <div className={styles.meters}>{verification.verified_usage.map((reading) => {
                  const compensable = verification.compensable_usage.find((value) => value.meter === reading.meter)?.quantity
                  return <span key={reading.meter}>{reading.meter}: {reading.quantity} / {compensable ?? 0}</span>
                })}</div>
                <code className={styles.digest}>{verification.event_digest}</code>
              </article>
            )
          })}
        </div>
      </section>

      {selected && <IssueExecutionReceiptDialog candidate={selected} busy={busy} error={dialogError} onClose={() => { if (!busy) setSelected(null) }} onSubmit={issue} />}
    </main>
  )
}

function outcomeLabel(value: string) { return ({ succeeded: '完成', failed: '失败', canceled: '取消' } as Record<string, string>)[value] ?? value }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
