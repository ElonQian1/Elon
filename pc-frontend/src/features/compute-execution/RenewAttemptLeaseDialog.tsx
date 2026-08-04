import { useState, type FormEvent } from 'react'
import { HeartPulse, LoaderCircle, X } from 'lucide-react'
import { type ComputeAttemptLeaseStateReceipt, type RenewComputeAttemptLeaseBody } from './computeExecutionApi'
import styles from './ComputeExecutionDialog.module.css'

interface Props {
  state: ComputeAttemptLeaseStateReceipt
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: RenewComputeAttemptLeaseBody) => Promise<void>
}

export default function RenewAttemptLeaseDialog({ state, busy, error, onClose, onSubmit }: Props) {
  const [idempotencyKey] = useState(createKey)
  const [heartbeatRef, setHeartbeatRef] = useState('')
  const currentExpiry = new Date(state.lease.expires_at).getTime()
  const hardDeadline = new Date(state.lease.hard_deadline_at).getTime()
  const [expiresAt, setExpiresAt] = useState(() => localValue(new Date(Math.min(currentExpiry + 10 * 60_000, hardDeadline))))
  const [confirmed, setConfirmed] = useState(false)
  const target = new Date(expiresAt).getTime()
  const validTime = Number.isFinite(target) && target > currentExpiry && target <= hardDeadline && currentExpiry > Date.now()
  const valid = heartbeatRef.trim() && validTime && confirmed && !busy

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!valid) return
    await onSubmit({
      expected_lease_revision: state.lease_revision,
      expected_lease_digest: state.lease_digest,
      expected_fencing_generation: state.lease.fencing_generation,
      executor_heartbeat_ref: heartbeatRef.trim(),
      expires_at: new Date(expiresAt).toISOString(),
      idempotency_key: idempotencyKey,
      confirm_executor_alive: true,
    })
  }

  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}><section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="renew-lease-title"><header><div><HeartPulse size={18} /><h2 id="renew-lease-title">续租 Attempt Lease</h2></div><button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header><form onSubmit={(event) => void submit(event)}>{error && <div className={styles.error}>{error}</div>}<div className={styles.facts}><div><span>当前版本</span><strong>{state.lease_revision}</strong></div><div><span>当前软期限</span><strong>{formatTime(state.lease.expires_at)}</strong></div><div><span>不可变硬期限</span><strong>{formatTime(state.lease.hard_deadline_at)}</strong></div></div><label className={styles.singleField}><span>外部执行器心跳引用</span><input value={heartbeatRef} onChange={(event) => { setHeartbeatRef(event.target.value); setConfirmed(false) }} placeholder="receipt://executor/heartbeat/..." /></label><label className={styles.singleField}><span>新的软期限</span><input type="datetime-local" value={expiresAt} max={localValue(new Date(hardDeadline))} onChange={(event) => { setExpiresAt(event.target.value); setConfirmed(false) }} /></label>{!validTime && <div className={styles.error}>新软期限必须晚于当前期限且不超过硬期限；已经过期的 Lease 不可复活。</div>}<div className={styles.boundary}>续租只记录外部心跳声明并推进 Lease revision；不验证心跳签名，不发送节点命令，也不改变容量、Reservation 或预授权。</div><label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已确认外部执行器仍存活，并核对当前 Lease revision、digest 与 fencing generation。</span></label><code>{state.lease_digest}</code><footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!valid}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在续租' : '确认续租'}</button></footer></form></section></div>
}

function createKey() { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return `pc-compute-lease-renew:${nonce}` }
function localValue(date: Date) { return Number.isFinite(date.getTime()) ? new Date(date.getTime() - date.getTimezoneOffset() * 60_000).toISOString().slice(0, 16) : '' }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
