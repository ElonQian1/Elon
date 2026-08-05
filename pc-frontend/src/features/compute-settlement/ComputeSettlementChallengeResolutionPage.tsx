import { useCallback, useEffect, useState } from 'react'
import { CircleCheck, Gavel, LoaderCircle, RefreshCw, ShieldAlert } from 'lucide-react'
import ResolveSettlementChallengeDialog from './ResolveSettlementChallengeDialog'
import SettlementChallengeHistoryList from './SettlementChallengeHistoryList'
import {
  computeSettlementChallengeResolutionApi,
  type ComputeSettlementChallengeReceipt,
  type ComputeSettlementChallengeHistoryItem,
  type ComputeSettlementChallengeResolutionReceipt,
  type ResolveComputeSettlementChallengeBody,
} from './computeSettlementChallengeResolutionApi'
import styles from './ComputeSettlementChallengePage.module.css'

export default function ComputeSettlementChallengeResolutionPage() {
  const [challenges, setChallenges] = useState<ComputeSettlementChallengeReceipt[]>([])
  const [history, setHistory] = useState<ComputeSettlementChallengeHistoryItem[]>([])
  const [selected, setSelected] = useState<ComputeSettlementChallengeReceipt | null>(null)
  const [latest, setLatest] = useState<ComputeSettlementChallengeResolutionReceipt | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [dialogError, setDialogError] = useState('')

  const loadChallenges = useCallback(async () => {
    setLoading(true)
    setError('')
    try {
      const [open, nextHistory] = await Promise.all([
        computeSettlementChallengeResolutionApi.listAdminOpen(),
        computeSettlementChallengeResolutionApi.listAdminHistory(),
      ])
      setChallenges(open)
      setHistory(nextHistory)
    } catch (reason) {
      setError(messageOf(reason, '待裁决结算申诉读取失败'))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { void loadChallenges() }, [loadChallenges])

  async function resolve(body: ResolveComputeSettlementChallengeBody) {
    if (!selected || busy) return
    setBusy(true)
    setDialogError('')
    try {
      const receipt = await computeSettlementChallengeResolutionApi.resolve(selected, body)
      setLatest(receipt)
      setSelected(null)
      await loadChallenges()
    } catch (reason) {
      setDialogError(messageOf(reason, '结算申诉裁决失败'))
    } finally {
      setBusy(false)
    }
  }

  return (
    <main className={styles.page}>
      <header className={styles.pageHeader}>
        <div><span>管理员争议控制面</span><h1>申诉裁决</h1><p>为 open 结算申诉登记唯一终态</p></div>
        <button type="button" onClick={() => void loadChallenges()} disabled={loading} title="刷新待裁决申诉"><RefreshCw size={15} className={loading ? styles.spinning : ''} />刷新</button>
      </header>
      <div className={styles.warning}><ShieldAlert size={16} /><span>接受或驳回只登记 v197 决议，不会退款、纠正或移动余额；accepted 必须继续完成独立 v199 纠正。</span></div>
      {error && <div className={styles.alert} data-tone="error"><ShieldAlert size={15} />{error}</div>}
      {latest && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />已登记 {latest.action} 决议 · {shortId(latest.resolution_id)}</div>}

      <section className={styles.queue}>
        <header><div><Gavel size={17} /><h2>待裁决申诉</h2></div><span>{challenges.length}</span></header>
        {loading && !challenges.length && <div className={styles.empty}><LoaderCircle size={17} className={styles.spinning} />读取候选</div>}
        {!loading && !challenges.length && <div className={styles.empty}>暂无 open 结算申诉</div>}
        <div className={styles.candidateList}>
          {challenges.map((challenge) => (
            <article className={styles.candidate} key={challenge.challenge_id}>
              <header>
                <div><b>{challenge.reason_code}</b><span>{formatTime(challenge.opened_at)}</span></div>
                <button type="button" onClick={() => { setDialogError(''); setSelected(challenge) }}>核对并裁决</button>
              </header>
              <div className={styles.facts}>
                <div><span>消费者</span><strong>{shortId(challenge.consumer_account_id)}</strong></div>
                <div><span>Provider</span><strong>{shortId(challenge.provider_account_id)}</strong></div>
                <div><span>证据引用</span><strong>{challenge.evidence_refs.length}</strong></div>
                <div><span>挑战截止</span><strong>{formatTime(challenge.challenge_deadline)}</strong></div>
              </div>
              <p className={styles.summary}>{challenge.summary}</p>
              <code className={styles.digest}>{challenge.event_digest}</code>
            </article>
          ))}
        </div>
      </section>
      <SettlementChallengeHistoryList items={history} loading={loading} />
      {selected && <ResolveSettlementChallengeDialog challenge={selected} busy={busy} error={dialogError} onClose={() => { if (!busy) setSelected(null) }} onSubmit={resolve} />}
    </main>
  )
}

function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
