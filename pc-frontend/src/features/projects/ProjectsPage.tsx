import { useEffect, useMemo, useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'
import { CalendarDays, ChevronRight, Code2, FolderKanban, Plus, Settings, Store, UsersRound } from 'lucide-react'
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
  const selectedProjectId = searchParams.get('project') ?? ''
  const selectedProject = projects.find((project) => project.id === selectedProjectId)

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

  function showProjectHome(projectId: string) {
    setSearchParams({ project: projectId })
  }

  async function continueProject(projectId: string) {
    await useProjectStore.getState().selectProject(projectId)
    navigate('/workspace')
  }

  async function handleCreated(project: { id?: string }) {
    setShowCreate(false)
    await loadProjects()
    if (project.id) showProjectHome(project.id)
  }

  return (
    <div className={styles.page}>
      <aside className={styles.sidebar} aria-label="项目中心导航">
        <div className={styles.sidebarHeader}>
          <div>
            <strong>项目中心</strong>
            <span>我的项目工作台</span>
          </div>
          <button className={styles.sideCreateBtn} onClick={() => setShowCreate(true)} type="button" title="新建项目">
            <Plus size={16} aria-hidden="true" />
          </button>
        </div>

        <div className={styles.sideNav} role="tablist" aria-label="项目中心">
          <button
            className={styles.sideNavBtn}
            data-active={activeTab === 'mine' || selectedProject ? 'true' : 'false'}
            role="tab"
            aria-selected={activeTab === 'mine' || !!selectedProject}
            onClick={() => switchTab('mine')}
            type="button"
          >
            <FolderKanban size={16} aria-hidden="true" />
            <span>我的项目</span>
            <strong>{projects.length}</strong>
          </button>
          <button
            className={styles.sideNavBtn}
            data-active={activeTab === 'plaza' && !selectedProject ? 'true' : 'false'}
            role="tab"
            aria-selected={activeTab === 'plaza' && !selectedProject}
            onClick={() => switchTab('plaza')}
            type="button"
          >
            <Store size={16} aria-hidden="true" />
            <span>项目广场</span>
          </button>
        </div>

        <div className={styles.sideSectionTitle}>我的项目</div>
        <div className={styles.sideProjectList}>
          {!projectsLoaded && <div className={styles.sideEmpty}>读取中...</div>}
          {projectsLoaded && projects.length === 0 && <div className={styles.sideEmpty}>暂无项目</div>}
          {projects.map((project) => (
            <button
              key={project.id}
              className={styles.sideProject}
              data-active={selectedProjectId === project.id ? 'true' : undefined}
              onClick={() => showProjectHome(project.id)}
              type="button"
              title={project.name}
            >
              <ProjectIcon project={project} compact />
              <span>
                <strong>{project.name}</strong>
                <small>{roleLabel(project.my_role || project.role)}</small>
              </span>
            </button>
          ))}
        </div>
      </aside>

      <main className={[styles.main, activeTab === 'plaza' && !selectedProject ? styles.mainPlaza : ''].join(' ')}>
        {activeTab === 'mine' && !selectedProject && (
          <header className={styles.header}>
            <div className={styles.titleBlock}>
              <h1>我的项目</h1>
              <p>管理你的应用项目，进入项目后继续查看频道和会话。</p>
            </div>
            <button className={styles.createBtn} onClick={() => setShowCreate(true)} type="button">
              <Plus size={16} aria-hidden="true" />
              <span>新建项目</span>
            </button>
          </header>
        )}

        <div className={styles.content}>
          {selectedProject ? (
            <ProjectHome
              project={selectedProject}
              onContinue={continueProject}
              onSettings={(projectId) => navigate(`/projects/${projectId}`)}
              onMembers={(projectId) => navigate(`/projects/${projectId}/members`)}
            />
          ) : activeTab === 'mine' ? (
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
                <div className={styles.emptyState}>读取项目列表...</div>
              ) : filteredProjects.length === 0 ? (
                <div className={styles.emptyState}>
                  <FolderKanban size={24} aria-hidden="true" />
                  <strong>{query ? '没有匹配的项目' : '还没有项目'}</strong>
                  <span>{query ? '换个关键词，或清空搜索。' : '可以新建项目，或去项目广场加入公开项目。'}</span>
                </div>
              ) : (
                <div className={styles.projectList}>
                  {filteredProjects.map((project) => (
                    <ProjectRow key={project.id} project={project} onOpen={showProjectHome} />
                  ))}
                </div>
              )}
            </section>
          ) : (
            <ProjectPlazaView />
          )}
        </div>
      </main>

      <aside className={styles.rightPanel} aria-label="项目中心侧栏">
        <section className={styles.rightSection}>
          <span className={styles.rightEyebrow}>当前</span>
          <strong>{selectedProject ? '正在查看项目首页' : activeTab === 'plaza' ? '正在浏览项目广场' : '正在管理我的项目'}</strong>
          <p>{selectedProject ? '这里先展示项目资料和入口；点击继续开发后才进入项目频道和会话工作台。' : activeTab === 'plaza' ? '中间区域默认展示公开项目，左侧保留你的项目入口。' : '从左侧或列表进入项目后，会先看到项目首页。'}</p>
        </section>

        <section className={styles.rightStats} aria-label="项目统计">
          <div>
            <strong>{projects.length}</strong>
            <span>我的项目</span>
          </div>
          <div>
            <strong>{filteredProjects.length}</strong>
            <span>当前匹配</span>
          </div>
        </section>

        <section className={styles.rightActions} aria-label="快捷操作">
          {selectedProject && (
            <button type="button" onClick={() => continueProject(selectedProject.id)}>
              <Code2 size={15} aria-hidden="true" />
              <span>继续开发</span>
            </button>
          )}
          <button type="button" onClick={() => switchTab('plaza')}>
            <Store size={15} aria-hidden="true" />
            <span>浏览项目广场</span>
          </button>
          <button type="button" onClick={() => switchTab('mine')}>
            <FolderKanban size={15} aria-hidden="true" />
            <span>查看我的项目</span>
          </button>
          <button type="button" onClick={() => setShowCreate(true)}>
            <Plus size={15} aria-hidden="true" />
            <span>新建项目</span>
          </button>
        </section>

        <section className={styles.rightSection}>
          <span className={styles.rightEyebrow}>结构</span>
          <p>左侧负责浏览项目，中间展示项目首页或广场；只有继续开发时才进入频道、会话和成员侧栏。</p>
        </section>
      </aside>

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

function ProjectHome({
  project,
  onContinue,
  onSettings,
  onMembers,
}: {
  project: Project
  onContinue: (projectId: string) => void
  onSettings: (projectId: string) => void
  onMembers: (projectId: string) => void
}) {
  return (
    <section className={styles.projectHome} aria-label="项目首页">
      <div className={styles.projectHero}>
        <ProjectIcon project={project} />
        <div className={styles.projectHeroText}>
          <span>{roleLabel(project.my_role || project.role)}</span>
          <h1>{project.name}</h1>
          <p>{project.description || '这个项目还没有填写简介。'}</p>
        </div>
        <button className={styles.continueBtn} type="button" onClick={() => onContinue(project.id)}>
          <Code2 size={16} aria-hidden="true" />
          <span>继续开发</span>
        </button>
      </div>

      <div className={styles.projectHomeGrid}>
        <div className={styles.projectHomeCard}>
          <strong>项目资料</strong>
          <dl>
            <div><dt>项目 ID</dt><dd>{project.id}</dd></div>
            <div><dt>模板</dt><dd>{project.template || '-'}</dd></div>
            <div><dt>来源</dt><dd>{project.source_type || '项目中心'}</dd></div>
          </dl>
        </div>

        <div className={styles.projectHomeCard}>
          <strong>状态</strong>
          <dl>
            <div><dt>成员</dt><dd>{project.member_count ?? 1}</dd></div>
            <div><dt>角色</dt><dd>{roleLabel(project.my_role || project.role)}</dd></div>
            <div><dt>更新</dt><dd>{formatProjectDate(project.updated_at || project.created_at)}</dd></div>
          </dl>
        </div>

        <div className={styles.projectHomeActions}>
          <button type="button" onClick={() => onContinue(project.id)}>
            <Code2 size={15} aria-hidden="true" />
            <span>继续开发</span>
          </button>
          <button type="button" onClick={() => onSettings(project.id)}>
            <Settings size={15} aria-hidden="true" />
            <span>项目设置</span>
          </button>
          <button type="button" onClick={() => onMembers(project.id)}>
            <UsersRound size={15} aria-hidden="true" />
            <span>成员管理</span>
          </button>
        </div>
      </div>

      <div className={styles.projectHomeNote}>
        <CalendarDays size={15} aria-hidden="true" />
        <span>项目首页用于浏览和管理；开发会话、频道和 AI 运行过程会在继续开发后打开。</span>
      </div>
    </section>
  )
}

function ProjectRow({ project, onOpen }: { project: Project; onOpen: (projectId: string) => void }) {
  return (
    <button className={styles.projectRow} type="button" onClick={() => onOpen(project.id)}>
      <ProjectIcon project={project} />
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
