import { useCallback, useEffect, useState } from 'react'
import { CircleCheck, LoaderCircle, RefreshCw, Scale, ShieldCheck, TriangleAlert } from 'lucide-react'
import DecideVerificationDialog from './DecideVerificationDialog'
import {
  computeVerificationApi,
  type ComputeAttemptVerificationDecisionReceipt,
  type ComputePendingAttemptVerificationCandidate,
  type DecideComputeAttemptVerificationBody,
} from './computeVerificationApi'
import styles from './ComputeVerificationPage.module.css'

export default function ComputeVerificationPage() {
  const [candidates, setCandidates] = useState<ComputePendingAttemptVerificationCandidate[]>([])
  const [selected, setSelected] = useState<ComputePendingAttemptVerificationCandidate | null>(null)
  const [latest, setLatest] = useState<ComputeAttemptVerificationDecisionReceipt | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [dialogError, setDialogError] = useState('')

  const loadCandidates = useCallback(async () => {
    setLoading(true)
    setError('')
    try {
      setCandidates(await computeVerificationApi.listPending())
    } catch (reason) {
      setError(messageOf(reason, '待验证证据链读取失败'))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { void loadCandidates() }, [loadCandidates])

  async function decide(body: DecideComputeAttemptVerificationBody) {
    if (!selected || busy) return
    setBusy(true)
    setDialogError('')
    try {
      const receipt = await computeVerificationApi.decide(selected, body)
      setLatest(receipt)
      setSelected(null)
      await loadCandidates()
    } catch (reason) {
      setDialogError(messageOf(reason, 'Verification 决定保存失败'))
    } finally {
      setBusy(false)
    }
  }

  return (
    <main className={styles.page}>
      <header className={styles.pageHeader}>
        <div><span>管理员控制面</span><h1>算力验证</h1><p>三方证据与保守计量决定</p></div>
        <button type="button" onClick={() => void loadCandidates()} disabled={loading} title="刷新待验证证据链"><RefreshCw size={15} className={loading ? styles.spinning : ''} />刷新</button>
      </header>

      {error && <div className={styles.alert} data-tone="error"><TriangleAlert size={15} />{error}</div>}
      {latest && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />已保存 {decisionLabel(latest.decision)} 决定 · {shortId(latest.verification_decision_id)}</div>}

      <section className={styles.queue}>
        <header><div><ShieldCheck size={17} /><h2>待验证证据链</h2></div><span>{candidates.length}</span></header>
        {loading && !candidates.length && <div className={styles.empty}><LoaderCircle size={17} className={styles.spinning} />读取证据链</div>}
        {!loading && !candidates.length && <div className={styles.empty}>暂无待验证证据链</div>}
        <div className={styles.candidateList}>
          {candidates.map((item) => {
            const candidate = item.terminal_candidate
            const observation = item.platform_observation
            return (
              <article className={styles.candidate} key={candidate.terminal_candidate_id}>
                <header>
                  <div><Scale size={15} /><strong>{shortId(candidate.job_id)}</strong><span>{formatTime(observation.observed_at)}</span></div>
                  <button type="button" onClick={() => { setDialogError(''); setSelected(item) }}>形成决定</button>
                </header>
                <div className={styles.evidenceStrip}>
                  <div><span>Provider</span><b data-decision={candidate.outcome}>{outcomeLabel(candidate.outcome)}</b></div>
                  <div><span>消费者</span><b data-decision={item.consumer_review.decision}>{decisionLabel(item.consumer_review.decision)}</b></div>
                  <div><span>平台</span><b data-decision={observation.observed_outcome}>{outcomeLabel(observation.observed_outcome)}</b></div>
                </div>
                <div className={styles.facts}>
                  <div><span>Provider</span><strong>{shortId(candidate.provider_id)}</strong></div>
                  <div><span>用量序号</span><strong>{item.provider_usage.sequence_no}</strong></div>
                  <div><span>差异 Meter</span><strong>{observation.variance_meters.length}</strong></div>
                  <div><span>策略</span><strong>保守最小值 v1</strong></div>
                </div>
                <div className={styles.meters}>{item.provider_usage.cumulative_declared_usage.map((reading) => {
                  const observed = observation.cumulative_observed_usage.find((value) => value.meter === reading.meter)?.quantity
                  return <span key={reading.meter}>{reading.meter}: {reading.quantity} / {observed ?? '缺失'}</span>
                })}</div>
              </article>
            )
          })}
        </div>
      </section>

      {selected && <DecideVerificationDialog candidate={selected} busy={busy} error={dialogError} onClose={() => { if (!busy) setSelected(null) }} onSubmit={decide} />}
    </main>
  )
}

function outcomeLabel(value: string) { return ({ succeeded: '完成', failed: '失败', canceled: '取消', indeterminate: '待定' } as Record<string, string>)[value] ?? value }
function decisionLabel(value: string) { return ({ accepted: '接受', rejected: '拒绝', disputed: '争议' } as Record<string, string>)[value] ?? value }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
