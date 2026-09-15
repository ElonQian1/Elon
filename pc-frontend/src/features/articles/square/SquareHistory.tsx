import { useEffect, useState } from 'react'
import { square, labels, modes, stamp, creatorUrl, postIdFromLink, type Job } from './squareApi'
import styles from '../Articles.module.css'
export default function SquareHistory() {
  const [rows, setRows] = useState<Job[]>([]), [next, setNext] = useState<number | null>(null), [busy, setBusy] = useState(false), [error, setError] = useState(''), [resolve, setResolve] = useState(''), [link, setLink] = useState('')
  const load = async (offset = 0) => { const p = await square<{ items: Job[]; next_offset: number | null }>(`/jobs?offset=${offset}`); setRows(v => offset ? [...v, ...p.items] : p.items); setNext(p.next_offset) }
  const run = async (fn: () => Promise<void>) => { if (busy) return; setBusy(true); setError(''); try { await fn() } catch (e) { setError(e instanceof Error ? e.message : '读取失败') } finally { setBusy(false) } }
  useEffect(() => { void run(() => load()) }, []) // Mounted once per visible history panel.
  const action = (id: string, verb: string, data = {}) => run(async () => { await square(`/jobs/${id}/${verb}`, 'POST', data); setResolve(''); await load() })
  return <section className={styles.fields} aria-label="币安发布记录"><div className={styles.toolbar}><h3>发布记录</h3><button disabled={busy} onClick={() => void run(() => load())}>刷新记录</button></div>
    <p>定时任务由服务器发送，关闭此页面不影响排队。修改或删除币安已有内容，请到<a href={creatorUrl} target="_blank" rel="noopener noreferrer">创作者中心</a>操作。</p>
    {error && <p className={styles.error} role="alert">{error}</p>}{busy && <p role="status">正在处理…</p>}
    {!rows.length && !busy && !error && <p>还没有发布记录。从文章预览页选择“币安广场”开始。</p>}
    {rows.map(j => <section key={j.id} className={styles.block}><strong>{j.title}</strong><p>{modes[j.mode]} · 文章 v{j.revision} · {labels[j.status] || j.status}</p><small>计划 {stamp(j.scheduled_at)} · 尝试 {j.attempts} 次</small><p>{j.message}</p>
      <div className={styles.toolbar}>{j.post_url && <a href={j.post_url} target="_blank" rel="noopener noreferrer">打开币安帖子 ↗</a>}
        {['queued', 'preparing', 'failed'].includes(j.status) && <button disabled={busy} onClick={() => void action(j.id, 'cancel')}>取消任务</button>}
        {['failed', 'cancelled'].includes(j.status) && <button disabled={busy} onClick={() => { if (confirm('将按这条记录保存的文章版本重新发布，确定继续？')) void action(j.id, 'retry') }}>重试此版本</button>}
        {j.status === 'uncertain' && <button disabled={busy} onClick={() => { setResolve(j.id); setLink('') }}>核实发布结果</button>}
      </div>
      {resolve === j.id && <div><p>请先在币安确认是否已有帖子，避免重复发帖。</p><label>已有帖子链接<input value={link} onChange={e => setLink(e.target.value)} placeholder="币安帖子链接或数字ID" /></label><button disabled={busy || !link.trim()} onClick={() => void run(async () => { await square(`/jobs/${j.id}/resolve`, 'POST', { post_id: postIdFromLink(link) }); setResolve(''); await load() })}>记录已发布</button><button disabled={busy} onClick={() => { if (confirm('我已在币安核实，这篇内容没有发布。确认后可再次发送。')) void action(j.id, 'resolve', { not_published: true }) }}>我已核实未发布</button></div>}
    </section>)}
    {next !== null && <button disabled={busy} onClick={() => void run(() => load(next))}>加载更多</button>}
  </section>
}
