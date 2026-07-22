import { useCallback, useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Image, Loader2, Search } from 'lucide-react'
import { api } from '../../api/client'
import avatarAsset from '../../assets/project-plaza/avatar.png'
import cardAsset from '../../assets/project-plaza/card.png'
import heartAsset from '../../assets/project-plaza/heart.png'
import starAsset from '../../assets/project-plaza/star.png'
import thumbnailAsset from '../../assets/project-plaza/thumbnail.png'
import plazaChevronAsset from '../../../../server/src/assets/project_view_chevron.png'
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

type JoinStatus = 'joined' | 'requested' | 'error'

const PAGE_SIZE = 30

export default function ProjectPlazaView() {
  const navigate = useNavigate()
  const [projects, setProjects] = useState<PlazaProject[]>([])
  const [loading, setLoading] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)
  const [error, setError] = useState('')
  const [searchOpen, setSearchOpen] = useState(false)
  const [searchQ, setSearchQ] = useState('')
  const [submittedQ, setSubmittedQ] = useState('')
  const [joiningId, setJoiningId] = useState<string | null>(null)
  const [joinStatus, setJoinStatus] = useState<Record<string, JoinStatus>>({})
  const [nextCursor, setNextCursor] = useState<string | null>(null)
  const [hasMore, setHasMore] = useState(false)
  const [reactions, setReactions] = useState<Record<string, boolean>>(() => {
    try {
      return JSON.parse(localStorage.getItem('project-plaza-reactions') || '{}') as Record<string, boolean>
    } catch {
      return {}
    }
  })

  const load = useCallback(async (options?: { append?: boolean; cursor?: string | null }) => {
    const append = options?.append ?? false
    if (append && !options?.cursor) return
    setError('')
    if (append) setLoadingMore(true)
    else setLoading(true)
    try {
      const params = new URLSearchParams({ limit: String(PAGE_SIZE), page_mode: 'cursor' })
      if (options?.cursor) params.set('cursor', options.cursor)
      if (submittedQ.trim()) params.set('q', submittedQ.trim())
      const data = await api.get<StoreProjectsResponse>(`/api/store/projects?${params.toString()}`)
      const incoming = data.projects ?? []
      setProjects((current) => append ? mergeProjects(current, incoming) : incoming)
      setNextCursor(data.next_cursor ?? null)
      setHasMore(Boolean(data.has_more && data.next_cursor))
    } catch (caught) {
      setError((caught as { message?: string }).message ?? '项目广场加载失败')
    } finally {
      setLoading(false)
      setLoadingMore(false)
    }
  }, [submittedQ])

  useEffect(() => {
    void load()
  }, [load])

  function toggleReaction(projectId: string, kind: 'favorite' | 'liked') {
    const key = `${projectId}:${kind}`
    setReactions((current) => {
      const next = { ...current, [key]: !current[key] }
      localStorage.setItem('project-plaza-reactions', JSON.stringify(next))
      return next
    })
  }

  function handleSearch(event: React.FormEvent) {
    event.preventDefault()
    setNextCursor(null)
    setHasMore(false)
    setSubmittedQ(searchQ)
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

  async function runPrimaryAction(project: PlazaProject) {
    const status = joinStatus[project.id]
    if (project.viewer_role || status === 'joined' || status === 'requested') {
      await openProject(project)
      return
    }
    await handleJoin(project)
  }

  return (
    <section className={styles.page} aria-label="项目广场">
      <header className={styles.toolbar}>
        <h1 className={styles.pageTitle}>项目广场</h1>
      </header>

      {error && <div className={styles.error}>{error}</div>}

      {loading ? (
        <div className={styles.loading}>
          <Loader2 size={18} aria-hidden="true" />
          <span>读取项目广场…</span>
        </div>
      ) : projects.length === 0 ? (
        <div className={styles.empty}>
          <strong>没有找到符合条件的项目</strong>
          <button type="button" onClick={() => { setSearchQ(''); setSubmittedQ('') }}>查看全部</button>
        </div>
      ) : (
        <div className={styles.scrollArea}>
          <div className={styles.sectionHeading}>
            <h2>推荐</h2>
            <button
              className={styles.searchToggle}
              type="button"
              aria-label="搜索项目"
              aria-expanded={searchOpen}
              onClick={() => setSearchOpen((open) => !open)}
            >
              <Search aria-hidden="true" />
            </button>
          </div>

          {searchOpen && (
            <form onSubmit={handleSearch} className={styles.searchRow}>
              <Search size={18} aria-hidden="true" />
              <input
                className={styles.searchInput}
                value={searchQ}
                onChange={(event) => setSearchQ(event.target.value)}
                placeholder="搜索项目、作者"
                autoFocus
              />
              <button className={styles.searchBtn} type="submit">搜索</button>
            </form>
          )}

          <div className={styles.featuredRail}>
            {projects.slice(0, 5).map((project) => {
              const title = project.display_name || project.name
              return (
                <article className={styles.featuredCard} key={`featured-${project.id}`}>
                  <img className={styles.featuredSurface} src={cardAsset} alt="" />
                  <div className={styles.featuredContent}>
                    <div className={styles.featuredTop}>
                      <img className={styles.avatar} src={avatarAsset} alt="" />
                      <span>
                        <button
                          type="button"
                          aria-label={reactions[`${project.id}:favorite`] ? '取消收藏' : '收藏'}
                          data-active={reactions[`${project.id}:favorite`]}
                          onClick={() => toggleReaction(project.id, 'favorite')}
                        ><img src={starAsset} alt="" /></button>
                        <button
                          type="button"
                          aria-label={reactions[`${project.id}:liked`] ? '取消点赞' : '点赞'}
                          data-active={reactions[`${project.id}:liked`]}
                          onClick={() => toggleReaction(project.id, 'liked')}
                        ><img src={heartAsset} alt="" /></button>
                      </span>
                    </div>
                    <strong>{title}</strong>
                    <p>{project.description || '这个项目还没有填写简介。'}</p>
                    <div className={styles.mediaRow}><span><Image /></span><span><Image /></span></div>
                    <button
                      className={styles.primaryAction}
                      type="button"
                      disabled={joiningId === project.id}
                      aria-label={primaryActionLabel(project, joinStatus[project.id])}
                      onClick={() => void runPrimaryAction(project)}
                    >
                      {joiningId === project.id
                        ? <Loader2 className={styles.spinner} />
                        : <img src={plazaChevronAsset} alt="" aria-hidden="true" />}
                    </button>
                  </div>
                </article>
              )
            })}
          </div>

          <div className={styles.allHeading}><h2>全部</h2></div>
          <div className={styles.projectList} data-testid="project-list">
            {projects.map((project) => (
              <article className={styles.projectRow} data-testid="project-row" data-project-id={project.id} key={project.id}>
                <img className={styles.thumbnail} src={thumbnailAsset} alt="" />
                <button className={styles.projectRowMain} type="button" onClick={() => void openProject(project)}>
                  <strong>{project.display_name || project.name}</strong>
                  <span>{project.description || '这个项目还没有填写简介。'}</span>
                </button>
                <button
                  className={styles.rowAction}
                  type="button"
                  disabled={joiningId === project.id}
                  aria-label={primaryActionLabel(project, joinStatus[project.id])}
                  onClick={() => void runPrimaryAction(project)}
                >
                  {joiningId === project.id
                    ? <Loader2 className={styles.spinner} />
                    : <img src={plazaChevronAsset} alt="" aria-hidden="true" />}
                </button>
              </article>
            ))}
          </div>

          {hasMore && (
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

function primaryActionLabel(project: PlazaProject, status?: JoinStatus): string {
  const title = project.display_name || project.name
  if (project.viewer_role || status === 'joined' || status === 'requested') return `进入${title}`
  if (project.join_mode === 'approval') return `申请加入${title}`
  if (project.join_mode === 'invite') return `查看${title}`
  return `加入${title}`
}

function mergeProjects(current: PlazaProject[], incoming: PlazaProject[]): PlazaProject[] {
  const seen = new Set(current.map((project) => project.id))
  return [...current, ...incoming.filter((project) => !seen.has(project.id))]
}
