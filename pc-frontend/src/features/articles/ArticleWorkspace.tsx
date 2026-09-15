import { useEffect, useState } from 'react'
import { articles, errorMessage, type Article, type Card, type Group } from './articleApi'
import ArticleEditor from './ArticleEditor'
import ArticleReader from './ArticleReader'
import styles from './Articles.module.css'
import ArticleDialog from './ArticleDialog'

function Library({ groupId, groups, onClose }: { groupId: string; groups: Group[]; onClose: () => void }) {
  const [mine, setMine] = useState(false); const [rows, setRows] = useState<Card[]>([]); const [next, setNext] = useState<number | null>(null)
  const [busy, setBusy] = useState(false); const [error, setError] = useState(''); const [editing, setEditing] = useState<Article>(); const [reading, setReading] = useState<Card>(); const [reload, setReload] = useState(0)
  useEffect(() => { let live = true; setBusy(true); setError(''); setRows([]); setNext(null)
    articles.list(mine ? undefined : groupId).then(p => { if (live) { setRows(p.items); setNext(p.next_offset) } }).catch(e => { if (live) setError(errorMessage(e)) }).finally(() => { if (live) setBusy(false) })
    return () => { live = false }
  }, [mine, groupId, reload])
  const run = async (fn: () => Promise<void>) => { if (busy) return; setBusy(true); setError(''); try { await fn() } catch (e) { setError(errorMessage(e)) } finally { setBusy(false) } }
  return <ArticleDialog label="文章中心" onCancel={() => { if (!editing) onClose() }}><section className={styles.panel}>
    {editing ? <ArticleEditor initial={editing} groups={groups} groupId={groupId} onClose={() => { setEditing(undefined); setReload(v => v + 1) }} /> : <>
      <header className={styles.toolbar}><button onClick={onClose}>返回群聊</button><h2>文章</h2><button className={styles.primary} disabled={busy} onClick={() => void run(async () => setEditing(await articles.create()))}>写文章</button></header>
      <nav className={styles.tabs} aria-label="文章范围"><button disabled={busy} aria-pressed={!mine} onClick={() => setMine(false)}>群文章</button><button disabled={busy} aria-pressed={mine} onClick={() => setMine(true)}>我的文章</button></nav>
      {error && <p className={styles.error} role="alert">{error} <button onClick={() => setReload(v => v + 1)}>重试</button></p>}
      <div className={styles.library}>{rows.map(c => <button key={c.id} className={styles.card} disabled={busy || c.status === 'withdrawn'} onClick={() => mine ? void run(async () => setEditing(await articles.draft(c.id))) : setReading(c)}>
        {c.cover_data_url && <img src={c.cover_data_url} alt="" />}<span className={styles.cardText}><small>{c.author_name} · {c.status === 'draft' ? '草稿' : c.status === 'withdrawn' ? '已撤下' : '已发布'}</small><strong>{c.title || '未命名文章'}</strong><span>{c.summary || '打开查看文章'}</span></span>
      </button>)}</div>
      {busy && <p className={styles.empty} role="status">正在加载…</p>}{!busy && !rows.length && !error && <p className={styles.empty}>{mine ? '还没有文章，开始写第一篇吧。' : '群里还没有文章，可以从“写文章”开始。'}</p>}
      {next !== null && <button disabled={busy} onClick={() => void run(async () => { const p = await articles.list(mine ? undefined : groupId, next); setRows(v => [...v, ...p.items]); setNext(p.next_offset) })}>加载更多</button>}
    </>}{reading && <ArticleReader id={reading.id} revision={reading.revision} onClose={() => setReading(undefined)} />}
  </section></ArticleDialog>
}
export default function ArticleWorkspace({ groupId, groups }: { groupId: string; groups: Group[] }) {
  const [open, setOpen] = useState(false)
  return <><button className={styles.entry} onClick={() => setOpen(true)}>文章</button>{open && <Library groupId={groupId} groups={groups} onClose={() => setOpen(false)} />}</>
}
