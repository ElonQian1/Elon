import { useCallback, useEffect, useState } from 'react'
import { FolderSearch, RefreshCw, ExternalLink } from 'lucide-react'
import { getDesktopInvoke } from '../shell/desktopShell'
import useLocalAiOwnerIdentity from '../user-browser/useLocalAiOwnerIdentity'
import type { ResearchSession, SiteManifest } from './types'
import { useResearchRequest } from './useResearchRequest'
import ResourceInspector from './ResourceInspector'
import styles from './BrowserResearch.module.css'

export default function BrowserResearchPage() {
  const [projectRoot, setProjectRoot] = useState('')
  const [project, setProject] = useState('')
  const identity = useLocalAiOwnerIdentity()
  const ownerReady = !identity.checking && identity.ownerKey && !identity.ownerKey.startsWith('anonymous-session:')
  const desktop = Boolean(getDesktopInvoke())
  return <div className={styles.page}>
    <header className={styles.header}>
      <div className={styles.heading}><FolderSearch size={25} /><div><span>本机研究资料</span><h1>浏览器研究</h1></div></div>
      <span className={styles.badge}>按需读取 · 本机保存</span>
    </header>
    <div className={styles.intro}>
      在独立浏览器窗口登录官网，查看页面实际加载的资源和请求。研究资料与 AI 共用，无需手动导出。
    </div>
    {!desktop && <p className={styles.notice}>请在一龙 Windows 客户端打开此页，以使用独立浏览器与本机研究库。</p>}
    <form className={styles.projectForm} onSubmit={(event) => { event.preventDefault(); setProject(projectRoot.trim()) }}>
      <label>研究项目目录<input value={projectRoot} maxLength={2048} placeholder="选择本次研究所属的本机项目目录"
        autoComplete="off" spellCheck={false} onChange={(event) => { setProjectRoot(event.target.value); setProject('') }} /></label>
      <button className={styles.primary} disabled={!desktop || !projectRoot.trim()} type="submit">加载研究项目</button>
    </form>
    {project && !ownerReady && <p className={styles.notice}>正在确认当前本机身份；账号冲突或身份未就绪时，研究资料保持关闭。</p>}
    {project && ownerReady ? <ResearchWorkspace key={`${project}:${identity.ownerKey}`} project={project} /> : <div className={styles.empty}>
      <FolderSearch size={32} /><h2>先确定研究项目</h2><p>项目、账号和站点分别隔离。资料仅按所选范围读取。</p>
    </div>}
  </div>
}

function ResearchWorkspace({ project }: { project: string }) {
  const [sites, setSites] = useState<SiteManifest[]>([])
  const [sessions, setSessions] = useState<ResearchSession[]>([])
  const [siteId, setSiteId] = useState('')
  const [selected, setSelected] = useState<ResearchSession | null>(null)
  const [loaded, setLoaded] = useState(false)
  const [now, setNow] = useState(Date.now)
  const request = useResearchRequest(project)
  const run = request.run

  const refresh = useCallback(async () => {
    const catalog = await run({ kind: 'sites' })
    if (catalog?.kind !== 'sites') return
    setSites(catalog.items)
    setSiteId((current) => catalog.items.some((item) => item.id === current) ? current : catalog.items[0]?.id ?? '')
    const list = await run({ kind: 'sessions' })
    if (list?.kind === 'sessions') { setSessions(list.items); setLoaded(true) }
  }, [run])
  useEffect(() => { void refresh() }, [refresh])
  useEffect(() => { const timer = window.setInterval(() => setNow(Date.now()), 1000); return () => window.clearInterval(timer) }, [])
  async function open() {
    const result = await request.run({ kind: 'open', site_id: siteId })
    if (result?.kind === 'open') { setSelected(result.session); setSessions((old) => [result.session, ...old.filter((item) => item.id !== result.session.id)]) }
  }
  async function update(kind: 'status' | 'pause' | 'resume') {
    if (!selected) return
    const result = await request.run({ kind, session_id: selected.id })
    if (result && 'session' in result) {
      setSelected(result.session)
      setSessions((old) => old.map((item) => item.id === result.session.id ? result.session : item))
    }
  }
  const site = sites.find((item) => item.id === siteId)
  const expired = Boolean(selected && selected.expires_at_ms <= now)
  return <>
    {request.error && <p role="alert" className={styles.error}>{request.error}</p>}
    <div className={styles.workspace}>
      <aside className={styles.sidebar}>
        <section className={styles.card}>
          <div className={styles.sectionTitle}><h2>站点与会话</h2><button disabled={request.busy} onClick={() => void refresh()}><RefreshCw size={13} />{loaded ? '刷新' : '读取配置'}</button></div>
          <label>站点配置<select disabled={request.busy || !sites.length} value={siteId} onChange={(event) => { request.cancel(); setSiteId(event.target.value); setSelected(null) }}>
            {!sites.length && <option value="">{loaded ? '没有已登记站点' : '读取配置后选择'}</option>}
            {sites.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}
          </select></label>
          {site && <p className={styles.url}>{site.entry_url}</p>}
          <button className={styles.primary} disabled={!siteId || request.busy} onClick={() => void open()}><ExternalLink size={14} />打开独立官网登录窗口</button>
          <p className={styles.help}>首次登录与站点验证在新窗口完成。使用独立登录空间，不接管已有浏览器。</p>
        </section>
        <section className={styles.card}>
          <h2>已有研究会话</h2>
          {!sessions.filter((item) => !siteId || item.site_id === siteId).length && <p className={styles.help}>暂无可见会话。空列表不代表网站没有相关业务。</p>}
          <div className={styles.sessions}>{sessions.filter((item) => !siteId || item.site_id === siteId).map((item) =>
            <button key={item.id} disabled={request.busy} data-active={selected?.id === item.id} onClick={() => { request.cancel(); setSelected(item) }}>
              <strong>{sites.find((value) => value.id === item.site_id)?.name ?? item.site_id}</strong><span>{phaseLabel(item.phase)} · {item.resource_count} 个资源</span><small>{item.id}</small>
            </button>)}</div>
        </section>
        {selected && <section className={styles.card}>
          <div className={styles.sectionTitle}><h2>{expired ? '已过期' : phaseLabel(selected.phase)}</h2><span className={styles.badge}>{selected.active && !expired ? '采集中' : '未采集'}</span></div>
          <dl className={styles.facts}><dt>文档代次</dt><dd>{selected.generation}</dd><dt>资源 / 请求</dt><dd>{selected.resource_count} / {selected.request_count}</dd><dt>到期时间</dt><dd>{formatTime(selected.expires_at_ms)}</dd></dl>
          <div className={styles.actions}><button disabled={request.busy} onClick={() => void update('status')}>刷新状态</button><button disabled={request.busy || expired} onClick={() => void update(selected.active ? 'pause' : 'resume')}>{selected.active ? '暂停采集' : '恢复采集'}</button></div>
          {selected.gaps.length > 0 && <p className={styles.notice}>采集存在缺口：{selected.gaps.join('、')}</p>}
        </section>}
        {request.busy && <div className={styles.waiting} role="status">等待宿主回执…<button onClick={request.cancel}>停止等待</button><small>停止等待不撤回已开始的操作。</small></div>}
      </aside>
      {selected && expired ? <div className={styles.empty}><h2>研究会话已过期</h2><p>已关闭此会话的资料视图。重新打开官网可建立新的研究会话。</p></div>
        : selected ? <ResourceInspector key={`${project}:${selected.id}:${selected.generation}`} project={project} session={selected} />
        : <div className={styles.empty}><h2>选择一个研究会话</h2><p>可以检索资源、定位代码片段，或按请求 ID 查看业务样本。</p><p>网页内容是待分析材料；HTTP 状态不代表业务操作成功。</p></div>}
    </div>
  </>
}
export function phaseLabel(value: string): string {
  return ({ active: '采集中', observing: '采集中', capturing: '采集中', paused: '已暂停', expired: '已过期', opening: '正在打开', closed: '已关闭', unavailable: '宿主不可用', ready: '已就绪' } as Record<string, string>)[value] ?? '状态待确认'
}
export function formatTime(value: number): string {
  return value ? new Date(value).toLocaleString('zh-CN', { hour12: false }) : '未报告'
}
