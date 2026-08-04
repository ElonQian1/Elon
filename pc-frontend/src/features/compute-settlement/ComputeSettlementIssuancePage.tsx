import { useCallback, useEffect, useState } from 'react'
import { Banknote, CircleCheck, LoaderCircle, RefreshCw, TriangleAlert } from 'lucide-react'
import IssueSettlementReceiptDialog from './IssueSettlementReceiptDialog'
import {
  computeSettlementIssuanceApi,
  type ComputeAttemptSettlementReceipt,
  type ComputePendingAttemptSettlementCandidate,
  type SettleComputeAttemptBody,
} from './computeSettlementIssuanceApi'
import { formatFen, formatMicros } from './settlementFormatting'
import styles from './ComputeSettlementIssuancePage.module.css'

export default function ComputeSettlementIssuancePage() {
  const [candidates, setCandidates] = useState<ComputePendingAttemptSettlementCandidate[]>([])
  const [selected, setSelected] = useState<ComputePendingAttemptSettlementCandidate | null>(null)
  const [latest, setLatest] = useState<ComputeAttemptSettlementReceipt | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [dialogError, setDialogError] = useState('')

  const loadCandidates = useCallback(async () => {
    setLoading(true)
    setError('')
    try {
      setCandidates(await computeSettlementIssuanceApi.listPending())
    } catch (reason) {
      setError(messageOf(reason, '待结算回执读取失败'))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { void loadCandidates() }, [loadCandidates])

  async function settle(body: SettleComputeAttemptBody) {
    if (!selected || busy) return
    setBusy(true)
    setDialogError('')
    try {
      const receipt = await computeSettlementIssuanceApi.settle(selected, body)
      setLatest(receipt)
      setSelected(null)
      await loadCandidates()
    } catch (reason) {
      setDialogError(messageOf(reason, 'Settlement Receipt 生成失败'))
    } finally {
      setBusy(false)
    }
  }

  return (
    <main className={styles.page}>
      <header className={styles.pageHeader}>
        <div><span>管理员资金控制面</span><h1>待结算回执</h1><p>结清预授权并登记 Provider pending 收益</p></div>
        <button type="button" onClick={() => void loadCandidates()} disabled={loading} title="刷新待结算回执"><RefreshCw size={15} className={loading ? styles.spinning : ''} />刷新</button>
      </header>

      <div className={styles.warning}><TriangleAlert size={16} /><span>本操作会实际扣结平台内消费者预授权并登记待结算收益，但不会发起银行、钱包或链上付款。</span></div>
      {error && <div className={styles.alert} data-tone="error"><TriangleAlert size={15} />{error}</div>}
      {latest && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />已生成 Settlement Receipt · {shortId(latest.settlement.settlement_receipt_id)}</div>}

      <section className={styles.queue}>
        <header><div><Banknote size={17} /><h2>待结算可信终态</h2></div><span>{candidates.length}</span></header>
        {loading && !candidates.length && <div className={styles.empty}><LoaderCircle size={17} className={styles.spinning} />读取候选</div>}
        {!loading && !candidates.length && <div className={styles.empty}>暂无待结算 Attempt</div>}
        <div className={styles.candidateList}>
          {candidates.map((candidate) => {
            const preview = candidate.preview
            return (
              <article className={styles.candidate} key={candidate.finalization.finalization_id}>
                <header>
                  <div><b>{candidate.finalization.outcome}</b><span>{formatTime(candidate.finalization.finalized_at)}</span></div>
                  <button type="button" onClick={() => { setDialogError(''); setSelected(candidate) }}>核对并结算</button>
                </header>
                <div className={styles.facts}>
                  <div><span>消费者预授权</span><strong>{formatFen(preview.budget_reserved_fen)}</strong></div>
                  <div><span>实际扣结</span><strong>{formatFen(preview.consumer_charge_fen)}</strong></div>
                  <div><span>退回消费者</span><strong>{formatFen(preview.consumer_refund_fen)}</strong></div>
                  <div><span>Provider pending</span><strong>{formatMicros(preview.amounts.provider_payable_micros)}</strong></div>
                </div>
                <div className={styles.effects}><span>Job → settled</span><span>余额 → pending</span><span>外部付款 → none</span></div>
                <code className={styles.digest}>{candidate.finalization.event_digest}</code>
              </article>
            )
          })}
        </div>
      </section>

      {selected && <IssueSettlementReceiptDialog candidate={selected} busy={busy} error={dialogError} onClose={() => { if (!busy) setSelected(null) }} onSubmit={settle} />}
    </main>
  )
}

function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
