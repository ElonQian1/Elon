import { useEffect, useMemo, useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'
import { ChevronRight, FolderKanban, Plus, Store, UsersRound } from 'lucide-react'
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
    setSearchParams(tab === 'mine' ? {} : { tab })
  }

  async function openProject(projectId: string) {
    await useProjectStore.getState().selectProject(projectId)
    navigate('/')
  }

  async function handleCreated(project: { id?: string }) {
    setShowCreate(false)
    await loadProjects()
    if (project.id) await openProject(project.id)
  }

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <div className={styles.titleBlock}>
          <h1>项目中心</h1>
          <p>管理你的应用项目，也可以在项目广场发现公开应用。</p>
        </div>
        <button className={styles.createBtn} onClick={() => setShowCreate(true)} type="button">
          <Plus size={16} aria-hidden="true" />
          <span>新建项目</span>
        </button>
      </header>

      <div className={styles.tabs} role="tablist" aria-label="项目中心">
        <button
          className={styles.tab}
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
          className={styles.tab}
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

      <main className={styles.content}>
        {activeTab === 'mine' ? (
          <section className={styles.minePanel} aria-label="我的项目">
            <div className={styles.mineToolbar}>
              <input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="搜索我的项目"
              />
              <button type="button" onClick={() => switchTab('plaza')}>去项目广场</button>
            </div>

            {!projectsLoaded ? (
              <div className={styles.emptyState}>读取项目列表…</div>
            ) : filteredProjects.length === 0 ? (
              <div className={styles.emptyState}>
                <FolderKanban size={24} aria-hidden="true" />
                <strong>{query ? '没有匹配的项目' : '还没有项目'}</strong>
                <span>{query ? '换个关键词，或清空搜索。' : '可以新建项目，或去项目广场加入公开项目。'}</span>
              </div>
            ) : (
              <div className={styles.projectList}>
                {filteredProjects.map((project) => (
                  <ProjectRow key={project.id} project={project} onOpen={openProject} />
                ))}
              </div>
            )}
          </section>
        ) : (
          <ProjectPlazaView />
        )}
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

function ProjectRow({ project, onOpen }: { project: Project; onOpen: (projectId: string) => void }) {
  const iconSrc = project.icon_data_url || project.icon || ''
  return (
    <button className={styles.projectRow} type="button" onClick={() => onOpen(project.id)}>
      {iconSrc ? (
        <img className={styles.projectIcon} src={iconSrc} alt="" />
      ) : (
        <span className={styles.projectIconFallback}>{project.name[0]?.toUpperCase() || '项'}</span>
      )}
      <span className={styles.projectMain}>
        <strong>{project.name}</strong>
        <span>{project.description || '暂无简介'}</span>
        <em>
          <UsersRound size={13} aria-hidden="true" />
          {project.member_count ?? 1} 成员 · {roleLabel(project.my_role || project.role)} · {formatProjectDate(project.updated_at || project.created_at)}
        </em>
      </span>
      <ChevronRight className={styles.projectChevron} size={17} aria-hidden="true" />
    </button>
  )
}

function normalizeTab(value: string | null): ProjectCenterTab {
  return value === 'plaza' ? 'plaza' : 'mine'
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
