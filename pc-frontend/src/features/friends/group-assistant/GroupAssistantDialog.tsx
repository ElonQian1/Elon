import { useEffect, useRef, useState } from 'react'
import { ArrowLeft, RefreshCw, Trash2 } from 'lucide-react'
import { getAuthToken } from '../../../api/client'
import MarkdownContent from '../../markdown/MarkdownContent'
import SocialDialog from '../SocialDialog'
import { socialRequest } from '../socialChatOperations'
import styles from '../SocialTools.module.css'

interface Binding {
  id: string; title: string; owner_name: string; owned: boolean; state: string
  synced_at: string | null; update_count: number
}
interface Update { id: string; content: string; created_at: string; sequence: number }
const labels: Record<string, string> = {
  ready: '已同步', no_update: '暂无新结果', missing: '原任务已删除或不可访问',
  auth_required: '分享者需登录原账号', requires_action: '需分享者处理', unavailable: '同步暂不可用',
}
const date = (value: string | null) => value ? new Date(value).toLocaleString() : '尚未同步'

export default function GroupAssistantDialog({ groupId, onClose }: { groupId: string; onClose: () => void }) {
  const [items, setItems] = useState<Binding[]>([])
  const [binding, setBinding] = useState<Binding | null>(null)
  const [updates, setUpdates] = useState<Update[]>([])
  const [cursor, setCursor] = useState<number | null>(null)
  const [active, setActive] = useState<Update | null>(null)
  const [removing, setRemoving] = useState<Binding | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const epoch = useRef(0)
  const owner = useRef(getAuthToken())
  const abort = useRef<AbortController | null>(null)
  const base = `/api/me/groups/${encodeURIComponent(groupId)}/ai-assistant`
  function cancel() { epoch.current++; abort.current?.abort(); setBusy(false) }
  async function run(work: (signal: AbortSignal, current: () => boolean) => Promise<void>) {
    if (!owner.current || owner.current !== getAuthToken()) { cancel(); onClose(); return }
    cancel(); const id = epoch.current; const controller = new AbortController(); abort.current = controller
    const current = () => id === epoch.current && owner.current === getAuthToken()
    setBusy(true); setError('')
    try { await work(controller.signal, current) }
    catch (failure) { if (current()) setError((failure as Error).message || '读取失败，请重试') }
    finally {
      if (id === epoch.current) { setBusy(false); if (owner.current !== getAuthToken()) onClose() }
    }
  }
  function list() { void run(async (signal, current) => {
    const page = await socialRequest<{ items: Binding[] }>(base, { signal })
    if (current()) setItems(page.items)
  }) }
  function read(row: Binding, before = 0) { void run(async (signal, current) => {
    const page = await socialRequest<{ items: Update[]; next_cursor: number | null }>(`${base}/${encodeURIComponent(row.id)}?before=${before}`, { signal })
    if (current()) { setBinding(row); setActive(null); setUpdates(old => before ? [...old, ...page.items] : page.items); setCursor(page.next_cursor) }
  }) }
  function revoke(row: Binding) { void run(async (signal, current) => {
    await socialRequest(`${base}/${encodeURIComponent(row.id)}`, { method: 'DELETE', signal })
    if (current()) { setItems(old => old.filter(item => item.id !== row.id)); setRemoving(null) }
  }) }
  useEffect(() => {
    setBinding(null); setActive(null); setItems([]); list()
    return () => { epoch.current++; abort.current?.abort() }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [groupId])
  function back() { cancel(); setError(''); if (active) setActive(null); else { setBinding(null); setUpdates([]); list() } }
  return <SocialDialog title={binding ? '更新动态' : '群 AI 助手'} onClose={() => { cancel(); onClose() }}>
    <div className={styles.buttons}>
      {binding && <button type="button" title="返回" aria-label="返回" onClick={back}><ArrowLeft size={18} /></button>}
      <button type="button" title="刷新" aria-label="刷新" disabled={busy} onClick={() => binding ? read(binding) : list()}><RefreshCw size={18} /></button>
      <span className={styles.hint}>分享者在线时同步 · 最近结果保留 100 条</span>
    </div>
    {error && <p role="alert" className={styles.error}>{error}</p>}
    {busy && <p role="status">正在读取</p>}
    {!binding && <>
      {!items.length && !busy && !error && <p>暂无关注事项</p>}
      {items.map(row => <section className={styles.item} key={row.id}>
        <button type="button" disabled={busy} onClick={() => read(row)}>{row.title}</button>
        <p className={styles.hint}>{row.owner_name} · ChatGPT · {labels[row.state] || '同步暂不可用'} · {row.update_count} 条更新</p>
        <p className={styles.hint}>最近同步：{date(row.synced_at)}</p>
        {row.owned && <button type="button" title="取消分享" aria-label={`取消分享 ${row.title}`} disabled={busy} onClick={() => setRemoving(row)}><Trash2 size={16} /></button>}
      </section>)}
      {removing && <section role="alertdialog" aria-label="确认取消分享" className={styles.item}>
        <p>取消分享“{removing.title}”？群内结果将撤下，ChatGPT 原任务不变，别人已保存的副本无法收回。</p>
        <div className={styles.buttons}><button type="button" disabled={busy} onClick={() => revoke(removing)}>取消分享</button><button type="button" onClick={() => setRemoving(null)}>返回</button></div>
      </section>}
    </>}
    {binding && <><h3>{binding.title}</h3>
      {active ? <><p className={styles.hint}>{date(active.created_at)}</p><MarkdownContent content={active.content} copy /></> : <>
        {!updates.length && !busy && !error && <p>尚无已同步更新</p>}
        {updates.map(update => <section className={styles.item} key={update.id}>
          <button type="button" onClick={() => setActive(update)}>{date(update.created_at)}</button><p>{update.content.slice(0, 160)}</p>
        </section>)}
        {cursor !== null && <button type="button" disabled={busy} onClick={() => read(binding, cursor)}>较早更新</button>}
      </>}
    </>}
  </SocialDialog>
}
