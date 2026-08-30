import { ChevronDown, ChevronRight } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useLocation, useNavigate } from 'react-router-dom'
import { useAuthStore } from '../../store/auth'
import {
  pathMatches,
  sectionsForWorkspace,
  workspaceForPath,
  type NavSection,
  type WorkspaceKey,
} from './navigationModel'
import styles from './WorkspaceFeatureNav.module.css'

const WORKSPACE_TITLES: Record<WorkspaceKey, string> = {
  ai: 'AI 工作区',
  projects: '项目工作区',
  messages: '消息',
  compute: '算力工作区',
  admin: '管理中心',
}

function FeatureSection({
  section,
  pathname,
  excludedPaths,
}: {
  section: NavSection
  pathname: string
  excludedPaths: string[]
}) {
  const [collapsed, setCollapsed] = useState(false)
  const navigate = useNavigate()
  const items = section.items.filter((item) => !excludedPaths.some((path) => pathMatches(item.path, path)))
  if (items.length === 0) return null
  const active = items.some((item) => pathMatches(pathname, item.path))

  return (
    <section className={styles.section}>
      <button
        className={[styles.sectionTitle, active ? styles.sectionTitleActive : ''].join(' ')}
        type="button"
        onClick={() => setCollapsed((value) => !value)}
        aria-expanded={!collapsed}
      >
        {collapsed ? <ChevronRight size={12} aria-hidden="true" /> : <ChevronDown size={12} aria-hidden="true" />}
        <span>{section.label}</span>
      </button>
      {!collapsed && (
        <div className={styles.itemList}>
          {items.map((item) => {
            const Icon = item.Icon
            const activeItem = pathMatches(pathname, item.path)
            return (
              <button
                key={item.path}
                className={[styles.item, activeItem ? styles.itemActive : ''].join(' ')}
                type="button"
                onClick={() => navigate(item.path)}
                title={item.label}
                aria-current={activeItem ? 'page' : undefined}
              >
                <Icon size={14} aria-hidden="true" />
                <span>{item.label}</span>
              </button>
            )
          })}
        </div>
      )}
    </section>
  )
}

export default function WorkspaceFeatureNav({ excludedPaths = [] }: { excludedPaths?: string[] }) {
  const { pathname } = useLocation()
  const user = useAuthStore((state) => state.user)
  const workspace = workspaceForPath(pathname)
  const sections = useMemo(() => sectionsForWorkspace(workspace), [workspace])
  const isAdmin = user && ['admin', 'owner'].includes(user.role ?? '')

  return (
    <nav className={styles.nav} aria-label={`${WORKSPACE_TITLES[workspace]}功能导航`}>
      <div className={styles.heading}>
        <span>工作区功能</span>
        <strong>{WORKSPACE_TITLES[workspace]}</strong>
      </div>
      <div className={styles.sections}>
        {sections.map((section) => (
          <FeatureSection key={section.id} section={section} pathname={pathname} excludedPaths={excludedPaths} />
        ))}
        {workspace === 'admin' && !isAdmin && <p className={styles.notice}>当前账号没有管理权限。</p>}
      </div>
    </nav>
  )
}
