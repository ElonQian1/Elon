import { useMemo, useState, type FormEvent } from 'react'
import { LoaderCircle, Radar, X } from 'lucide-react'
import {
  type ComputeObservationSource,
  type ComputeObservedOutcome,
  type ComputePendingPlatformObservationCandidate,
  type ObserveComputeAttemptTerminalCandidateBody,
} from './computePlatformObservationApi'
import styles from './ComputePlatformObservationPage.module.css'

interface Props {
  candidate: ComputePendingPlatformObservationCandidate
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: ObserveComputeAttemptTerminalCandidateBody) => Promise<void>
}

export default function ObserveTerminalCandidateDialog({ candidate, busy, error, onClose, onSubmit }: Props) {
  const terminal = candidate.terminal_candidate
  const [source, setSource] = useState<ComputeObservationSource>('control_plane')
  const [outcome, setOutcome] = useState<ComputeObservedOutcome>(terminal.outcome)
  const [observerRef, setObserverRef] = useState('')
  const [evidenceText, setEvidenceText] = useState('')
  const [quantities, setQuantities] = useState<Record<string, number>>(() => Object.fromEntries(candidate.provider_usage.cumulative_declared_usage.map((reading) => [reading.meter, reading.quantity])))
  const [idempotencyKey] = useState(createKey)
  const [confirmed, setConfirmed] = useState(false)
  const evidence = useMemo(() => evidenceText.split(/\r?\n/).map((value) => value.trim()).filter(Boolean), [evidenceText])
  const evidenceValid = evidence.length > 0 && evidence.length <= 16 && new Set(evidence).size === evidence.length && evidence.every((value) => value.length <= 1000)
  const metersValid = candidate.provider_usage.cumulative_declared_usage.length > 0 && candidate.provider_usage.cumulative_declared_usage.every((reading) => Number.isSafeInteger(quantities[reading.meter]) && quantities[reading.meter] >= 0)
  const valid = observerRef.trim().length > 0 && observerRef.trim().length <= 1000 && evidenceValid && metersValid && confirmed && !busy

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (!valid) return
    await onSubmit({
      expected_terminal_candidate_id: terminal.terminal_candidate_id,
      expected_terminal_candidate_event_digest: terminal.event_digest,
      observation_source: source,
      observer_ref: observerRef.trim(),
      observed_outcome: outcome,
      cumulative_observed_usage: candidate.provider_usage.cumulative_declared_usage.map((reading) => ({ meter: reading.meter, cumulative_quantity: quantities[reading.meter] })),
      evidence_refs: [...evidence].sort(),
      idempotency_key: idempotencyKey,
      confirm_platform_observation_only: true,
    })
  }

  return (
    <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="platform-observation-title">
        <header><div><Radar size={18} /><h2 id="platform-observation-title">登记平台观测</h2></div><button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
        <form onSubmit={(event) => void submit(event)}>
          {error && <div className={styles.dialogError}>{error}</div>}
          <div className={styles.sourceTabs} aria-label="观测来源">
            {(['control_plane', 'transport_gateway', 'server_metering'] as ComputeObservationSource[]).map((value) => <button type="button" data-active={source === value} key={value} onClick={() => { setSource(value); setConfirmed(false) }}>{sourceLabel(value)}</button>)}
          </div>
          <div className={styles.outcomeTabs} aria-label="平台观测结果">
            {(['succeeded', 'failed', 'canceled', 'indeterminate'] as ComputeObservedOutcome[]).map((value) => <button type="button" data-active={outcome === value} key={value} onClick={() => { setOutcome(value); setConfirmed(false) }}>{outcomeLabel(value)}</button>)}
          </div>
          <section className={styles.meterEditor}>
            <header><strong>累计 Meter</strong><span>Provider 声明 / 平台观测</span></header>
            {candidate.provider_usage.cumulative_declared_usage.map((reading) => (
              <label key={reading.meter}>
                <span>{reading.meter}</span><code>{reading.quantity}</code>
                <input type="number" min="0" step="1" value={quantities[reading.meter]} onChange={(event) => { setQuantities((current) => ({ ...current, [reading.meter]: Number(event.target.value) })); setConfirmed(false) }} />
              </label>
            ))}
          </section>
          <div className={styles.formGrid}>
            <label><span>观测系统引用</span><input value={observerRef} maxLength={1000} onChange={(event) => { setObserverRef(event.target.value); setConfirmed(false) }} placeholder="observation://control-plane/..." /></label>
            <label data-wide="true"><span>外部证据引用（每行一条）</span><textarea rows={4} value={evidenceText} onChange={(event) => { setEvidenceText(event.target.value); setConfirmed(false) }} /></label>
          </div>
          {!metersValid && <div className={styles.validation}>每个 meter 必须保留且填写非负安全整数。</div>}
          {!evidenceValid && <div className={styles.validation}>至少填写一条唯一证据引用，总数最多 16 条，每条最多 1000 字符。</div>}
          <div className={styles.boundary}>平台观测只是 Verification 的输入，不是可信用量、执行回执或结算授权。</div>
          <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对候选、meter 和证据引用，并确认只登记第一份平台观测。</span></label>
          <code className={styles.eventDigest}>{terminal.event_digest}</code>
          <footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!valid}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在保存' : '保存观测'}</button></footer>
        </form>
      </section>
    </div>
  )
}

function sourceLabel(value: ComputeObservationSource) { return ({ control_plane: '控制面', transport_gateway: '传输网关', server_metering: '服务端计量' })[value] }
function outcomeLabel(value: ComputeObservedOutcome) { return ({ succeeded: '完成', failed: '失败', canceled: '取消', indeterminate: '待定' })[value] }
function createKey() { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return `pc-compute-platform-observation:${nonce}` }
