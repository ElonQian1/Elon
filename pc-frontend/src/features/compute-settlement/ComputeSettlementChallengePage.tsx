import { useCallback, useEffect, useState } from 'react'
import { CircleCheck, LoaderCircle, RefreshCw, Scale, ShieldAlert, Undo2 } from 'lucide-react'
import OpenSettlementChallengeDialog from './OpenSettlementChallengeDialog'
import WithdrawSettlementChallengeDialog from './WithdrawSettlementChallengeDialog'
import {
  computeSettlementChallengeApi,
  type ComputePendingSettlementChallengeCandidate,
  type ComputeSettlementChallengeReceipt,
  type OpenComputeSettlementChallengeBody,
} from './computeSettlementChallengeApi'
import {
  computeSettlementChallengeResolutionApi,
  type ComputeSettlementChallengeResolutionReceipt,
  type WithdrawComputeSettlementChallengeBody,
} from './computeSettlementChallengeResolutionApi'
import { formatFen, formatMicros } from './settlementFormatting'
import styles from './ComputeSettlementChallengePage.module.css'

export default function ComputeSettlementChallengePage() {
  const [candidates, setCandidates] = useState<ComputePendingSettlementChallengeCandidate[]>([])
  const [selected, setSelected] = useState<ComputePendingSettlementChallengeCandidate | null>(null)
  const [openChallenges, setOpenChallenges] = useState<ComputeSettlementChallengeReceipt[]>([])
  const [selectedWithdrawal, setSelectedWithdrawal] = useState<ComputeSettlementChallengeReceipt | null>(null)
  const [latest, setLatest] = useState<ComputeSettlementChallengeReceipt | null>(null)
  const [latestResolution, setLatestResolution] = useState<ComputeSettlementChallengeResolutionReceipt | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [dialogError, setDialogError] = useState('')

  const loadCandidates = useCallback(async () => {
    setLoading(true)
    setError('')
    try {
      const [pending, open] = await Promise.all([
        computeSettlementChallengeApi.listPending(),
        computeSettlementChallengeResolutionApi.listConsumerOpen(),
      ])
      setCandidates(pending)
      setOpenChallenges(open)
    } catch (reason) {
      setError(messageOf(reason, '待申诉结算读取失败'))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { void loadCandidates() }, [loadCandidates])

  async function openChallenge(body: OpenComputeSettlementChallengeBody) {
    if (!selected || busy) return
    setBusy(true)
    setDialogError('')
    try {
      const receipt = await computeSettlementChallengeApi.open(selected, body)
      setLatest(receipt)
      setSelected(null)
      await loadCandidates()
    } catch (reason) {
      setDialogError(messageOf(reason, '结算申诉提交失败'))
    } finally {
      setBusy(false)
    }
  }

  async function withdrawChallenge(body: WithdrawComputeSettlementChallengeBody) {
    if (!selectedWithdrawal || busy) return
    setBusy(true)
    setDialogError('')
    try {
      const receipt = await computeSettlementChallengeResolutionApi.withdraw(selectedWithdrawal, body)
      setLatestResolution(receipt)
      setSelectedWithdrawal(null)
      await loadCandidates()
    } catch (reason) {
      setDialogError(messageOf(reason, '结算申诉撤回失败'))
    } finally {
      setBusy(false)
    }
  }

  return (
    <main className={styles.page}>
      <header className={styles.pageHeader}>
        <div><span>消费者结算控制面</span><h1>结算申诉</h1><p>核对仍在 72 小时窗口内的算力结算</p></div>
        <button type="button" onClick={() => void loadCandidates()} disabled={loading} title="刷新待申诉结算"><RefreshCw size={15} className={loading ? styles.spinning : ''} />刷新</button>
      </header>

      <div className={styles.warning}><ShieldAlert size={16} /><span>提出申诉只会阻断该笔 pending 收益后续释放，不会立即退款、撤销结算或调用银行、钱包和链上支付。</span></div>
      {error && <div className={styles.alert} data-tone="error"><ShieldAlert size={15} />{error}</div>}
      {latest && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />申诉已登记 · {shortId(latest.challenge_id)}</div>}
      {latestResolution && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />申诉已撤回 · {shortId(latestResolution.resolution_id)}</div>}

      <section className={styles.queue}>
        <header><div><Scale size={17} /><h2>可申诉结算</h2></div><span>{candidates.length}</span></header>
        {loading && !candidates.length && <div className={styles.empty}><LoaderCircle size={17} className={styles.spinning} />读取候选</div>}
        {!loading && !candidates.length && <div className={styles.empty}>暂无处于申诉窗口的结算</div>}
        <div className={styles.candidateList}>
          {candidates.map((candidate) => {
            const receipt = candidate.settlement
            return (
              <article className={styles.candidate} key={receipt.settlement.settlement_receipt_id}>
                <header>
                  <div><b>pending</b><span>截止 {formatTime(candidate.challenge_deadline)}</span></div>
                  <button type="button" onClick={() => { setDialogError(''); setSelected(candidate) }}>核对并申诉</button>
                </header>
                <div className={styles.facts}>
                  <div><span>消费者扣结</span><strong>{formatFen(receipt.consumer_charged_fen)}</strong></div>
                  <div><span>已退预授权</span><strong>{formatFen(receipt.consumer_refunded_fen)}</strong></div>
                  <div><span>Provider pending</span><strong>{formatMicros(receipt.settlement.amounts.provider_payable_micros)}</strong></div>
                  <div><span>结算时间</span><strong>{formatTime(receipt.settled_at)}</strong></div>
                </div>
                <div className={styles.effects}><span>余额不变</span><span>释放将阻断</span><span>外部付款 none</span></div>
                <code className={styles.digest}>{receipt.event_digest}</code>
              </article>
            )
          })}
        </div>
      </section>

      <section className={styles.queue}>
        <header><div><Undo2 size={17} /><h2>待处理申诉</h2></div><span>{openChallenges.length}</span></header>
        {!loading && !openChallenges.length && <div className={styles.empty}>暂无可撤回的 open 申诉</div>}
        <div className={styles.candidateList}>
          {openChallenges.map((challenge) => (
            <article className={styles.candidate} key={challenge.challenge_id}>
              <header>
                <div><b>open</b><span>截止 {formatTime(challenge.challenge_deadline)}</span></div>
                <button type="button" onClick={() => { setDialogError(''); setSelectedWithdrawal(challenge) }}>核对并撤回</button>
              </header>
              <div className={styles.facts}>
                <div><span>原因</span><strong>{challenge.reason_code}</strong></div>
                <div><span>提交时间</span><strong>{formatTime(challenge.opened_at)}</strong></div>
                <div><span>消费者</span><strong>{shortId(challenge.consumer_account_id)}</strong></div>
                <div><span>Provider</span><strong>{shortId(challenge.provider_account_id)}</strong></div>
              </div>
              <p className={styles.summary}>{challenge.summary}</p>
              <code className={styles.digest}>{challenge.event_digest}</code>
            </article>
          ))}
        </div>
      </section>

      {selected && <OpenSettlementChallengeDialog candidate={selected} busy={busy} error={dialogError} onClose={() => { if (!busy) setSelected(null) }} onSubmit={openChallenge} />}
      {selectedWithdrawal && <WithdrawSettlementChallengeDialog challenge={selectedWithdrawal} busy={busy} error={dialogError} onClose={() => { if (!busy) setSelectedWithdrawal(null) }} onSubmit={withdrawChallenge} />}
    </main>
  )
}

function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
