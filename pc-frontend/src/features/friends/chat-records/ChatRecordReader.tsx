import { useCallback, useEffect, useRef, useState } from 'react'
import { ArrowLeft, Download, Play, X } from 'lucide-react'
import { useAuthStore } from '../../../store/auth'
import { recordPath, recordRequest, type RecordCard, type RecordRow, type RecordView } from './recordApi'
import styles from './ChatRecords.module.css'
import SocialLinkCards from '../SocialLinkCards'
import { useRecordWindow } from './useRecordWindow'
import { useReaderTabs } from '../../reader/readerTabsStore'

function Asset({ row, card }: { row: RecordRow; card: RecordCard }) {
  const [load, setLoad] = useState(row.kind === 'image')
  const [url, setUrl] = useState(''), [error, setError] = useState(''), [retry, setRetry] = useState(0)
  const anchor = useRef<HTMLDivElement>(null)
  const [visible, setVisible] = useState(false)
  useEffect(() => {
    const node = anchor.current; if (!node) return
    const observer = new IntersectionObserver(entries => { if (entries.some(e => e.isIntersecting)) setVisible(true) })
    observer.observe(node); return () => observer.disconnect()
  }, [])
  useEffect(() => {
    if (!load || !visible || !row.asset_id) return
    const controller = new AbortController(); let objectUrl = ''
    setError(''); setUrl('')
    recordRequest(`${recordPath(card)}/assets/${encodeURIComponent(row.asset_id)}`, controller.signal, true)
      .then(value => { if (controller.signal.aborted) return; const blob = value as Blob
        if ((row.kind === 'image' && !blob.type.startsWith('image/')) || (row.kind === 'video' && !blob.type.startsWith('video/'))) throw Error('附件类型不匹配')
        objectUrl = URL.createObjectURL(blob); setUrl(objectUrl)
      }).catch(e => { if (!controller.signal.aborted) setError(e.message || '附件加载失败') })
    return () => { controller.abort(); if (objectUrl) URL.revokeObjectURL(objectUrl) }
  }, [load, visible, row.asset_id, row.kind, card.group_id, card.record_id, retry])
  return <div ref={anchor} className={styles.asset}>
    {!load && <button type="button" onClick={() => setLoad(true)}>{row.kind === 'video' ? <Play size={18} /> : <Download size={18} />}{row.kind === 'video' ? '播放视频' : '读取附件'}</button>}
    {load && !url && !error && <span role="status">正在读取附件…</span>}
    {error && <p role="alert">{error}<button onClick={() => setRetry(v => v + 1)}>重试</button></p>}
    {url && (row.kind === 'image' ? <a href={url} target="_blank" rel="noreferrer"><img src={url} alt={row.filename} /></a>
      : row.kind === 'video' ? <video src={url} controls preload="metadata" aria-label={row.filename} />
        : <a href={url} download={row.filename || '附件'}><Download size={16} />{row.filename || '下载附件'}</a>)}
  </div>
}
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
    <nav><span>微信导出 · {messages.length} 条</span><button onClick={() => setRaw(v => !v)}>{raw ? '返回记录' : '原始文本'}</button>
      {view && view.owner_id === owner && <button onClick={() => setConfirmRevoke(v => !v)}>撤回分享</button>}</nav>
    {confirmRevoke && <p className={styles.notice}>撤回后群成员将不能读取此记录。<button onClick={() => void revoke()}>确认撤回</button><button onClick={() => setConfirmRevoke(false)}>取消</button></p>}
    {error && <p className={styles.notice} role="alert">{error}<button onClick={() => setRetry(v => v + 1)}>重试</button></p>}
    {!view && !error && <p role="status">正在读取…</p>}
    <div className={styles.feed} ref={feed}>
      {!!view?.document.warnings.length && <details><summary>{view.document.warnings.length} 项导入提示</summary>{view.document.warnings.map((s, i) => <p key={i}>{s}</p>)}</details>}
      {raw ? <pre>{view?.document.raw_text}</pre> : messages.map(row => <article key={row.id} className={styles.row}>
        <div className={styles.avatar} aria-hidden="true">{Array.from(row.sender)[0] || '?'}</div><div className={styles.body}>
          <div className={styles.meta}><span>{row.sender}</span><time>{row.time}</time></div>
          {row.kind === 'forward' ? <button className={styles.nested} onClick={() => move(row.id)}>聊天记录 · {view?.document.messages.filter(m => m.parent_id === row.id).length} 条</button> : <>
            {!ElonSocialLinks.compact(row.text) && <p className={styles.text}>{row.text.split(/(https?:\/\/[^\s]+)/g).map((s, i) => /^https?:\/\//.test(s) ? <a key={i} href={s} target="_blank" rel="noreferrer">{s}</a> : s)}</p>}
            <SocialLinkCards text={row.text} owner={owner || ''} compact onDesktopOpen={openLink} />
            {row.asset_id ? <Asset key={`${owner}:${row.id}`} row={row} card={card} /> : row.filename && <small>导出包未提供可用附件</small>}
          </>}
        </div>
      </article>)}
    </div>
  </dialog>
}
