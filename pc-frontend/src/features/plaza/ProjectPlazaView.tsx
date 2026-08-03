import { useCallback, useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Download, LayoutGrid, Loader2, LogIn, Search, UsersRound } from 'lucide-react'
import { api } from '../../api/client'
import { useProjectStore } from '../conversation/useProjectStore'
import styles from './PlazaPage.module.css'

export interface PlazaProject {
  id: string
  name: string
  display_name?: string
  description?: string
  template: string
  owner_account: string
  member_count: number
  join_mode: string
  viewer_role?: string
  latest_apk_url?: string
  icon_data_url?: string
  created_at: string
  updated_at?: string
}

interface StoreProjectsResponse {
  projects?: PlazaProject[]
  total?: number | null
  next_cursor?: string | null
  has_more?: boolean
}

interface Filter {
  key: string
  label: string
  params?: Record<string, string | boolean>
  joinedOnly?: boolean
}

type JoinStatus = 'joined' | 'requested' | 'error'

const PAGE_SIZE = 30

const FILTERS: Filter[] = [
  { key: 'all', label: '全部' },
  { key: 'installable', label: '可安装', params: { has_apk: true } },
  { key: 'no_approval', label: '无审批', params: { join_mode: 'open' } },
  { key: 'joined', label: '已加入', joinedOnly: true },
  { key: 'popular', label: '最热门', params: { sort: 'members' } },
]

export default function ProjectPlazaView() {
  const navigate = useNavigate()
  const [projects, setProjects] = useState<PlazaProject[]>([])
  const [loading, setLoading] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)
  const [error, setError] = useState('')
  const [searchQ, setSearchQ] = useState('')
  const [submittedQ, setSubmittedQ] = useState('')
  const [activeFilter, setActiveFilter] = useState('all')
  const [joiningId, setJoiningId] = useState<string | null>(null)
  const [joinStatus, setJoinStatus] = useState<Record<string, JoinStatus>>({})
  const [total, setTotal] = useState<number | null>(null)
  const [nextCursor, setNextCursor] = useState<string | null>(null)
  const [hasMore, setHasMore] = useState(false)

  const load = useCallback(async (options?: { append?: boolean; cursor?: string | null }) => {
    const append = options?.append ?? false
    if (append && !options?.cursor) return
    const filter = FILTERS.find((item) => item.key === activeFilter) ?? FILTERS[0]
    setError('')
    if (append) setLoadingMore(true)
    else setLoading(true)
    try {
      if (filter.joinedOnly) {
        const data = await api.get<{ projects?: PlazaProject[] }>('/api/store/joined')
        const source = data.projects ?? []
        const keyword = submittedQ.trim().toLowerCase()
        const filtered = keyword
          ? source.filter((project) => {
            const haystack = [
              project.display_name,
              project.name,
              project.description,
              project.owner_account,
            ].filter(Boolean).join(' ').toLowerCase()
            return haystack.includes(keyword)
          })
          : source
        setProjects(filtered)
        setTotal(filtered.length)
        setNextCursor(null)
        setHasMore(false)
        return
      }

      const params = new URLSearchParams({ limit: String(PAGE_SIZE), page_mode: 'cursor' })
      if (options?.cursor) params.set('cursor', options.cursor)
      if (submittedQ.trim()) params.set('q', submittedQ.trim())
      if (filter.params) {
        for (const [key, value] of Object.entries(filter.params)) {
          params.set(key, String(value))
        }
      }
      const data = await api.get<StoreProjectsResponse>(`/api/store/projects?${params.toString()}`)
      const incoming = data.projects ?? []
      setProjects((current) => append ? mergeProjects(current, incoming) : incoming)
      setTotal(typeof data.total === 'number' ? data.total : null)
      setNextCursor(data.next_cursor ?? null)
      setHasMore(Boolean(data.has_more && data.next_cursor))
    } catch (caught) {
      setError((caught as { message?: string }).message ?? '项目广场加载失败')
    } finally {
      setLoading(false)
      setLoadingMore(false)
    }
  }, [activeFilter, submittedQ])

  useEffect(() => {
    void load()
  }, [load])

  function handleSearch(event: React.FormEvent) {
    event.preventDefault()
    setNextCursor(null)
    setHasMore(false)
    setTotal(null)
    setSubmittedQ(searchQ)
  }

  function switchFilter(key: string) {
    setActiveFilter(key)
    setNextCursor(null)
    setHasMore(false)
    setTotal(null)
  }

  async function openProject(project: PlazaProject) {
    await useProjectStore.getState().selectProject(project.id)
    navigate('/workspace')
  }

  async function handleJoin(project: PlazaProject) {
    setJoiningId(project.id)
    try {
      if (project.join_mode === 'open' || project.join_mode === 'readonly') {
        await api.post(`/api/projects/${encodeURIComponent(project.id)}/join`, {})
        setJoinStatus((current) => ({ ...current, [project.id]: 'joined' }))
        setProjects((current) => current.map((item) => (
          item.id === project.id ? { ...item, viewer_role: 'member' } : item
        )))
        await useProjectStore.getState().loadProjects()
      } else if (project.join_mode === 'invite') {
        await openProject(project)
      } else {
        await api.post(`/api/projects/${encodeURIComponent(project.id)}/request-join`, {})
        setJoinStatus((current) => ({ ...current, [project.id]: 'requested' }))
      }
    } catch (caught) {
      setJoinStatus((current) => ({ ...current, [project.id]: 'error' }))
      setError((caught as { message?: string }).message ?? '操作失败')
    } finally {
      setJoiningId(null)
    }
  }

  const canLoadMore = activeFilter !== 'joined' && hasMore

  return (
    <section className={styles.page} aria-label="项目广场">
      <header className={styles.toolbar}>
        <div className={styles.toolbarTitle}>
          <LayoutGrid size={18} aria-hidden="true" />
          <div>
            <strong>项目广场</strong>
            <span>发现、安装或加入公开项目</span>
          </div>
        </div>
        <form onSubmit={handleSearch} className={styles.searchRow}>
          <Search size={16} aria-hidden="true" />
          <input
            className={styles.searchInput}
            value={searchQ}
            onChange={(event) => setSearchQ(event.target.value)}
            placeholder="搜索应用、项目、作者"
          />
          <button className={styles.searchBtn} type="submit">搜索</button>
        </form>
        <div className={styles.filterRow} aria-label="项目筛选">
          {FILTERS.map((filter) => (
            <button
              key={filter.key}
              className={[styles.filterBtn, activeFilter === filter.key ? styles.filterActive : ''].join(' ')}
              onClick={() => switchFilter(filter.key)}
              type="button"
            >
              {filter.label}
            </button>
          ))}
        </div>
      </header>

      {error && <div className={styles.error}>{error}</div>}

      {loading ? (
        <div className={styles.loading}>
          <Loader2 size={18} aria-hidden="true" />
          <span>读取项目广场…</span>
        </div>
      ) : projects.length === 0 ? (
        <div className={styles.empty}>
          <LayoutGrid size={24} aria-hidden="true" />
          <strong>没有找到符合条件的项目</strong>
          <span>可以换个关键词，或查看全部公开项目。</span>
        </div>
      ) : (
        <div className={styles.scrollArea}>
          <div className={styles.resultMeta}>
            <span>{total === null ? `已显示 ${projects.length}` : `已显示 ${projects.length} / ${total || projects.length}`}</span>
          </div>
          <div className={styles.grid} data-testid="project-list">
            {projects.map((project) => (
              <ProjectCard
                key={project.id}
                project={project}
                joining={joiningId === project.id}
                joinStatus={joinStatus[project.id]}
                onJoin={handleJoin}
                onOpen={openProject}
              />
            ))}
          </div>
          {canLoadMore && (
            <div className={styles.loadMoreRow}>
              <button
                className={styles.loadMoreBtn}
                data-testid="project-list-more"
                type="button"
                disabled={loadingMore}
                onClick={() => void load({ append: true, cursor: nextCursor })}
              >
                {loadingMore ? '加载中…' : '加载更多'}
              </button>
            </div>
          )}
        </div>
      )}
    </section>
  )
}

function ProjectCard({
  project,
  joining,
  joinStatus,
  onJoin,
  onOpen,
}: {
  project: PlazaProject
  joining: boolean
  joinStatus?: JoinStatus
  onJoin: (project: PlazaProject) => void
  onOpen: (project: PlazaProject) => void
}) {
  const title = project.display_name || project.name
  const alreadyJoined = Boolean(project.viewer_role) || joinStatus === 'joined'
  const requested = joinStatus === 'requested'
  const isOpen = project.join_mode === 'open'
  const isReadonly = project.join_mode === 'readonly'

  return (
    <article className={styles.card} data-testid="project-row" data-project-id={project.id}>
      <header className={styles.cardHead}>
        <ProjectIcon project={project} />
        <div className={styles.cardTitle}>
          <strong title={title}>{title}</strong>
          <span title={project.owner_account}>@{project.owner_account}</span>
        </div>
        <span className={styles.statusPill} data-mode={project.latest_apk_url ? 'installable' : project.join_mode}>
          {project.latest_apk_url ? '可安装' : joinModeLabel(project.join_mode)}
        </span>
      </header>

      <p className={styles.cardDesc}>{project.description || '这个项目还没有填写简介。'}</p>

      <div className={styles.cardMeta}>
        <span><UsersRound size={13} aria-hidden="true" />{project.member_count} 成员</span>
        <span>{project.template || '项目'}</span>
        <span>{formatProjectDate(project.updated_at || project.created_at)}</span>
      </div>

      <div className={styles.cardActions}>
        {project.latest_apk_url && (
          <a
            href={project.latest_apk_url}
            className={styles.apkBtn}
            target="_blank"
            rel="noopener noreferrer"
          >
            <Download size={14} aria-hidden="true" />
            <span>下载</span>
          </a>
        )}
        {alreadyJoined ? (
          <button className={styles.openBtn} type="button" onClick={() => onOpen(project)}>
            <LogIn size={14} aria-hidden="true" />
            <span>进入项目</span>
          </button>
        ) : requested ? (
          <span className={styles.requestedLabel}>已申请</span>
        ) : project.join_mode === 'invite' ? (
          <button className={styles.openBtn} type="button" onClick={() => onOpen(project)}>
            <LogIn size={14} aria-hidden="true" />
            <span>查看项目</span>
          </button>
        ) : (
          <button
            className={styles.joinBtn}
            disabled={joining}
            onClick={() => onJoin(project)}
            type="button"
          >
            {joining ? '处理中…' : isOpen || isReadonly ? '加入项目' : '申请加入'}
          </button>
        )}
      </div>
    </article>
  )
}

function ProjectIcon({ project }: { project: PlazaProject }) {
  const title = project.display_name || project.name
  if (project.icon_data_url) {
    return <img className={styles.cardIconImage} src={project.icon_data_url} alt="" />
  }
  return <div className={styles.cardIcon}>{title[0]?.toUpperCase() || '项'}</div>
}

function joinModeLabel(mode: string): string {
  if (mode === 'open') return '无审批'
  if (mode === 'approval') return '需审批'
  if (mode === 'readonly') return '只读'
  if (mode === 'invite') return '邀请'
  return mode || '公开'
}

function formatProjectDate(value?: string): string {
  if (!value) return '最近更新'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return '最近更新'
  return new Intl.DateTimeFormat('zh-CN', { month: '2-digit', day: '2-digit' }).format(date)
}

function mergeProjects(current: PlazaProject[], incoming: PlazaProject[]): PlazaProject[] {
  const seen = new Set(current.map((project) => project.id))
  return [...current, ...incoming.filter((project) => !seen.has(project.id))]
}
