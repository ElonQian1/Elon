import { ChevronDown, ChevronRight, PanelLeftClose, Plus, Search } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useLocation, useNavigate } from 'react-router-dom'
import { useAuthStore } from '../../store/auth'
import { useProjectStore } from '../conversation/useProjectStore'
import {
  pathMatches,
  sectionsForWorkspace,
  workspaceForPath,
  type NavSection,
  type WorkspaceKey,
} from './navigationModel'
import styles from './WorkspaceNav.module.css'

const WORKSPACE_TITLES: Record<WorkspaceKey, string> = {
  ai: 'AI 工作区',
  projects: '项目工作区',
  messages: '消息',
  compute: '算力工作区',
  admin: '管理中心',
}

function Section({ section, pathname, onNavigate }: { section: NavSection; pathname: string; onNavigate: () => void }) {
  const [collapsed, setCollapsed] = useState(false)
  const navigate = useNavigate()
  const active = section.items.some((item) => pathMatches(pathname, item.path))

  return (
    <section className={styles.section}>
      <button
        className={[styles.sectionTitle, active ? styles.sectionTitleActive : ''].join(' ')}
        type="button"
        onClick={() => setCollapsed((value) => !value)}
        aria-expanded={!collapsed}
      >
        {collapsed ? <ChevronRight size={13} aria-hidden="true" /> : <ChevronDown size={13} aria-hidden="true" />}
        <span>{section.label}</span>
      </button>
      {!collapsed && (
        <div className={styles.itemList}>
          {section.items.map((item) => {
            const isActive = pathMatches(pathname, item.path)
            const Icon = item.Icon
            return (
              <button
                key={item.path}
                className={[styles.item, isActive ? styles.itemActive : ''].join(' ')}
                type="button"
                onClick={() => { navigate(item.path); onNavigate() }}
                title={item.label}
                aria-current={isActive ? 'page' : undefined}
              >
                <Icon size={16} aria-hidden="true" />
                <span>{item.label}</span>
              </button>
            )
          })}
        </div>
      )}
    </section>
  )
}

export default function WorkspaceNav({ onClose }: { onClose: () => void }) {
  const { pathname } = useLocation()
  const navigate = useNavigate()
  const user = useAuthStore((state) => state.user)
  const projects = useProjectStore((state) => state.projects)
  const activeProjectId = useProjectStore((state) => state.activeProjectId)
  const workspace = workspaceForPath(pathname)
  const sections = useMemo(() => sectionsForWorkspace(workspace), [workspace])
  const isAdmin = user && ['admin', 'owner'].includes(user.role ?? '')
  const currentProject = projects.find((project) => project.id === activeProjectId)

  return (
    <aside className={styles.nav} aria-label={`${WORKSPACE_TITLES[workspace]}导航`}>
      <header className={styles.header}>
        <div className={styles.titleBlock}>
          <span className={styles.kicker}>工作区</span>
          <strong>{WORKSPACE_TITLES[workspace]}</strong>
        </div>
        <div className={styles.headerActions}>
          {workspace === 'projects' && (
            <button className={styles.iconButton} type="button" title="新建项目" aria-label="新建项目" onClick={() => { navigate('/projects'); onClose() }}>
              <Plus size={16} aria-hidden="true" />
            </button>
          )}
          <button className={styles.iconButton} type="button" title="收起工作区导航" aria-label="收起工作区导航" onClick={onClose}>
            <PanelLeftClose size={16} aria-hidden="true" />
          </button>
        </div>
      </header>

      {workspace === 'projects' && projects.length > 0 && (
        <div className={styles.currentProject}>
          <span className={styles.currentProjectDot} aria-hidden="true" />
          <span>{currentProject?.name ?? '选择一个项目开始'}</span>
        </div>
      )}

      <button className={styles.searchHint} type="button" onClick={() => { navigate('/projects'); onClose() }}>
        <Search size={14} aria-hidden="true" />
        <span>搜索工作区</span>
        <kbd>⌘K</kbd>
      </button>

      <div className={styles.sections}>
        {sections.map((section) => <Section key={section.id} section={section} pathname={pathname} onNavigate={onClose} />)}
        {workspace === 'admin' && !isAdmin && <p className={styles.notice}>当前账号没有管理权限。</p>}
      </div>

      <footer className={styles.footer}>功能按工作区分组，可点击标题折叠。</footer>
    </aside>
  )
}
