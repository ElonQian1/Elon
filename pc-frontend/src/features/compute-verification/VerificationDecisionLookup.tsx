import { type FormEvent, useLayoutEffect, useRef, useState } from 'react'
import { CircleCheck, LoaderCircle, RefreshCw, Search, TriangleAlert } from 'lucide-react'
import {
  computeVerificationApi,
  type ValidatedVerificationDecisionRead,
} from './computeVerificationApi'
import styles from './ComputeVerificationPage.module.css'

type LookupState =
  | { status: 'idle' }
  | { status: 'loading'; leaseId: string }
  | { status: 'empty'; leaseId: string }
  | { status: 'error'; leaseId: string }
  | { status: 'success'; receipt: ValidatedVerificationDecisionRead }

export default function VerificationDecisionLookup() {
  const [leaseInput, setLeaseInput] = useState('')
  const [state, setState] = useState<LookupState>({ status: 'idle' })
  const requestGeneration = useRef(0)
  const busy = useRef(false)

  useLayoutEffect(() => {
    requestGeneration.current += 1
    busy.current = false
    setState({ status: 'idle' })
    return () => {
      requestGeneration.current += 1
      busy.current = false
    }
  }, [leaseInput])

  async function readRetained() {
    const requestedLeaseId = leaseInput
    if (!requestedLeaseId || busy.current) return
    busy.current = true
    const generation = ++requestGeneration.current
    setState({ status: 'loading', leaseId: requestedLeaseId })
    try {
      const receipt = await computeVerificationApi.readRetained(requestedLeaseId)
      if (generation !== requestGeneration.current) return
      setState({ status: 'success', receipt })
    } catch (reason) {
      if (generation !== requestGeneration.current) return
      setState(
        statusOf(reason) === 404
          ? { status: 'empty', leaseId: requestedLeaseId }
          : { status: 'error', leaseId: requestedLeaseId },
      )
    } finally {
      if (generation === requestGeneration.current) busy.current = false
    }
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    void readRetained()
  }

  const loading = state.status === 'loading'

  return (
    <section className={styles.lookup} aria-labelledby="verification-decision-lookup-title">
      <header>
        <div>
          <Search size={17} />
          <div>
            <h2 id="verification-decision-lookup-title">按 Lease 读取历史决定</h2>
            <p>独立读取 retained v192；accepted、rejected、disputed 均可见，不要求 v193 或 Carrier。</p>
          </div>
        </div>
      </header>

      <form className={styles.lookupForm} onSubmit={submit} aria-busy={loading}>
        <label htmlFor="verification-lease-id">Attempt Lease ID</label>
        <div>
          <input
            id="verification-lease-id"
            value={leaseInput}
            onChange={(event) => setLeaseInput(event.target.value)}
            placeholder="输入完整 Lease ID"
            autoComplete="off"
          />
          <button type="submit" disabled={loading || !leaseInput}>
            {loading ? <LoaderCircle size={14} className={styles.spinning} /> : <Search size={14} />}
            {loading ? '读取中' : '读取决定'}
          </button>
        </div>
      </form>

      {state.status === 'idle' && (
        <p className={styles.lookupHint}>读取只会重新审计历史证据，不会写入、重放或推进任何状态。</p>
      )}
      {state.status === 'loading' && (
        <p className={styles.lookupHint} role="status">
          正在读取 Lease <code title={state.leaseId}>{state.leaseId}</code> 的 retained v192…
        </p>
      )}
      {state.status === 'empty' && (
        <div className={styles.lookupEmpty} role="status">
          <span>该 Lease 没有可读的历史 Verification 决定。</span>
          <code title={state.leaseId}>{state.leaseId}</code>
        </div>
      )}
      {state.status === 'error' && (
        <div className={styles.lookupError} role="alert">
          <TriangleAlert size={15} />
          <span>历史决定读取失败或响应未通过审计，请稍后重试。</span>
          <button type="button" onClick={() => void readRetained()}>
            <RefreshCw size={13} />重试
          </button>
        </div>
      )}
      {state.status === 'success' && <VerificationDecisionEvidence receipt={state.receipt} />}
    </section>
  )
}

function VerificationDecisionEvidence({ receipt }: { receipt: ValidatedVerificationDecisionRead }) {
  return (
    <article className={styles.lookupResult} data-decision={receipt.decision}>
      <header>
        <div>
          <CircleCheck size={15} />
          <strong>{decisionLabel(receipt.decision)} v192 决定</strong>
        </div>
        <span>{formatTime(receipt.decided_at)}</span>
      </header>
      <div className={styles.lookupFacts}>
        <LookupFact label="Lease ID" value={receipt.lease_id} />
        <LookupFact label="Verification ID" value={receipt.verification_decision_id} />
        <LookupFact label="Provider" value={receipt.provider_id} />
        <LookupFact label="Job" value={receipt.job_id} />
        <LookupFact label="Terminal candidate" value={receipt.terminal_candidate_id} />
        <LookupFact label="Platform observation" value={receipt.platform_observation_id} />
      </div>
      <div className={styles.lookupOutcomes}>
        <span>Provider <b data-decision={receipt.candidate_outcome}>{outcomeLabel(receipt.candidate_outcome)}</b></span>
        <span>消费者 <b data-decision={receipt.consumer_decision}>{decisionLabel(receipt.consumer_decision)}</b></span>
        <span>平台 <b data-decision={receipt.observed_outcome}>{outcomeLabel(receipt.observed_outcome)}</b></span>
      </div>
      <div className={styles.lookupMeters}>
        {receipt.verified_usage.map((reading, index) => (
          <span key={reading.meter}>
            {reading.meter}: verified {reading.quantity} / compensable {receipt.compensable_usage[index].quantity}
          </span>
        ))}
      </div>
      <p className={styles.lookupBoundary}>
        {receipt.verification_effect}；Execution Receipt、Lease、Job、容量、Reservation 与预授权均未改变。
      </p>
    </article>
  )
}

function LookupFact({ label, value }: { label: string; value: string }) {
  return <div><span>{label}</span><code title={value}>{value}</code></div>
}

function statusOf(reason: unknown) {
  if (reason && typeof reason === 'object' && 'status' in reason && typeof reason.status === 'number') {
    return reason.status
  }
  return 0
}

function outcomeLabel(value: string) {
  return ({ succeeded: '完成', failed: '失败', canceled: '取消', indeterminate: '待定' } as Record<string, string>)[value] ?? value
}

function decisionLabel(value: string) {
  return ({ accepted: '接受', rejected: '拒绝', disputed: '争议' } as Record<string, string>)[value] ?? value
}

function formatTime(value: string) {
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false })
}
