import { useLocation, useNavigate } from 'react-router-dom'
import { useState } from 'react'
import { Plus, Search } from 'lucide-react'
import { useAuthStore } from '../../store/auth'
import { isLocalWorkbench } from '../../api/runtime'
import { useProjectStore } from '../conversation/useProjectStore'
import UserAvatar, { userDisplayName } from './UserAvatar'
import { presenceLabel, useMyPresence } from './useMyPresence'
import {
  ADMIN_RAIL_ITEM,
  LOCAL_RAIL_ITEMS,
  WORKSPACE_RAIL_ITEMS,
  workspaceForPath,
  type WorkspaceKey,
} from './navigationModel'
import styles from './ServerRail.module.css'

interface ServerRailProps {
  workspaceNavOpen: boolean
  onToggleWorkspaceNav: (workspace: WorkspaceKey, isCurrentWorkspace: boolean) => void
}

export default function ServerRail({ workspaceNavOpen, onToggleWorkspaceNav }: ServerRailProps) {
  const navigate = useNavigate()
  const { pathname } = useLocation()
  const user = useAuthStore((state) => state.user)
  const localMode = isLocalWorkbench()
  const presence = useMyPresence(!localMode)
  const [tooltip, setTooltip] = useState<{ text: string; y: number } | null>(null)
  const workspace = workspaceForPath(pathname)
  const isAdmin = user && ['admin', 'owner'].includes(user.role ?? '')
  const railItems = localMode
    ? LOCAL_RAIL_ITEMS
    : isAdmin
      ? [...WORKSPACE_RAIL_ITEMS, ADMIN_RAIL_ITEM]
      : WORKSPACE_RAIL_ITEMS

  const projects = useProjectStore((state) => state.projects)
  const activeProjectId = useProjectStore((state) => state.activeProjectId)

  async function openProject(id: string) {
    await useProjectStore.getState().selectProject(id)
    if (pathname !== '/workspace') navigate('/workspace')
  }

  function showTip(event: React.MouseEvent<HTMLElement>, text: string) {
    const rect = event.currentTarget.getBoundingClientRect()
    setTooltip({ text, y: rect.top + rect.height / 2 })
  }

  return (
    <nav className={styles.rail} aria-label="全局工作区导航">
      {!localMode && (
        <button
          className={[styles.homeButton, workspace === 'ai' ? styles.active : ''].join(' ')}
          type="button"
          title="AI 工作区"
          aria-label="AI 工作区"
          aria-expanded={workspace === 'ai' ? workspaceNavOpen : undefined}
          onClick={() => { navigate('/ai'); onToggleWorkspaceNav('ai', workspace === 'ai') }}
        >
          <span>一</span>
        </button>
      )}

      {railItems.map((item) => {
        const active = item.workspace === workspace
        const Icon = item.Icon
        return (
          <button
            key={item.path}
            className={[styles.avatar, active ? styles.active : ''].join(' ')}
            style={{ '--item-color': item.color, '--item-hover': item.hoverColor } as React.CSSProperties}
            aria-expanded={workspace === item.workspace ? workspaceNavOpen : undefined}
            onClick={() => { navigate(item.path); onToggleWorkspaceNav(item.workspace, workspace === item.workspace) }}
            onMouseEnter={(event) => showTip(event, item.label)}
            onMouseLeave={() => setTooltip(null)}
            title={item.label}
            aria-label={item.label}
            type="button"
          >
            <Icon className={styles.icon} aria-hidden="true" strokeWidth={2.3} />
          </button>
        )
      })}

      {!localMode && <div className={styles.divider} />}

      {!localMode && (
        <div className={styles.projectStack} aria-label="项目快捷入口">
          <button
            className={styles.projectAction}
            type="button"
            title="新建项目"
            aria-label="新建项目"
            onClick={() => navigate('/projects')}
          >
            <Plus size={16} aria-hidden="true" />
          </button>
          {projects.map((project) => {
            const isActiveProject = pathname === '/workspace' && project.id === activeProjectId
            const iconSrc = project.icon_data_url || project.icon || ''
            return (
              <button
                key={project.id}
                className={[styles.avatar, styles.projectAvatar, isActiveProject ? styles.active : ''].join(' ')}
                onClick={() => void openProject(project.id)}
                onMouseEnter={(event) => showTip(event, project.name)}
                onMouseLeave={() => setTooltip(null)}
                title={project.name}
                aria-label={project.name}
                type="button"
              >
                {iconSrc
                  ? <img src={iconSrc} alt="" className={styles.projectIcon} onError={(event) => { event.currentTarget.style.display = 'none' }} />
                  : <span className={styles.projectFallback}>{project.name[0]?.toUpperCase() ?? '?'}</span>}
              </button>
            )
          })}
        </div>
      )}

      {!localMode && <div className={styles.divider} />}

      {!localMode && (
        <button
          className={styles.utilityButton}
          type="button"
          title="项目中心"
          aria-label="项目中心"
          onClick={() => navigate('/projects')}
        >
          <Search size={16} aria-hidden="true" />
        </button>
      )}

      {!localMode && user && (
        <button
          className={[styles.avatar, styles.userAvatar].join(' ')}
          title={`${userDisplayName(user)} — ${presenceLabel(presence?.status)}`}
          aria-label={`${userDisplayName(user)} — ${presenceLabel(presence?.status)}`}
          onMouseEnter={(event) => showTip(event, `${userDisplayName(user)} · ${presenceLabel(presence?.status)}`)}
          onMouseLeave={() => setTooltip(null)}
          onClick={() => navigate('/account')}
          type="button"
        >
          <UserAvatar user={user} size="rail" showStatus presenceStatus={presence?.status} className={styles.railUserAvatar} />
        </button>
      )}

      {tooltip && (
        <div className={styles.tooltip} style={{ top: tooltip.y, transform: 'translateY(-50%)' }}>
          {tooltip.text}
        </div>
      )}
    </nav>
  )
}
