import { useCallback, useEffect, useState } from 'react'
import { CheckCheck, ClipboardCheck, LoaderCircle, RefreshCw, TriangleAlert } from 'lucide-react'
import ReviewTerminalCandidateDialog from './ReviewTerminalCandidateDialog'
import {
  computeConsumerReviewApi,
  type ComputeAttemptConsumerReviewReceipt,
  type ComputeAttemptTerminalCandidateReceipt,
  type ReviewComputeAttemptTerminalCandidateBody,
} from './computeConsumerReviewApi'
import styles from './ComputeConsumerReviewPage.module.css'

export default function ComputeConsumerReviewPage() {
  const [candidates, setCandidates] = useState<ComputeAttemptTerminalCandidateReceipt[]>([])
  const [selected, setSelected] = useState<ComputeAttemptTerminalCandidateReceipt | null>(null)
  const [latestReview, setLatestReview] = useState<ComputeAttemptConsumerReviewReceipt | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [dialogError, setDialogError] = useState('')

  const loadCandidates = useCallback(async () => {
    setLoading(true)
    setError('')
    try {
      setCandidates(await computeConsumerReviewApi.listPending())
    } catch (reason) {
      setError(messageOf(reason, '待验收交付读取失败'))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { void loadCandidates() }, [loadCandidates])

  async function review(body: ReviewComputeAttemptTerminalCandidateBody) {
    if (!selected || busy) return
    setBusy(true)
    setDialogError('')
    try {
      const receipt = await computeConsumerReviewApi.review(selected, body)
      setLatestReview(receipt)
      setSelected(null)
      await loadCandidates()
    } catch (reason) {
      setDialogError(messageOf(reason, '消费者审核证据保存失败'))
    } finally {
      setBusy(false)
    }
  }

  return (
    <main className={styles.page}>
      <header className={styles.pageHeader}>
        <div>
          <span>消费者控制面</span>
          <h1>算力验收</h1>
          <p>Provider 终态候选</p>
        </div>
        <button type="button" onClick={() => void loadCandidates()} disabled={loading} title="刷新待验收交付">
          <RefreshCw size={15} className={loading ? styles.spinning : ''} />刷新
        </button>
      </header>

      {error && <div className={styles.alert} data-tone="error"><TriangleAlert size={15} />{error}</div>}
      {latestReview && (
        <div className={styles.alert} data-tone="success">
          <CheckCheck size={15} />
          已保存 {decisionLabel(latestReview.decision)} 证据 · {shortId(latestReview.consumer_review_id)}
        </div>
      )}

      <section className={styles.queue}>
        <header>
          <div><ClipboardCheck size={17} /><h2>待验收交付</h2></div>
          <span>{candidates.length}</span>
        </header>
        {loading && !candidates.length && <div className={styles.empty}><LoaderCircle size={17} className={styles.spinning} />读取交付候选</div>}
        {!loading && !candidates.length && <div className={styles.empty}>暂无待验收交付</div>}
        <div className={styles.candidateList}>
          {candidates.map((candidate) => (
            <article className={styles.candidate} key={candidate.terminal_candidate_id}>
              <header>
                <div>
                  <b data-outcome={candidate.outcome}>{outcomeLabel(candidate.outcome)}</b>
                  <span>{formatTime(candidate.declared_at)}</span>
                </div>
                <button type="button" onClick={() => { setDialogError(''); setSelected(candidate) }}>审核交付</button>
              </header>
              <div className={styles.facts}>
                <div><span>Job</span><strong>{shortId(candidate.job_id)}</strong></div>
                <div><span>Provider</span><strong>{shortId(candidate.provider_id)}</strong></div>
                <div><span>最终用量序号</span><strong>{candidate.final_usage_sequence_no}</strong></div>
                <div><span>结果工件</span><strong>{candidate.result_artifacts.length}</strong></div>
              </div>
              <div className={styles.summary}>
                <span>{candidate.reason_code}</span>
                <code>{candidate.output_digest ?? candidate.final_cumulative_usage_digest}</code>
              </div>
            </article>
          ))}
        </div>
      </section>

      {selected && (
        <ReviewTerminalCandidateDialog
          candidate={selected}
          busy={busy}
          error={dialogError}
          onClose={() => { if (!busy) setSelected(null) }}
          onSubmit={review}
        />
      )}
    </main>
  )
}

function outcomeLabel(value: string) { return ({ succeeded: '已完成', failed: '执行失败', canceled: '已取消' } as Record<string, string>)[value] ?? value }
function decisionLabel(value: string) { return ({ accepted: '接受', rejected: '拒绝', disputed: '争议' } as Record<string, string>)[value] ?? value }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
