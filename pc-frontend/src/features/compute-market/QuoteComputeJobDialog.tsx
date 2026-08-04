import { useState, type FormEvent } from 'react'
import { LoaderCircle, LockKeyhole, X } from 'lucide-react'
import { type ComputeQuoteCandidate } from './computeMarketApi'
import styles from './QuoteComputeJobDialog.module.css'

interface Props { candidate: ComputeQuoteCandidate; busy: boolean; error: string; onClose: () => void; onSubmit: () => Promise<void> }

export default function QuoteComputeJobDialog({ candidate, busy, error, onClose, onSubmit }: Props) {
  const [confirmed, setConfirmed] = useState(false)
  async function submit(event: FormEvent<HTMLFormElement>) { event.preventDefault(); if (confirmed && !busy) await onSubmit() }
  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}><section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="quote-job-title"><header><div><LockKeyhole size={18} /><h2 id="quote-job-title">锁定报价</h2></div><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header><form onSubmit={(event) => void submit(event)}>{error && <div className={styles.error}>{error}</div>}<div className={styles.facts}><div><span>Provider</span><strong>{candidate.provider.display_name}</strong></div><div><span>最高金额</span><strong>{formatAmount(candidate.price_snapshot.consumer_max_amount_micros, candidate.price_snapshot.currency)}</strong></div><div><span>信任层</span><strong>{candidate.provider.trust_tier}</strong></div><div><span>失效时间</span><strong>{formatTime(candidate.price_snapshot.expires_at)}</strong></div></div><code>{candidate.price_snapshot.snapshot_digest}</code><div className={styles.boundary}>锁价只把 Job 绑定到当前 Offer 与不可变快照；不会冻结余额、持有容量或创建 Reservation。</div><label><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对 Provider、金额、有效期和快照摘要，确认锁定该报价。</span></label><footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!confirmed || busy}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在锁定' : '确认锁价'}</button></footer></form></section></div>
}

function formatAmount(value: number, currency: string) { return `${currency} ${(value / 1_000_000).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 6 })}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
