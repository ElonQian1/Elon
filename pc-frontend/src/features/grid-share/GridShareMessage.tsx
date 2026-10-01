import { useEffect, useRef, useState } from 'react'
import GridShareReaderDialog from './GridShareReaderDialog'
import { detailSections, gridCard, labels, value, type GridShareView } from './gridShareModel'
import { readShare, sharePath, shareRequest } from './gridShareApi'
import GridShareSummary from './GridShareSummary'
import { GridShareComposeDialog } from './GridShareComposer'
import styles from './GridShare.module.css'

interface Props { content: string; group: string; owner: string; title: string; onQuote: () => void; onAnalyze: () => void; onSent: () => void }
export default function GridShareMessage(props: Props) {
  const card = gridCard(props.content)!
  const [open, setOpen] = useState(false), [view, setView] = useState<GridShareView | null>(null)
  const [error, setError] = useState(''), [busy, setBusy] = useState(false), [tab, setTab] = useState('参数')
  const [updating, setUpdating] = useState(false)
  const controller = useRef<AbortController | null>(null)
  useEffect(() => () => controller.current?.abort(), [props.owner, props.group])
  function close() { controller.current?.abort(); setOpen(false); setView(null); setUpdating(false); setError(''); setBusy(false) }
  async function load(id = card.snapshot_id) {
    controller.current?.abort(); const run = new AbortController(); controller.current = run
    setOpen(true); setView(null); setError(''); setBusy(true)
    try {
      if (card.group_id !== props.group) throw Error('请从原群聊打开此分享')
      const next = await readShare(props.group, id, run.signal)
      if (!run.signal.aborted) setView(next)
    } catch (e) { if (!run.signal.aborted) setError((e as Error).message) }
    finally { if (!run.signal.aborted) setBusy(false) }
  }
  async function revoke() {
    if (!view || busy) return
    setBusy(true)
    try { await shareRequest(sharePath(props.group, view.snapshot_id), controller.current!.signal, undefined, 'DELETE'); close(); props.onSent() }
    catch (e) { setView(null); setError((e as Error).message); setBusy(false) }
  }
  const selected = view?.document.grid, original = view?.snapshot_id === card.snapshot_id
  return <>
    <button type="button" className={styles.card} onClick={() => void load()} aria-label={`查看 ${card.title}`}><GridShareSummary grid={card.grid} /><span>查看网格详情 ›</span></button>
    {open && <GridShareReaderDialog onClose={close}>
      {busy && <p role="status">正在读取…</p>}{error && <p role="alert">{error} <button type="button" onClick={() => void load()}>重试</button></p>}
      {view && selected && <><p className={styles.notice}>{view.owner_name} 分享 · 历史快照，打开详情不会刷新币安</p><div className={styles.summary}><GridShareSummary grid={selected} /></div>
        {view.latest_snapshot_id && <p><button type="button" onClick={() => void load(view.latest_snapshot_id!)}>已有更新 · 查看最新快照</button></p>}
        <div className={styles.tabs} role="tablist" aria-label="详情分类">{Object.keys(detailSections).map(t => <button key={t} type="button" role="tab" aria-selected={tab === t} onClick={() => setTab(t)}>{t}</button>)}</div>
        <dl className={styles.rows}>{detailSections[tab].map(key => <div key={key} style={{ display: 'contents' }}><dt>{labels[key]}</dt><dd>{value(selected, key)}</dd></div>)}</dl>
        {selected.note && <blockquote>{selected.note}</blockquote>}<p className={styles.notice}>“未读取”表示此快照没有该数据。网格利润与策略总盈亏不是同一口径。</p>
        <div className={styles.actions}>{original ? <><button type="button" onClick={() => { close(); props.onQuote() }}>引用讨论</button><button type="button" onClick={() => { close(); props.onAnalyze() }}>询问群 AI</button></> : <p>如需引用最新版本，请从群里的新卡片发起。</p>}
          {view.owner_id === props.owner && <><button type="button" disabled={busy || !!view.latest_snapshot_id} onClick={() => setUpdating(true)}>手动更新快照</button><button type="button" disabled={busy} onClick={() => void revoke()}>撤回分享</button></>}
        </div>
      </>}
    </GridShareReaderDialog>}
    {updating && view && <GridShareComposeDialog owner={props.owner} group={props.group} title={props.title} previous={view.snapshot_id} onClose={() => setUpdating(false)} onSent={() => { close(); props.onSent() }} />}
  </>
}
