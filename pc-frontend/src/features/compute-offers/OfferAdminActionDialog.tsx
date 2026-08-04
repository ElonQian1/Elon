import { useState, type FormEvent } from 'react'
import { LoaderCircle, PackageCheck, X } from 'lucide-react'
import { type MyComputeOfferView } from '../compute-supply/computeOfferApi'
import { type ComputeOfferAdminAction } from './computeOfferAdminApi'
import styles from './OfferAdminActionDialog.module.css'

interface Props {
  view: MyComputeOfferView
  action: ComputeOfferAdminAction
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (reason: string) => Promise<void>
}

export default function OfferAdminActionDialog({ view, action, busy, error, onClose, onSubmit }: Props) {
  const [reason, setReason] = useState('')
  const [confirmed, setConfirmed] = useState(false)
  const requiresReason = action !== 'publish'
  const canSubmit = Boolean((!requiresReason || reason.trim()) && reason.trim().length <= 1000 && confirmed && !busy)
  async function submit(event: FormEvent<HTMLFormElement>) { event.preventDefault(); if (canSubmit) await onSubmit(reason.trim()) }
  const copy = ACTION_COPY[action]
  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}><section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="offer-admin-action-title">
    <header><div><PackageCheck size={18} /><h2 id="offer-admin-action-title">{copy.title}</h2></div><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
    <form onSubmit={(event) => void submit(event)}>{error && <div className={styles.error}>{error}</div>}<div className={styles.identity}><span>{view.offer.sku.sku_id} · {view.offer.status} · v{view.offer.offer_version}</span><code>{view.offer.offer_digest}</code></div><div className={styles.boundary}>{copy.description}</div>{requiresReason && <label className={styles.reason}><span>操作原因</span><textarea value={reason} onChange={(event) => { setReason(event.target.value); setConfirmed(false) }} maxLength={1000} rows={5} required /></label>}<label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对当前 Offer 版本、摘要和依赖边界，确认执行该状态变化。</span></label><footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={action === 'publish' ? styles.primary : styles.danger} disabled={!canSubmit}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在提交' : copy.confirm}</button></footer></form>
  </section></div>
}

const ACTION_COPY: Record<ComputeOfferAdminAction, { title: string; confirm: string; description: string }> = {
  publish: { title: '发布 Offer', confirm: '确认发布', description: '发布会追加 active 版本并保存回执；不会生成 Price Snapshot、移动容量、预留资源或移动资金。' },
  drain: { title: '让 Offer 安全退场', confirm: '开始退场', description: 'draining 会停止进入新的报价候选，但保留已有 Reservation、Attempt 和资金事实。' },
  expire: { title: '终结为已到期', confirm: '确认到期', description: '仅在有效期已结束且不存在 pending/active Reservation 时成功；不会自动取消预留、退款或归还 Claim。' },
  revoke: { title: '终结为已撤销', confirm: '确认撤销', description: '仅对 draining Offer 生效，并要求不存在活动预留；不会改写此前发布和退场回执。' },
}
