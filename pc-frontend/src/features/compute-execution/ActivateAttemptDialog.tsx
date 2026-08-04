import { useState, type FormEvent, type ReactNode } from 'react'
import { LoaderCircle, Play, X } from 'lucide-react'
import { type ComputeReservationReceipt } from '../compute-market/computeMarketApi'
import { type ActivateComputeAttemptBody } from './computeExecutionApi'
import styles from './ComputeExecutionDialog.module.css'

interface Props {
  candidate: ComputeReservationReceipt
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: ActivateComputeAttemptBody) => Promise<void>
}

export default function ActivateAttemptDialog({ candidate, busy, error, onClose, onSubmit }: Props) {
  const [identity] = useState(createIdentity)
  const boundary = new Date(candidate.reservation.expires_at).getTime()
  const [executorId, setExecutorId] = useState('')
  const [shardId, setShardId] = useState('')
  const [acceptanceRef, setAcceptanceRef] = useState('')
  const [credentialRef, setCredentialRef] = useState('')
  const [credentialHint, setCredentialHint] = useState('')
  const [hardDeadline, setHardDeadline] = useState(() => localValue(new Date(boundary)))
  const [leaseExpiry, setLeaseExpiry] = useState(() => localValue(new Date(Math.min(Date.now() + 10 * 60_000, boundary - 60_000))))
  const [accepted, setAccepted] = useState(false)
  const [referenceOnly, setReferenceOnly] = useState(false)
  const expiryMs = new Date(leaseExpiry).getTime()
  const hardMs = new Date(hardDeadline).getTime()
  const validTimes = Number.isFinite(expiryMs) && Number.isFinite(hardMs) && expiryMs > Date.now() && hardMs > expiryMs && hardMs <= boundary
  const valid = executorId.trim() && acceptanceRef.trim() && credentialRef.trim() && credentialHint.trim() && validTimes && accepted && referenceOnly && !busy

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!valid) return
    await onSubmit({
      lease_id: identity.leaseId, reservation_id: candidate.reservation.reservation_id,
      executor_id: executorId.trim(), shard_id: shardId.trim() || null, attempt_no: 1, fencing_generation: 1,
      executor_acceptance_ref: acceptanceRef.trim(), lease_credential_ref: credentialRef.trim(), lease_credential_hint: credentialHint.trim(),
      expected_job_revision: candidate.reservation.job.job_revision, expected_job_digest: candidate.reservation.job.job_digest,
      expected_reservation_revision: candidate.revision, expected_reservation_digest: candidate.reservation_digest,
      expected_claim_revision: candidate.reservation.capacity_claim.claim_revision, expected_claim_digest: candidate.reservation.capacity_claim.claim_digest,
      expires_at: new Date(leaseExpiry).toISOString(), hard_deadline_at: new Date(hardDeadline).toISOString(),
      idempotency_key: identity.idempotencyKey, confirm_executor_accepted: true,
    })
  }

  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}><section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="activate-attempt-title"><header><div><Play size={18} /><h2 id="activate-attempt-title">登记首次 Attempt 激活</h2></div><button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header><form onSubmit={(event) => void submit(event)}>{error && <div className={styles.error}>{error}</div>}<div className={styles.facts}><div><span>任务</span><strong>{candidate.reservation.price_snapshot.sku.task_kind}</strong></div><div><span>Reservation</span><strong>{shortId(candidate.reservation.reservation_id)}</strong></div><div><span>预算预授权</span><strong>{formatAmount(candidate.reservation.price_snapshot.consumer_max_amount_micros)}</strong></div></div><div className={styles.grid}><Field label="Executor ID"><input value={executorId} onChange={(event) => { setExecutorId(event.target.value); setAccepted(false) }} /></Field><Field label="Shard ID（可选）"><input value={shardId} onChange={(event) => { setShardId(event.target.value); setAccepted(false) }} /></Field><Field label="执行器接受证明引用" wide><input value={acceptanceRef} onChange={(event) => { setAcceptanceRef(event.target.value); setAccepted(false) }} placeholder="receipt://executor/accepted/..." /></Field><Field label="Lease 凭据引用"><input value={credentialRef} onChange={(event) => { setCredentialRef(event.target.value); setReferenceOnly(false) }} placeholder="vault://lease/..." /></Field><Field label="凭据提示"><input value={credentialHint} onChange={(event) => { setCredentialHint(event.target.value); setReferenceOnly(false) }} placeholder="不含密钥正文" /></Field><Field label="Lease 软期限"><input type="datetime-local" value={leaseExpiry} max={hardDeadline} onChange={(event) => { setLeaseExpiry(event.target.value); setAccepted(false) }} /></Field><Field label="不可变硬期限"><input type="datetime-local" value={hardDeadline} max={localValue(new Date(boundary))} onChange={(event) => { setHardDeadline(event.target.value); setAccepted(false) }} /></Field></div>{!validTimes && <div className={styles.error}>软期限必须在未来且早于硬期限，硬期限不得越过 Reservation 到期时间。</div>}<div className={styles.boundary}>该入口只登记“外部执行器已经接受”的事实并把 held 容量激活；不会向节点发送 Start 命令，不会验证证明签名，也不会新增扣款。</div><label className={styles.confirm}><input type="checkbox" checked={referenceOnly} onChange={(event) => setReferenceOnly(event.target.checked)} /><span>凭据字段仅填写保险箱或外部系统引用，不包含 Token、私钥或口令正文。</span></label><label className={styles.confirm}><input type="checkbox" checked={accepted} onChange={(event) => setAccepted(event.target.checked)} /><span>我已在外部系统确认执行器真实接受该任务，并核对当前 Job、Reservation 与 Claim 版本。</span></label><code>{candidate.reservation_digest}</code><footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!valid}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在登记' : '确认激活'}</button></footer></form></section></div>
}

function Field({ label, wide, children }: { label: string; wide?: boolean; children: ReactNode }) { return <label data-wide={wide || undefined}><span>{label}</span>{children}</label> }
function createIdentity() { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return { leaseId: `lease_${nonce}`, idempotencyKey: `pc-compute-attempt:${nonce}` } }
function localValue(date: Date) { return Number.isFinite(date.getTime()) ? new Date(date.getTime() - date.getTimezoneOffset() * 60_000).toISOString().slice(0, 16) : '' }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-7)}` }
function formatAmount(micros: number) { return `CNY ${(micros / 1_000_000).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 6 })}` }
