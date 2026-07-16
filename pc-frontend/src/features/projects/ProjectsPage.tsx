import { useEffect, useMemo, useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'
import {
  ChevronRight,
  FolderKanban,
  LoaderCircle,
  Plus,
  Settings,
  Store,
  UsersRound,
} from 'lucide-react'
import { CreateProjectModal } from './CreateProjectModal'
import { useProjectStore } from '../conversation/useProjectStore'
import type { Project } from '../conversation/types'
import ProjectPlazaView from '../plaza/ProjectPlazaView'
import styles from './ProjectsPage.module.css'

type ProjectCenterTab = 'mine' | 'plaza'

export default function ProjectsPage() {
  const navigate = useNavigate()
  const [searchParams, setSearchParams] = useSearchParams()
  const [showCreate, setShowCreate] = useState(false)
  const [query, setQuery] = useState('')
  const [openingProjectId, setOpeningProjectId] = useState('')
  const projects = useProjectStore((state) => state.projects)
  const projectsLoaded = useProjectStore((state) => state.projectsLoaded)
  const loadProjects = useProjectStore((state) => state.loadProjects)
  const activeTab = normalizeTab(searchParams.get('tab'))

  useEffect(() => {
    loadProjects().catch(() => {})
  }, [loadProjects])

  const filteredProjects = useMemo(() => {
    const keyword = query.trim().toLowerCase()
    if (!keyword) return projects
    return projects.filter((project) => {
      const haystack = [
        project.name,
        project.description,
        project.template,
        project.role,
        project.my_role,
      ].filter(Boolean).join(' ').toLowerCase()
      return haystack.includes(keyword)
    })
  }, [projects, query])

  function switchTab(tab: ProjectCenterTab) {
    setSearchParams(tab === 'plaza' ? {} : { tab })
  }

  async function openProject(projectId: string) {
    if (!projectId || openingProjectId) return
    setOpeningProjectId(projectId)
    try {
      await useProjectStore.getState().selectProject(projectId)
      navigate('/workspace')
    } finally {
      setOpeningProjectId('')
    }
  }

  async function handleCreated(project: { id?: string }) {
    setShowCreate(false)
    await loadProjects()
    if (project.id) await openProject(project.id)
  }

  return (
    <div className={styles.page}>
      <aside className={styles.sidebar} aria-label="项目中心导航">
        <div className={styles.sidebarHeader}>
          <div>
            <strong>项目中心</strong>
            <span>选择项目，直接开始工作</span>
          </div>
          <button className={styles.sideCreateBtn} onClick={() => setShowCreate(true)} type="button" title="新建项目">
            <Plus size={16} aria-hidden="true" />
          </button>
        </div>

        <div className={styles.sideNav} role="tablist" aria-label="项目中心">
          <button
            className={styles.sideNavBtn}
            data-active={activeTab === 'mine' ? 'true' : 'false'}
            role="tab"
            aria-selected={activeTab === 'mine'}
            onClick={() => switchTab('mine')}
            type="button"
          >
            <FolderKanban size={16} aria-hidden="true" />
            <span>我的项目</span>
            <strong>{projects.length}</strong>
          </button>
          <button
            className={styles.sideNavBtn}
            data-active={activeTab === 'plaza' ? 'true' : 'false'}
            role="tab"
            aria-selected={activeTab === 'plaza'}
            onClick={() => switchTab('plaza')}
            type="button"
          >
            <Store size={16} aria-hidden="true" />
            <span>项目广场</span>
          </button>
        </div>

        <div className={styles.sideSectionTitle}>快速进入</div>
        <div className={styles.sideProjectList}>
          {!projectsLoaded && <div className={styles.sideEmpty}>读取中...</div>}
          {projectsLoaded && projects.length === 0 && <div className={styles.sideEmpty}>暂无项目</div>}
          {projects.map((project) => {
            const opening = openingProjectId === project.id
            return (
              <button
                key={project.id}
                className={styles.sideProject}
                data-opening={opening ? 'true' : undefined}
                onClick={() => void openProject(project.id)}
                type="button"
                title={`进入 ${project.name}`}
                disabled={!!openingProjectId}
              >
                <ProjectIcon project={project} compact />
                <span>
                  <strong>{project.name}</strong>
                  <small>{opening ? '正在打开项目首页…' : roleLabel(project.my_role || project.role)}</small>
                </span>
                {opening && <LoaderCircle className={styles.spinner} size={14} aria-hidden="true" />}
              </button>
            )
          })}
        </div>
      </aside>

      <main className={styles.main}>
        {activeTab === 'mine' && (
          <header className={styles.header}>
            <div className={styles.titleBlock}>
              <span>我的工作空间</span>
              <h1>我的项目</h1>
              <p>点击项目直接进入介绍页、频道和会话；设置与成员管理保留在次级入口。</p>
            </div>
            <div className={styles.headerActions}>
              <button className={styles.plazaBtn} onClick={() => switchTab('plaza')} type="button">
                <Store size={16} aria-hidden="true" />
                <span>浏览项目广场</span>
              </button>
              <button className={styles.createBtn} onClick={() => setShowCreate(true)} type="button">
                <Plus size={16} aria-hidden="true" />
                <span>新建项目</span>
              </button>
            </div>
          </header>
        )}

        <div className={styles.content}>
          {activeTab === 'mine' ? (
            <section className={styles.minePanel} aria-label="我的项目">
              <div className={styles.mineToolbar}>
                <label>
                  <span className={styles.srOnly}>搜索我的项目</span>
                  <input
                    value={query}
                    onChange={(event) => setQuery(event.target.value)}
                    placeholder="搜索项目名称、简介或模板"
                  />
                </label>
                <div className={styles.projectCount}>
                  <strong>{filteredProjects.length}</strong>
                  <span>{query ? '个匹配项目' : '个可用项目'}</span>
                </div>
              </div>

              {!projectsLoaded ? (
                <div className={styles.emptyState}>读取项目列表...</div>
              ) : filteredProjects.length === 0 ? (
                <div className={styles.emptyState}>
                  <FolderKanban size={28} aria-hidden="true" />
                  <strong>{query ? '没有匹配的项目' : '还没有项目'}</strong>
                  <span>{query ? '换个关键词，或清空搜索。' : '新建项目，或从项目广场加入一个公开项目。'}</span>
                </div>
              ) : (
                <div className={styles.projectList}>
                  {filteredProjects.map((project) => (
                    <ProjectRow
                      key={project.id}
                      project={project}
                      opening={openingProjectId === project.id}
                      disabled={!!openingProjectId}
                      onOpen={openProject}
                      onManage={(projectId) => navigate(`/projects/${projectId}`)}
                    />
                  ))}
                </div>
              )}
            </section>
          ) : (
            <ProjectPlazaView />
          )}
        </div>
      </main>

      {showCreate && (
        <CreateProjectModal
          quickMode
          onClose={() => setShowCreate(false)}
          onCreated={handleCreated}
        />
      )}
    </div>
  )
}

function ProjectRow({
  project,
  opening,
  disabled,
  onOpen,
  onManage,
}: {
  project: Project
  opening: boolean
  disabled: boolean
  onOpen: (projectId: string) => void
  onManage: (projectId: string) => void
}) {
  return (
    <article className={styles.projectRow} data-opening={opening ? 'true' : undefined}>
      <button className={styles.projectOpen} type="button" onClick={() => void onOpen(project.id)} disabled={disabled}>
        <ProjectIcon project={project} />
        <span className={styles.projectMain}>
          <strong>{project.name}</strong>
          <span>{project.description || '这个项目还没有填写简介。'}</span>
          <em>
            <UsersRound size={13} aria-hidden="true" />
            {project.member_count ?? 1} 位成员 · {roleLabel(project.my_role || project.role)} · {formatProjectDate(project.updated_at || project.created_at)}
          </em>
        </span>
        <span className={styles.openHint}>
          {opening ? <LoaderCircle className={styles.spinner} size={15} aria-hidden="true" /> : <ChevronRight size={17} aria-hidden="true" />}
          {opening ? '正在打开' : '进入项目'}
        </span>
      </button>
      <button
        className={styles.projectManage}
        type="button"
        onClick={() => onManage(project.id)}
        title="管理项目"
        aria-label={`管理 ${project.name}`}
      >
        <Settings size={15} aria-hidden="true" />
      </button>
    </article>
  )
}

function ProjectIcon({ project, compact = false }: { project: Project; compact?: boolean }) {
  const iconSrc = project.icon_data_url || project.icon || ''
  const className = compact ? styles.sideProjectIcon : styles.projectIcon
  const fallbackClassName = compact ? styles.sideProjectIconFallback : styles.projectIconFallback
  return iconSrc ? (
    <img className={className} src={iconSrc} alt="" />
  ) : (
    <span className={fallbackClassName}>{project.name[0]?.toUpperCase() || '项'}</span>
  )
}

function normalizeTab(value: string | null): ProjectCenterTab {
  return value === 'mine' ? 'mine' : 'plaza'
}

function roleLabel(role?: string): string {
  if (role === 'owner') return '拥有者'
  if (role === 'admin') return '管理员'
  if (role === 'editor') return '编辑者'
  if (role === 'observer') return '观察者'
  if (role === 'member') return '成员'
  return '项目成员'
}

function formatProjectDate(value?: string): string {
  if (!value) return '最近更新'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return '最近更新'
  return new Intl.DateTimeFormat('zh-CN', { month: '2-digit', day: '2-digit' }).format(date)
}
