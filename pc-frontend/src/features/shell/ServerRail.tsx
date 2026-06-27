import { useNavigate, useLocation } from 'react-router-dom'
import { useState } from 'react'
import { useAuthStore } from '../../store/auth'
import { useProjectStore } from '../conversation/useProjectStore'
import styles from './ServerRail.module.css'

interface RailItem {
  path: string
  emoji: string
  label: string
  color: string
  hoverColor: string
}

const RAIL_ITEMS: RailItem[] = [
  { path: '/ai',      emoji: '🤖', label: '一龙 AI',   color: '#243246', hoverColor: '#3c6fa2' },
  { path: '/',        emoji: '📦', label: '项目对话',  color: '#2c2e35', hoverColor: '#4f5d71' },
  { path: '/friends', emoji: '👥', label: '好友',      color: '#2c2e35', hoverColor: '#4f5d71' },
  { path: '/plaza',   emoji: '🏪', label: '项目广场',  color: '#2c2e35', hoverColor: '#735db9' },
  { path: '/doctor',  emoji: '🩺', label: '电脑医生',  color: '#283342', hoverColor: '#315d72' },
  { path: '/node',    emoji: '🖥️', label: '分享算力',  color: '#2c2e35', hoverColor: '#365b44' },
  { path: '/voice',   emoji: '🎙️', label: 'AI 声音',  color: '#2f2a3a', hoverColor: '#7a4f9a' },
]

export default function ServerRail() {
  const navigate = useNavigate()
  const { pathname } = useLocation()
  const user = useAuthStore((s) => s.user)
  const [tooltip, setTooltip] = useState<{ text: string; y: number } | null>(null)

  // 项目列表（从 store 读取，实时响应）
  const projects = useProjectStore((s) => s.projects)
  const activeProjectId = useProjectStore((s) => s.activeProjectId)

  function isActive(path: string) {
    if (path === '/') return pathname === '/' && !activeProjectId
    return pathname.startsWith(path)
  }

  async function openProject(id: string) {
    // Discord 式：直接切换项目，停在 / 页面
    if (pathname !== '/') navigate('/')
    await useProjectStore.getState().selectProject(id)
  }

  function showTip(e: React.MouseEvent<HTMLElement>, text: string) {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
    setTooltip({ text, y: rect.top + rect.height / 2 })
  }

  return (
    <nav className={styles.rail}>
      {RAIL_ITEMS.map((item) => {
        const active = isActive(item.path)
        return (
          <button
            key={item.path}
            className={[styles.avatar, active ? styles.active : ''].join(' ')}
            style={{ '--item-color': item.color, '--item-hover': item.hoverColor } as React.CSSProperties}
            onClick={() => navigate(item.path)}
            onMouseEnter={(e) => showTip(e, item.label)}
            onMouseLeave={() => setTooltip(null)}
            title={item.label}
            type="button"
          >
            <span className={styles.icon}>{item.emoji}</span>
          </button>
        )
      })}

      {/* ── 项目列表分隔线 ── */}
      {projects.length > 0 && <div className={styles.divider} />}

      {/* ── 每个项目的 logo 按钮 ── */}
      {projects.map((p) => {
        const isActiveProject = p.id === activeProjectId && pathname === '/'
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

      <div className={styles.divider} />

      <div className={styles.spacer} />

      {/* 账号头像 → 点击进账号页 */}
      {user && (
        <button
          className={[styles.avatar, styles.userAvatar].join(' ')}
          title={`${user.nickname ?? user.account} — 账号设置`}
          onMouseEnter={(e) => showTip(e, user.nickname ?? user.account ?? '账号')}
          onMouseLeave={() => setTooltip(null)}
          onClick={() => navigate('/account')}
          type="button"
        >
          <span className={styles.icon}>
            {(user.nickname ?? user.account)?.[0]?.toUpperCase() ?? '?'}
          </span>
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
