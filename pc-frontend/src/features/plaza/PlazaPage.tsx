import { useEffect, useState, useCallback } from 'react'
import { api } from '../../api/client'
import styles from './PlazaPage.module.css'

interface PlazaProject {
  id: string
  name: string
  display_name?: string
  description?: string
  template: string
  owner_account: string
  member_count: number
  join_mode: string        // "open" | "approval" | "invite" | "readonly"
  viewer_role?: string     // 已加入时有值
  latest_apk_url?: string
  icon_data_url?: string
  created_at: string
}

interface Filter {
  key: string
  label: string
  params?: Record<string, string | boolean>
}

const FILTERS: Filter[] = [
  { key: 'all',         label: '全部' },
  { key: 'installable', label: '可安装', params: { has_apk: true } },
  { key: 'no_approval', label: '无审批', params: { join_mode: 'open' } },
  { key: 'popular',     label: '最热门', params: { sort: 'members' } },
]

export default function PlazaPage() {
  const [projects, setProjects] = useState<PlazaProject[]>([])
  const [loading, setLoading] = useState(true)
  const [searchQ, setSearchQ] = useState('')
  const [activeFilter, setActiveFilter] = useState('all')
  const [joiningId, setJoiningId] = useState<string | null>(null)
  const [joinStatus, setJoinStatus] = useState<Record<string, 'joined' | 'requested' | 'error'>>({})

  const load = useCallback(async (q: string, filterKey: string) => {
    setLoading(true)
    const filter = FILTERS.find((f) => f.key === filterKey) ?? FILTERS[0]
    const params = new URLSearchParams({ limit: '30' })
    if (q) params.set('q', q)
    if (filter.params) {
      for (const [k, v] of Object.entries(filter.params)) {
        params.set(k, String(v))
      }
    }
    try {
      const data = await api.get<{ projects?: PlazaProject[] }>(
        `/api/store/projects?${params.toString()}`,
      )
      setProjects(data.projects ?? [])
    } catch { /* ignore */ }
    finally { setLoading(false) }
  }, [])

  useEffect(() => { load(searchQ, activeFilter) }, [searchQ, activeFilter])

  function handleSearch(e: React.FormEvent) {
    e.preventDefault()
    load(searchQ, activeFilter)
  }

  async function handleJoin(project: PlazaProject) {
    setJoiningId(project.id)
    try {
      if (project.join_mode === 'open') {
        await api.post(`/api/projects/${encodeURIComponent(project.id)}/join`, {})
        setJoinStatus((prev) => ({ ...prev, [project.id]: 'joined' }))
      } else {
        await api.post(`/api/projects/${encodeURIComponent(project.id)}/request-join`, {})
        setJoinStatus((prev) => ({ ...prev, [project.id]: 'requested' }))
      }
    } catch (err) {
      setJoinStatus((prev) => ({ ...prev, [project.id]: 'error' }))
      alert((err as { message?: string }).message ?? '操作失败')
    } finally {
      setJoiningId(null)
    }
  }

  return (
    <div className={styles.page}>
      {/* 顶部搜索 + 过滤 */}
      <div className={styles.toolbar}>
        <form onSubmit={handleSearch} className={styles.searchRow}>
          <input
            className={styles.searchInput}
            value={searchQ}
            onChange={(e) => setSearchQ(e.target.value)}
            placeholder="搜索项目名称或描述"
          />
          <button className={styles.searchBtn} type="submit">搜索</button>
        </form>
        <div className={styles.filterRow}>
          {FILTERS.map((f) => (
            <button
              key={f.key}
              className={[styles.filterBtn, activeFilter === f.key ? styles.filterActive : ''].join(' ')}
              onClick={() => setActiveFilter(f.key)}
              type="button"
            >
              {f.label}
            </button>
          ))}
        </div>
      </div>

      {/* 项目卡片网格 */}
      {loading ? (
        <div className={styles.loading}>读取项目广场…</div>
      ) : projects.length === 0 ? (
        <div className={styles.empty}>没有找到符合条件的项目</div>
      ) : (
        <div className={styles.grid}>
          {projects.map((p) => {
            const alreadyJoined = !!p.viewer_role || joinStatus[p.id] === 'joined'
            const requested = joinStatus[p.id] === 'requested'
            const isOpen = p.join_mode === 'open'
            const hasApk = !!p.latest_apk_url

            return (
              <div key={p.id} className={styles.card}>
                <div className={styles.cardHead}>
                  <div
                    className={styles.cardIcon}
                    style={p.icon_data_url ? { backgroundImage: `url(${p.icon_data_url})`, backgroundSize: 'cover' } : {}}
                  >
                    {!p.icon_data_url && (p.display_name ?? p.name)[0]?.toUpperCase()}
                  </div>
                  <div className={styles.cardTitle}>
                    <strong>{p.display_name ?? p.name}</strong>
                    <span>@{p.owner_account}</span>
                  </div>
                </div>

                {p.description && (
                  <p className={styles.cardDesc}>{p.description.slice(0, 80)}{p.description.length > 80 ? '…' : ''}</p>
                )}

                <div className={styles.cardMeta}>
                  <span>{p.member_count} 成员</span>
                  <span>{p.template}</span>
                  {hasApk && <span className={styles.apkPill}>可安装 APK</span>}
                  <span className={[styles.joinModePill, isOpen ? styles.openPill : ''].join(' ')}>
                    {p.join_mode === 'open' ? '直接加入' : p.join_mode === 'approval' ? '申请加入' : p.join_mode}
                  </span>
                </div>

                <div className={styles.cardActions}>
                  {hasApk && (
                    <a
                      href={p.latest_apk_url}
                      className={styles.apkBtn}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      下载 APK
                    </a>
                  )}
                  {alreadyJoined ? (
                    <span className={styles.joinedLabel}>已加入</span>
                  ) : requested ? (
                    <span className={styles.requestedLabel}>已申请</span>
                  ) : p.join_mode !== 'invite' ? (
                    <button
                      className={styles.joinBtn}
                      disabled={joiningId === p.id}
                      onClick={() => handleJoin(p)}
                      type="button"
                    >
                      {joiningId === p.id ? '处理中…' : isOpen ? '加入项目' : '申请加入'}
                    </button>
                  ) : null}
                </div>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}
