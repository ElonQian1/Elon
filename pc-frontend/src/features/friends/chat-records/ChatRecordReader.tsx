import { useCallback, useEffect, useRef, useState } from 'react'
import { ArrowLeft, MoreVertical, X } from 'lucide-react'
import { useAuthStore } from '../../../store/auth'
import { recordPath, recordRequest, type RecordCard, type RecordView } from './recordApi'
import styles from './ChatRecords.module.css'
import { useRecordWindow } from './useRecordWindow'
import { useReaderTabs } from '../../reader/readerTabsStore'
import RecordAsset from './RecordAsset'
import RecordMessage from './RecordMessage'

export default function ChatRecordReader({ card, onClose }: { card: RecordCard; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null), feed = useRef<HTMLDivElement>(null)
  useRecordWindow(dialog)
  const [suspended, setSuspended] = useState(false)
  const readerPresented = useReaderTabs(s => s.presented)
  const openLink = useCallback(() => { dialog.current?.close(); setSuspended(true) }, [])
  useEffect(() => { if (suspended && !readerPresented) { dialog.current?.showModal(); setSuspended(false) } }, [suspended, readerPresented])
  const offsets = useRef(new Map<string, number>())
  const [parent, setParent] = useState<string | null>(null), [view, setView] = useState<RecordView>()
  const [error, setError] = useState(''), [retry, setRetry] = useState(0), [raw, setRaw] = useState(false), [confirmRevoke, setConfirmRevoke] = useState(false)
  const token = useAuthStore(s => s.token), owner = useAuthStore(s => s.user?.id)
  const initialOwner = useRef(owner)
  useEffect(() => { if (owner !== initialOwner.current) onClose() }, [owner, onClose])
  useEffect(() => { dialog.current?.showModal() }, [])
  useEffect(() => {
    const controller = new AbortController(); setView(undefined); setError('')
    recordRequest(recordPath(card), controller.signal).then(v => { if (!controller.signal.aborted) setView(v as RecordView) })
      .catch(e => { if (!controller.signal.aborted) setError(e.message || '读取失败') })
    return () => controller.abort()
  }, [card.record_id, card.group_id, retry, token])
  useEffect(() => { if (feed.current) feed.current.scrollTop = offsets.current.get(parent || '') || 0 }, [parent])
  function move(id: string | null) { offsets.current.set(parent || '', feed.current?.scrollTop || 0); setParent(id); setRaw(false) }
  function back() { if (raw) setRaw(false); else if (parent) move(view?.document.messages.find(m => m.id === parent)?.parent_id || null); else onClose() }
  async function revoke() {
    const controller = new AbortController()
    try { await recordRequest(recordPath(card), controller.signal, false, 'DELETE'); setView(undefined); setError('聊天记录已撤回'); setConfirmRevoke(false) }
    catch (e) { setError(e instanceof Error ? e.message : '撤回失败') }
  }
  const messages = view?.document.messages.filter(m => m.parent_id === parent) || []
  return <dialog ref={dialog} className={styles.reader} aria-label="聊天记录" onCancel={e => { e.preventDefault(); back() }}>
    <header data-record-drag title="拖动窗口，双击居中"><button onClick={back} title="返回" aria-label="返回"><ArrowLeft size={22} /></button><h2>{parent ? '转发的聊天记录' : view?.document.title || card.title}</h2><button onClick={onClose} title="关闭" aria-label="关闭"><X size={22} /></button></header>
    <nav><span>微信导出 · {messages.length} 条</span><details className={styles.options}><summary aria-label="更多"><MoreVertical size={20} /></summary>
      <button onClick={e => { e.currentTarget.closest('details')?.removeAttribute('open'); setRaw(v => !v) }}>{raw ? '返回记录' : '原始文本'}</button>
      {view && view.owner_id === owner && <button onClick={e => { e.currentTarget.closest('details')?.removeAttribute('open'); setConfirmRevoke(v => !v) }}>撤回分享</button>}</details></nav>
    {confirmRevoke && <p className={styles.notice}>撤回后群成员将不能读取此记录。<button onClick={() => void revoke()}>确认撤回</button><button onClick={() => setConfirmRevoke(false)}>取消</button></p>}
    {error && <p className={styles.notice} role="alert">{error}<button onClick={() => setRetry(v => v + 1)}>重试</button></p>}
    {!view && !error && <p role="status">正在读取…</p>}
    <div className={styles.feed} ref={feed}>
      {!!view?.document.warnings.length && <details><summary>{view.document.warnings.length} 项导入提示</summary>{view.document.warnings.map((s, i) => <p key={i}>{s}</p>)}</details>}
      {raw ? <pre>{view?.document.raw_text}</pre> : messages.map(row => <article key={row.id} className={styles.row}>
        <div className={styles.avatar} style={{ background: ElonRecordPresentation.identity(row.sender).color, color: '#fff' }} aria-hidden="true">{ElonRecordPresentation.identity(row.sender).initial}</div><div className={styles.body}>
          <div className={styles.meta}><span>{row.sender}</span><time>{row.time}</time></div>
          {row.kind === 'forward' ? <button className={styles.nested} onClick={() => move(row.id)}>聊天记录 · {view?.document.messages.filter(m => m.parent_id === row.id).length} 条</button> : <>
            <RecordMessage row={row} owner={owner || ''} openLink={openLink} />
            {row.asset_id ? <RecordAsset key={`${owner}:${row.id}`} row={row} card={card} /> : row.filename && <small>导出包未提供可用附件 · {row.filename}</small>}
          </>}
        </div>
      </article>)}
    </div>
  </dialog>
}
