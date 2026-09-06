import { useEffect, useState } from 'react'
import { FileCode2, Network, Search } from 'lucide-react'
import { useResearchRequest } from './useResearchRequest'
import type { ResearchResult, ResearchSession, ContentSlice } from './types'
import styles from './BrowserResearch.module.css'

type View = 'resources' | 'requests' | 'search'
type ListResult = Extract<ResearchResult, { kind: View }>
type DetailResult = Extract<ResearchResult, { kind: 'read_resource' | 'read_request' }>
export default function ResourceInspector({ project, session }: { project: string; session: ResearchSession }) {
  const [view, setView] = useState<View>('resources')
  const [query, setQuery] = useState('')
  const [searched, setSearched] = useState('')
  const [list, setList] = useState<ListResult | null>(null)
  const [detail, setDetail] = useState<DetailResult | null>(null)
  const { run, busy, error, cancel } = useResearchRequest(project, session.id)
  const searchValid = query.trim().length > 0 && new TextEncoder().encode(query.trim()).length <= 200

  useEffect(() => {
    setList(null)
    setDetail(null)
    if (view === 'search') return
    void run({ kind: view, session_id: session.id, offset: 0, limit: 30 }).then((result) => {
      if (result?.kind === 'resources' || result?.kind === 'requests') setList(result)
    })
  }, [run, session.id, view])

  async function load(offset = 0, text = searched) {
    setDetail(null)
    const result = await run({ kind: view, session_id: session.id, offset, limit: 30, ...(view === 'search' ? { query: text } : {}) })
    if (result?.kind === 'resources' || result?.kind === 'requests' || result?.kind === 'search') setList(result)
  }
  async function read(kind: 'read_resource' | 'read_request', id: string, offset = 0) {
    setDetail(null)
    const result = await run({ kind, session_id: session.id, offset, limit: 8192, ...(kind === 'read_resource' ? { resource_id: id } : { request_id: id }) })
    if (result?.kind === 'read_resource' || result?.kind === 'read_request') setDetail(result)
  }
  function changeView(next: View) { cancel(); setView(next); setList(null); setDetail(null) }
  return <section className={styles.inspector}>
    <div className={styles.toolbar}>
      <div className={styles.tabs} role="tablist" aria-label="研究资料类型">
        <button role="tab" aria-selected={view === 'resources'} onClick={() => changeView('resources')}><FileCode2 size={15} />资源</button>
        <button role="tab" aria-selected={view === 'requests'} onClick={() => changeView('requests')}><Network size={15} />请求</button>
        <button role="tab" aria-selected={view === 'search'} onClick={() => changeView('search')}><Search size={15} />搜索</button>
      </div>
      <button disabled={busy || (view === 'search' && !searched)} onClick={() => void load()}>刷新列表</button>
    </div>
    <p className={styles.help}>仅列出会话实际观察到的资料。凭据已定向排除，业务字段仍可能包含私人信息。</p>
    {view === 'search' && <form className={styles.searchForm} onSubmit={(event) => { event.preventDefault(); if (searchValid) { setSearched(query.trim()); void load(0, query.trim()) } }}>
      <label>搜索资源内容<input value={query} maxLength={256} autoComplete="off" spellCheck={false} placeholder="输入接口路径、字段名或代码关键词"
        onChange={(event) => { cancel(); setQuery(event.target.value); setSearched(''); setList(null); setDetail(null) }} /></label>
      <button className={styles.primary} disabled={!searchValid || busy}>搜索</button>
    </form>}
    {view === 'search' && query.trim() && !searchValid && <p className={styles.notice}>关键词过长，请缩短后搜索。</p>}
    {error && <p className={styles.error} role="alert">{error}</p>}
    {busy && <div className={styles.waiting} role="status">正在读取指定范围…<button onClick={cancel}>停止等待</button></div>}
    <div className={styles.results}>
      <div className={styles.resultList}>
        <div className={styles.sectionTitle}><h2>{view === 'resources' ? '已加载资源' : view === 'requests' ? '已观察请求' : '文本命中'}</h2><span>{list ? `${list.total} 项` : '尚未读取'}</span></div>
        {list?.partial && <p className={styles.notice}>本次检索范围不完整，可能有资源不可读或命中数量达到上限。</p>}
        {list && list.items.length === 0 && <p className={styles.help}>{view === 'search' ? '此范围没有命中。' : '尚无资料。打开官网并浏览所需页面后刷新。'} 空结果不能证明业务不存在。</p>}
        {list?.kind === 'resources' && list.items.map((item) => <button className={styles.resultRow} key={item.id} disabled={busy} onClick={() => void read('read_resource', item.id)}>
          <span className={styles.rowMeta}>{item.resource_type} · {item.size_bytes.toLocaleString()} 字节 · 第 {item.generation} 代{item.truncated ? ' · 已截断' : ''}{item.redacted ? ' · 已处理凭据' : ''}</span>
          <strong>{item.url}</strong><small>{item.id}</small>
        </button>)}
        {list?.kind === 'requests' && list.items.map((item) => <button className={styles.resultRow} key={item.id} disabled={busy} onClick={() => void read('read_request', item.id)}>
          <span className={styles.rowMeta}>{item.method} · HTTP {item.status ?? '未知'} · 第 {item.generation} 代</span><strong>{item.url}</strong><small>{item.id}</small>
        </button>)}
        {list?.kind === 'search' && list.items.map((item, index) => <button className={styles.resultRow} key={`${item.resource_id}:${item.offset}:${index}`} disabled={busy} onClick={() => void read('read_resource', item.resource_id, item.offset)}>
          <strong>{item.url}</strong><small>{item.resource_id} · 字节位置 {item.offset}</small><pre>{item.excerpt}</pre>
        </button>)}
        {list && <div className={styles.pagination}><span>当前从第 {list.offset + (list.items.length ? 1 : 0)} 项开始</span><button disabled={busy || list.offset === 0} onClick={() => void load(0)}>回到开头</button><button disabled={busy || list.next_offset === null} onClick={() => { if (list.next_offset !== null) void load(list.next_offset) }}>下一页</button></div>}
      </div>
      {detail && <article className={styles.detail}>
        <div className={styles.sectionTitle}><h2>{detail.kind === 'read_resource' ? '资源片段' : '请求样本'}</h2><button onClick={() => setDetail(null)}>收起</button></div>
        {detail.kind === 'read_resource' ? <>
          <p className={styles.url}>{detail.item.url}</p><p className={styles.help}>资源 {detail.item.id} · SHA-256 {detail.item.sha256}</p>
          {(detail.item.truncated || detail.item.redacted) && <p className={styles.notice}>{detail.item.truncated ? '资源保存时存在截断。' : ''}{detail.item.redacted ? '凭据内容经过定向处理。' : ''}</p>}
          <Slice value={detail} />
          <button disabled={busy || detail.next_offset === null} onClick={() => { if (detail.next_offset !== null) void read('read_resource', detail.item.id, detail.next_offset) }}>读取下一段</button>
        </> : <>
          <p className={styles.url}>{detail.request.method} {detail.request.url}</p><p className={styles.help}>请求 {detail.request.id} · HTTP {detail.request.status ?? '未知'}。业务是否成功需要结合响应内容判断。</p>
          <h3>请求内容</h3>{detail.request_body ? <Slice value={detail.request_body} /> : <p className={styles.help}>请求内容不可用或未捕获。</p>}
          <h3>响应内容</h3>{detail.response_body ? <Slice value={detail.response_body} /> : <p className={styles.help}>响应内容不可用或未捕获。</p>}
          {detail.request.request_resource_id && <button disabled={busy} onClick={() => void read('read_resource', detail.request.request_resource_id!)}>按资源读取请求内容</button>}
          {detail.request.response_resource_id && <button disabled={busy} onClick={() => void read('read_resource', detail.request.response_resource_id!)}>按资源读取响应内容</button>}
        </>}
      </article>}
    </div>
  </section>
}

function Slice({ value }: { value: ContentSlice }) {
  return <><p className={styles.help}>字节位置 {value.offset} · {value.complete ? '此范围已读完' : '仍有后续内容'}</p><pre className={styles.code}>{value.content || '（空内容）'}</pre></>
}
