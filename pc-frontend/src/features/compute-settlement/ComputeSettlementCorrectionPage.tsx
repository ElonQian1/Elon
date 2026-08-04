import { useCallback, useEffect, useState } from 'react'
import { CircleCheck, CircleMinus, LoaderCircle, RefreshCw, ShieldAlert } from 'lucide-react'
import CorrectSettlementDialog from './CorrectSettlementDialog'
import {
  computeSettlementCorrectionApi,
  type ComputePendingSettlementCorrectionCandidate,
  type ComputeSettlementCorrectionReceipt,
  type CorrectComputeAttemptSettlementBody,
} from './computeSettlementCorrectionApi'
import { formatFen, formatMicros } from './settlementFormatting'
import styles from './ComputeSettlementChallengePage.module.css'

export default function ComputeSettlementCorrectionPage() {
  const [candidates, setCandidates] = useState<ComputePendingSettlementCorrectionCandidate[]>([])
  const [selected, setSelected] = useState<ComputePendingSettlementCorrectionCandidate | null>(null)
  const [latest, setLatest] = useState<ComputeSettlementCorrectionReceipt | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [dialogError, setDialogError] = useState('')

  const loadCandidates = useCallback(async () => {
    setLoading(true)
    setError('')
    try {
      setCandidates(await computeSettlementCorrectionApi.listPending())
    } catch (reason) {
      setError(messageOf(reason, '待纠正结算读取失败'))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { void loadCandidates() }, [loadCandidates])

  async function correct(body: CorrectComputeAttemptSettlementBody) {
    if (!selected || busy) return
    setBusy(true)
    setDialogError('')
    try {
      const receipt = await computeSettlementCorrectionApi.correct(selected, body)
      setLatest(receipt)
      setSelected(null)
      await loadCandidates()
    } catch (reason) {
      setDialogError(messageOf(reason, '结算纠正执行失败'))
    } finally {
      setBusy(false)
    }
  }

  return (
    <main className={styles.page}>
      <header className={styles.pageHeader}>
        <div><span>管理员资金纠正控制面</span><h1>结算纠正</h1><p>处理 accepted 挑战的向下金额修正</p></div>
        <button type="button" onClick={() => void loadCandidates()} disabled={loading} title="刷新待纠正结算"><RefreshCw size={15} className={loading ? styles.spinning : ''} />刷新</button>
      </header>
      <div className={styles.warning}><ShieldAlert size={16} /><span>v199 会实际增加消费者平台内余额并冲减 Provider/平台 pending，但不会发起或证明银行、钱包或链上退款。</span></div>
      {error && <div className={styles.alert} data-tone="error"><ShieldAlert size={15} />{error}</div>}
      {latest && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />已生成 Correction Receipt · {shortId(latest.correction_id)}</div>}

      <section className={styles.queue}>
        <header><div><CircleMinus size={17} /><h2>accepted 待纠正</h2></div><span>{candidates.length}</span></header>
        {loading && !candidates.length && <div className={styles.empty}><LoaderCircle size={17} className={styles.spinning} />读取候选</div>}
        {!loading && !candidates.length && <div className={styles.empty}>暂无待纠正 accepted 挑战</div>}
        <div className={styles.candidateList}>
          {candidates.map((candidate) => {
            const receipt = candidate.settlement
            return (
              <article className={styles.candidate} key={candidate.resolution.resolution_id}>
                <header>
                  <div><b>accepted</b><span>{formatTime(candidate.resolution.resolved_at)}</span></div>
                  <button type="button" onClick={() => { setDialogError(''); setSelected(candidate) }}>核对并纠正</button>
                </header>
                <div className={styles.facts}>
                  <div><span>消费者原扣结</span><strong>{formatFen(receipt.consumer_charged_fen)}</strong></div>
                  <div><span>Provider 原 pending</span><strong>{formatMicros(receipt.settlement.amounts.provider_payable_micros)}</strong></div>
                  <div><span>平台原 pending</span><strong>{formatMicros(receipt.settlement.amounts.platform_margin_micros)}</strong></div>
                  <div><span>挑战原因</span><strong>{candidate.challenge.reason_code}</strong></div>
                </div>
                <p className={styles.summary}>{candidate.challenge.summary}</p>
                <code className={styles.digest}>{candidate.resolution.event_digest}</code>
              </article>
            )
          })}
        </div>
      </section>
      {selected && <CorrectSettlementDialog candidate={selected} busy={busy} error={dialogError} onClose={() => { if (!busy) setSelected(null) }} onSubmit={correct} />}
    </main>
  )
}

function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
