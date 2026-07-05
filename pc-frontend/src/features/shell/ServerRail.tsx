import { useNavigate, useLocation } from 'react-router-dom'
import { useState } from 'react'
import type { LucideIcon } from 'lucide-react'
import { Bot, Boxes, MonitorCog, UsersRound, Mic2 } from 'lucide-react'
import { useAuthStore } from '../../store/auth'
import { useProjectStore } from '../conversation/useProjectStore'
import UserAvatar, { userDisplayName } from './UserAvatar'
import { presenceLabel, useMyPresence } from './useMyPresence'
import styles from './ServerRail.module.css'

interface RailItem {
  path: string
  Icon: LucideIcon
  label: string
  color: string
  hoverColor: string
}

const RAIL_ITEMS: RailItem[] = [
  { path: '/ai',      Icon: Bot,          label: '一龙 AI',   color: '#2a2b2f', hoverColor: '#34363b' },
  { path: '/projects', Icon: Boxes,       label: '项目中心',  color: '#2a2b2f', hoverColor: '#34363b' },
  { path: '/friends', Icon: UsersRound,   label: '好友',      color: '#2a2b2f', hoverColor: '#34363b' },
  { path: '/node',    Icon: MonitorCog,   label: '分享算力',  color: '#2a2b2f', hoverColor: '#34363b' },
  { path: '/voice',   Icon: Mic2,         label: 'AI 声音',  color: '#2a2b2f', hoverColor: '#34363b' },
]

export default function ServerRail() {
  const navigate = useNavigate()
  const { pathname } = useLocation()
  const user = useAuthStore((s) => s.user)
  const presence = useMyPresence()
  const [tooltip, setTooltip] = useState<{ text: string; y: number } | null>(null)

  // 项目列表（从 store 读取，实时响应）
  const projects = useProjectStore((s) => s.projects)
  const activeProjectId = useProjectStore((s) => s.activeProjectId)

  function isActive(path: string) {
    return pathname.startsWith(path)
  }

  function handleRailClick(path: string) {
    // 点击「项目中心」时清空当前会话项目，避免项目快捷入口继续高亮。
    if (path === '/projects' && activeProjectId) {
      useProjectStore.getState().selectProject('')
      navigate('/projects')
      return
    }
    navigate(path)
  }

  async function openProject(id: string) {
    // 先更新状态（高亮立即生效），再导航到 /
    await useProjectStore.getState().selectProject(id)
    if (pathname !== '/') navigate('/')
  }

  function showTip(e: React.MouseEvent<HTMLElement>, text: string) {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
    setTooltip({ text, y: rect.top + rect.height / 2 })
  }

  return (
    <nav className={styles.rail}>
      {RAIL_ITEMS.map((item) => {
        const active = isActive(item.path)
        const Icon = item.Icon
        return (
          <button
            key={item.path}
            className={[styles.avatar, active ? styles.active : ''].join(' ')}
            style={{ '--item-color': item.color, '--item-hover': item.hoverColor } as React.CSSProperties}
            onClick={() => handleRailClick(item.path)}
            onMouseEnter={(e) => showTip(e, item.label)}
            onMouseLeave={() => setTooltip(null)}
            title={item.label}
            type="button"
          >
            <Icon className={styles.icon} aria-hidden="true" strokeWidth={2.3} />
          </button>
        )
      })}

      {/* ── 项目列表分隔线 ── */}
      {projects.length > 0 && <div className={styles.divider} />}

      <div className={styles.projectStack} aria-label="项目快捷入口">
        {projects.map((p) => {
          const isActiveProject = p.id === activeProjectId
          const iconSrc = p.icon_data_url || p.icon || ''
          return (
            <button
              key={p.id}
              className={[styles.avatar, styles.projectAvatar, isActiveProject ? styles.active : ''].join(' ')}
              onClick={() => openProject(p.id)}
              onMouseEnter={(e) => showTip(e, p.name)}
              onMouseLeave={() => setTooltip(null)}
              title={p.name}
              type="button"
            >
              {iconSrc
                ? <img src={iconSrc} alt="" className={styles.projectIcon} onError={(e) => { (e.currentTarget as HTMLImageElement).style.display = 'none' }} />
                : <span className={styles.projectFallback}>{p.name[0]?.toUpperCase() ?? '?'}</span>
              }
            </button>
          )
        })}
      </div>

      <div className={styles.divider} />

      {/* 账号头像 → 点击进账号页 */}
      {user && (
        <button
          className={[styles.avatar, styles.userAvatar].join(' ')}
          title={`${userDisplayName(user)} — ${presenceLabel(presence?.status)}`}
          aria-label={`${userDisplayName(user)} — ${presenceLabel(presence?.status)}`}
          onMouseEnter={(e) => showTip(e, `${userDisplayName(user)} · ${presenceLabel(presence?.status)}`)}
          onMouseLeave={() => setTooltip(null)}
          onClick={() => navigate('/account')}
          type="button"
        >
          <UserAvatar user={user} size="rail" showStatus presenceStatus={presence?.status} className={styles.railUserAvatar} />
        </button>
      )}

      {/* Tooltip */}
      {tooltip && (
        <div className={styles.tooltip} style={{ top: tooltip.y, transform: 'translateY(-50%)' }}>
          {tooltip.text}
        </div>
      )}
    </nav>
  )
}
