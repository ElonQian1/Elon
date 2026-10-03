import { useLocation, useNavigate } from 'react-router-dom'
import { useState } from 'react'
import { MonitorCog, Plus, Search } from 'lucide-react'
import { useAuthStore } from '../../store/auth'
import { useProjectStore } from '../conversation/useProjectStore'
import UserAvatar, { userDisplayName } from './UserAvatar'
import { presenceLabel, useMyPresence } from './useMyPresence'
import { useWorkbenchConnection } from './useWorkbenchConnection'
import {
  ADMIN_RAIL_ITEM,
  WORKSPACE_RAIL_ITEMS,
  workspaceForPath,
} from './navigationModel'
import styles from './ServerRail.module.css'

export default function ServerRail() {
  const navigate = useNavigate()
  const { pathname } = useLocation()
  const user = useAuthStore((state) => state.user)
  const { cloudState } = useWorkbenchConnection()
  const presence = useMyPresence(cloudState !== 'offline')
  const connectionLabel = cloudState === 'online' ? '云端已连接' : cloudState === 'offline' ? '云端暂时不可用' : '正在检查云端连接'
  const [tooltip, setTooltip] = useState<{ text: string; y: number } | null>(null)
  const workspace = workspaceForPath(pathname)
  const isAdmin = user && ['admin', 'owner'].includes(user.role ?? '')
  const railItems = isAdmin
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
      <button
        className={[styles.homeButton, workspace === 'ai' ? styles.active : ''].join(' ')}
        type="button"
        title="AI 工作区"
        aria-label="AI 工作区"
        onClick={() => navigate('/ai')}
      >
        <span>一</span>
      </button>

      {railItems.map((item) => {
        const active = item.workspace === workspace
        const Icon = item.Icon
        return (
          <button
            key={item.path}
            className={[styles.avatar, active ? styles.active : ''].join(' ')}
            style={{ '--item-color': item.color, '--item-hover': item.hoverColor } as React.CSSProperties}
            onClick={() => navigate(item.path)}
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

      <div className={styles.divider} />

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

      <div className={styles.divider} />

      <button
        className={styles.utilityButton}
        type="button"
        title="项目中心"
        aria-label="项目中心"
        onClick={() => navigate('/projects')}
      >
        <Search size={16} aria-hidden="true" />
      </button>

      <button
        className={styles.utilityButton}
        type="button"
        title={`连接与设备 · ${connectionLabel}`}
        aria-label="连接与设备"
        aria-current={pathname === '/node' ? 'page' : undefined}
        data-cloud-state={cloudState}
        onClick={() => navigate('/node')}
      >
        <MonitorCog size={16} aria-hidden="true" />
      </button>

      {user && (
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
