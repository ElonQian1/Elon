import { useCallback, useEffect, useState } from 'react'
import { CircleCheck, LoaderCircle, LockKeyhole, RefreshCw, TriangleAlert } from 'lucide-react'
import FinalizeAttemptDialog from './FinalizeAttemptDialog'
import {
  computeAttemptFinalizationApi,
  type ComputeAttemptFinalizationReceipt,
  type ComputePendingAttemptFinalizationCandidate,
  type FinalizeComputeAttemptBody,
} from './computeAttemptFinalizationApi'
import styles from './ComputeAttemptFinalizationPage.module.css'

export default function ComputeAttemptFinalizationPage() {
  const [candidates, setCandidates] = useState<ComputePendingAttemptFinalizationCandidate[]>([])
  const [selected, setSelected] = useState<ComputePendingAttemptFinalizationCandidate | null>(null)
  const [latest, setLatest] = useState<ComputeAttemptFinalizationReceipt | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [dialogError, setDialogError] = useState('')

  const loadCandidates = useCallback(async () => {
    setLoading(true)
    setError('')
    try {
      setCandidates(await computeAttemptFinalizationApi.listPending())
    } catch (reason) {
      setError(messageOf(reason, '待收口回执读取失败'))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { void loadCandidates() }, [loadCandidates])

  async function finalize(body: FinalizeComputeAttemptBody) {
    if (!selected || busy) return
    setBusy(true)
    setDialogError('')
    try {
      const receipt = await computeAttemptFinalizationApi.finalize(selected, body)
      setLatest(receipt)
      setSelected(null)
      await loadCandidates()
    } catch (reason) {
      setDialogError(messageOf(reason, '可信终态收口失败'))
    } finally {
      setBusy(false)
    }
  }

  return (
    <main className={styles.page}>
      <header className={styles.pageHeader}>
        <div><span>管理员控制面</span><h1>可信终态</h1><p>执行状态与容量账本的原子收口</p></div>
        <button type="button" onClick={() => void loadCandidates()} disabled={loading} title="刷新待收口回执"><RefreshCw size={15} className={loading ? styles.spinning : ''} />刷新</button>
      </header>

      <div className={styles.warning}><TriangleAlert size={16} /><span>该操作会推进任务状态并消费或归还容量。资金预授权与收益结算保持不变。</span></div>
      {error && <div className={styles.alert} data-tone="error"><TriangleAlert size={15} />{error}</div>}
      {latest && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />已应用可信终态 · {shortId(latest.finalization_id)}</div>}

      <section className={styles.queue}>
        <header><div><LockKeyhole size={17} /><h2>待收口执行回执</h2></div><span>{candidates.length}</span></header>
        {loading && !candidates.length && <div className={styles.empty}><LoaderCircle size={17} className={styles.spinning} />读取候选</div>}
        {!loading && !candidates.length && <div className={styles.empty}>暂无待收口 Execution Receipt</div>}
        <div className={styles.candidateList}>
          {candidates.map((candidate) => {
            const receipt = candidate.execution_receipt.receipt
            return (
              <article className={styles.candidate} key={receipt.receipt_id}>
                <header>
                  <div><b>{outcomeLabel(receipt.execution_status)}</b><span>{formatTime(candidate.execution_receipt.issued_at)}</span></div>
                  <button type="button" onClick={() => { setDialogError(''); setSelected(candidate) }}>核对并收口</button>
                </header>
                <div className={styles.facts}>
                  <div><span>Job</span><strong>{shortId(receipt.job_id)}</strong></div>
                  <div><span>Provider</span><strong>{shortId(receipt.provider_id)}</strong></div>
                  <div><span>Lease 修订</span><strong>r{candidate.expected_lease.revision}</strong></div>
                  <div><span>Fencing</span><strong>g{candidate.expected_fencing_generation}</strong></div>
                </div>
                <div className={styles.effects}>
                  <span>Lease → terminal</span><span>Job → verification_pending</span>
                  <span>Reservation → consumed</span><span>Settlement → pending</span>
                </div>
                <code className={styles.digest}>{receipt.receipt_digest}</code>
              </article>
            )
          })}
        </div>
      </section>

      {selected && <FinalizeAttemptDialog candidate={selected} busy={busy} error={dialogError} onClose={() => { if (!busy) setSelected(null) }} onSubmit={finalize} />}
    </main>
  )
}

function outcomeLabel(value: string) { return ({ succeeded: '完成', failed: '失败', canceled: '取消' } as Record<string, string>)[value] ?? value }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
