import { useState, type FormEvent } from 'react'
import { LoaderCircle, PackageX, X } from 'lucide-react'
import { type MyComputeOfferView } from './computeOfferApi'
import styles from './OfferDraftActionDialog.module.css'

interface Props { view: MyComputeOfferView; busy: boolean; error: string; onClose: () => void; onSubmit: () => Promise<void> }

export default function RevokeOfferDraftDialog({ view, busy, error, onClose, onSubmit }: Props) {
  const [confirmed, setConfirmed] = useState(false)
  async function submit(event: FormEvent<HTMLFormElement>) { event.preventDefault(); if (confirmed && !busy) await onSubmit() }
  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}><section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="revoke-offer-title">
    <header><div><PackageX size={18} /><h2 id="revoke-offer-title">撤销 Offer 草稿</h2></div><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
    <form onSubmit={(event) => void submit(event)}>{error && <div className={styles.error}>{error}</div>}<div className={styles.identity}><span>{view.offer.sku.sku_id} · v{view.offer.offer_version}</span><code>{view.offer.offer_digest}</code></div><div className={styles.warning}>撤销会追加不可变 revoked 版本。它只关闭尚未发布的意图，不取消 active 供给、预留、履约或资金。</div><label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我确认使用当前版本和摘要撤销该 draft。</span></label><footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.danger} disabled={!confirmed || busy}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在撤销' : '确认撤销'}</button></footer></form>
  </section></div>
}
