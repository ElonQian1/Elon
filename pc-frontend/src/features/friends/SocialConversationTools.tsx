import { useEffect, useRef, useState } from 'react'
import { Bot } from 'lucide-react'
import { copyTextToClipboard } from '../../lib/clipboard'
import MarkdownContent from '../markdown/MarkdownContent'
import type { ActiveConversation, SocialMessage } from './socialMessageTypes'
import { localMessageKey, type SavedSocialMessage } from './socialLocalState'
import { messageText, socialRequest } from './socialChatOperations'
import SocialDialog from './SocialDialog'
import styles from './SocialTools.module.css'

interface Summary { id: string; title: string; summary?: string; pinned_at?: string | null; status?: string }
interface Hit { id: string; content: string; sender_name: string; created_at: string }
interface Props {
  conversation: ActiveConversation; query: string; onQuery: (value: string) => void; messages: SocialMessage[]
  selected: SavedSocialMessage[]; selectionMode?: boolean; onClearSelection: () => void; onForward: (items: SavedSocialMessage[]) => void; onSave: (items: SavedSocialMessage[]) => void
  favorites: SavedSocialMessage[]; onRemoveFavorite: (key: string) => void; hiddenCount: number; onRestore: () => void
  onAiReply?: () => void
}

export default function SocialConversationTools(props: Props) {
  const { conversation, query, onQuery, selected } = props
  const [panel, setPanel] = useState<'favorites' | 'search' | 'summaries' | null>(null)
  const [search, setSearch] = useState('')
  const [hits, setHits] = useState<Hit[]>([])
  const [posts, setPosts] = useState<Summary[]>([])
  const [activePost, setActivePost] = useState<Summary | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [reload, setReload] = useState(0)
  const [searched, setSearched] = useState(false)
  const [notice, setNotice] = useState('')
  const epoch = useRef(0)
  useEffect(() => { epoch.current++; setBusy(false); setPanel(null); setHits([]); setPosts([]); setActivePost(null); setError(''); setNotice(''); return () => { epoch.current++ } }, [conversation.kind, conversation.id])
  useEffect(() => {
    if (panel !== 'summaries') return
    let stopped = false
    setBusy(true); setError('')
    socialRequest<{ posts: Summary[] }>(`/api/me/groups/${encodeURIComponent(conversation.id)}/summary-posts?limit=50`)
      .then(data => { if (!stopped) setPosts(data.posts.sort((a,b) => Number(!!b.pinned_at)-Number(!!a.pinned_at))) })
      .catch(failure => { if (!stopped) setError(failure.message) }).finally(() => { if (!stopped) setBusy(false) })
    return () => { stopped = true }
  }, [panel, conversation.id, reload])
  async function findMessages() {
    const request = ++epoch.current
    setBusy(true); setError(''); setHits([]); setSearched(true)
    try {
      const data = await socialRequest<{ retrieval: { hits: { message: Hit }[] } }>(`/api/me/groups/${encodeURIComponent(conversation.id)}/messages/search`, {
        method: 'POST', headers: { 'Content-Type':'application/json' }, body: JSON.stringify({ query: search.trim(), limit: 40 }),
      })
      if (request === epoch.current) setHits(data.retrieval.hits.map(hit => hit.message))
    } catch (failure) { if (request === epoch.current) setError((failure as Error).message) }
    finally { if (request === epoch.current) setBusy(false) }
  }
  async function showPost(post: Summary) {
    const request = ++epoch.current
    setBusy(true); setError('')
    try {
      const data = await socialRequest<{ post: Summary }>(`/api/me/groups/${encodeURIComponent(conversation.id)}/summary-posts/${encodeURIComponent(post.id)}`)
      if (request === epoch.current) setActivePost(data.post)
    } catch (failure) { if (request === epoch.current) setError((failure as Error).message) }
    finally { if (request === epoch.current) setBusy(false) }
  }
  return <>
    <div className={styles.toolbar} aria-label="聊天工具">
      <input aria-label="查找已加载消息" placeholder="查找已加载的消息" value={query} onChange={event => onQuery(event.target.value)} />
      {query && <button type="button" onClick={() => onQuery('')}>清除查找</button>}
      <button type="button" onClick={() => { setError(''); setPanel('favorites') }}>本机收藏</button>
      {!!props.hiddenCount && <button type="button" onClick={props.onRestore}>恢复本机隐藏（{props.hiddenCount}）</button>}
      {conversation.kind === 'group' && <><button type="button" onClick={() => { setError(''); setPanel('search') }}>群历史检索</button><button type="button" onClick={() => { setActivePost(null); setPanel('summaries') }}>置顶 / 总结</button></>}
      {query && <span className={styles.hint}>当前 {props.messages.length} 条已加载消息；不是全部历史</span>}
    </div>
    {props.selectionMode && <div className={styles.toolbar} aria-label="多选消息操作"><strong>已选 {selected.length} 条</strong>
      {props.onAiReply && <button type="button" disabled={!selected.length} onClick={props.onAiReply}><Bot size={16} aria-hidden="true" /> AI 分析</button>}
      <button type="button" disabled={!selected.length} onClick={() => void copyTextToClipboard(selected.map(item => messageText(item.message)).join('\n\n')).then(ok => setNotice(ok ? '已复制所选消息' : '复制失败'))}>复制所选</button>
      <button type="button" disabled={!selected.length} onClick={() => props.onForward(selected)}>转发所选</button><button type="button" disabled={!selected.length} onClick={() => props.onSave(selected)}>收藏所选</button><button type="button" onClick={props.onClearSelection}>退出多选</button>
    </div>}
    {notice && <p className={styles.hint} role="status">{notice}</p>}
    {panel && <SocialDialog title={panel === 'favorites' ? '本机收藏' : panel === 'search' ? '群历史检索' : '置顶与群聊总结'} onClose={() => { epoch.current++; setBusy(false); setPanel(null) }}>
      {panel === 'favorites' && <><p className={styles.hint}>保存在本机的消息快照，可能与原文当前版本不同；不会跨设备同步，退出登录时清理。</p>
        {!props.favorites.length && <p>还没有收藏的消息</p>}{props.favorites.map(item => <article className={styles.item} key={localMessageKey(item.conversation,item.message.id)}>
          <strong>{item.title}</strong><time> · {new Date(item.message.created_at).toLocaleString()}</time><pre>{messageText(item.message)}</pre>
          <button type="button" onClick={() => props.onRemoveFavorite(localMessageKey(item.conversation,item.message.id))}>取消收藏</button>
        </article>)}</>}
      {panel === 'search' && <><form onSubmit={event => { event.preventDefault(); if (search.trim()) void findMessages() }}>
        <label>关键词<input aria-label="群历史关键词" value={search} onChange={event => setSearch(event.target.value)} /></label><button type="submit" disabled={busy || !search.trim()}>搜索群历史</button></form>
        <p className={styles.hint}>从服务器检索群历史，最多显示 40 条匹配消息。</p>{hits.map(hit => <article className={styles.item} key={hit.id}><strong>{hit.sender_name}</strong><time> · {new Date(hit.created_at).toLocaleString()}</time><pre>{hit.content}</pre></article>)}
        {!busy && !error && !hits.length && <p>{searched ? '没有找到匹配消息' : '输入关键词后查找匹配消息'}</p>}
      </>}
      {panel === 'summaries' && (activePost ? <><button type="button" onClick={() => setActivePost(null)}>返回总结列表</button><h3>{activePost.title}</h3><MarkdownContent content={activePost.summary || '总结内容暂不可用'} copy={false} /></> : <>
        {!busy && !error && !posts.length && <p>群里还没有总结帖</p>}{posts.map(post => <article key={post.id} className={styles.item}><button type="button" disabled={busy} onClick={() => void showPost(post)}>{post.pinned_at ? '置顶 · ' : ''}{post.title}</button></article>)}
      </>)}
      {busy && <p role="status">正在读取…</p>}{error && <p className={styles.error} role="alert">{error} {panel === 'summaries' && <button type="button" onClick={() => { setReload(old => old + 1) }}>重试</button>}</p>}
    </SocialDialog>}
  </>
}
