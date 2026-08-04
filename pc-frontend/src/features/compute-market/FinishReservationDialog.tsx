import { useState, type FormEvent } from 'react'
import { Clock3, LoaderCircle, RotateCcw, X } from 'lucide-react'
import { type ComputeReservationReceipt } from './computeMarketApi'
import styles from './FinishReservationDialog.module.css'

interface Props {
  receipt: ComputeReservationReceipt
  action: 'release' | 'expire'
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (idempotencyKey: string) => Promise<void>
}

export default function FinishReservationDialog({ receipt, action, busy, error, onClose, onSubmit }: Props) {
  const [idempotencyKey] = useState(() => createKey(action, receipt.reservation.reservation_id))
  const [confirmed, setConfirmed] = useState(false)
  const isRelease = action === 'release'
  async function submit(event: FormEvent<HTMLFormElement>) { event.preventDefault(); if (confirmed && !busy) await onSubmit(idempotencyKey) }

  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}><section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="finish-reservation-title"><header><div>{isRelease ? <RotateCcw size={18} /> : <Clock3 size={18} />}<h2 id="finish-reservation-title">{isRelease ? '释放未执行预留' : '确认预留到期'}</h2></div><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header><form onSubmit={(event) => void submit(event)}>{error && <div className={styles.error}>{error}</div>}<div className={styles.facts}><div><span>Reservation</span><strong>{receipt.reservation.reservation_id}</strong></div><div><span>当前版本</span><strong>{receipt.revision}</strong></div><div><span>到期时间</span><strong>{formatTime(receipt.reservation.expires_at)}</strong></div><div><span>状态</span><strong>{receipt.reservation.status}</strong></div></div><div className={styles.boundary}>{isRelease ? '确认后，后端只会在 Job 尚未开始执行且精确版本仍有效时，原子释放容量、退回冻结余额并取消 Job。' : '确认后，后端只会在预留已达到到期边界且 Job 尚未开始执行时，原子标记到期、释放容量并退回冻结余额。'}</div><label><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对 Reservation 当前版本与摘要，确认执行{isRelease ? '释放' : '到期'}操作。</span></label><code>{receipt.reservation_digest}</code><footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} data-tone={isRelease ? 'release' : 'expire'} disabled={!confirmed || busy}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在处理' : isRelease ? '确认释放' : '确认到期'}</button></footer></form></section></div>
}

function createKey(action: string, reservationId: string) { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return `pc-compute-${action}:${reservationId}:${nonce}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
