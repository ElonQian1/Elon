import { useNavigate, useLocation } from 'react-router-dom'
import { useAuthStore } from '../../store/auth'
import { ModelPickerButton } from '../models/ModelPicker'
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
  { path: '/doctor',  emoji: '🩺', label: '电脑医生',  color: '#283342', hoverColor: '#315d72' },
  { path: '/node',    emoji: '🖥️', label: '分享算力',  color: '#2c2e35', hoverColor: '#365b44' },
  { path: '/voice',   emoji: '🎙️', label: 'AI 声音',  color: '#2f2a3a', hoverColor: '#7a4f9a' },
]

export default function ServerRail() {
  const navigate = useNavigate()
  const { pathname } = useLocation()
  const logout = useAuthStore((s) => s.logout)
  const user = useAuthStore((s) => s.user)

  function isActive(path: string) {
    if (path === '/') return pathname === '/'
    return pathname.startsWith(path)
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
            title={item.label}
            type="button"
          >
            <span className={styles.icon}>{item.emoji}</span>
          </button>
        )
      })}

      <div className={styles.divider} />

      {/* 模型选择器（紧凑版） */}
      <div className={styles.modelWrap}>
        <ModelPickerButton compact />
      </div>

      <div className={styles.spacer} />

      {/* 用户头像 + 登出 */}
      {user && (
        <button
          className={[styles.avatar, styles.userAvatar].join(' ')}
          title={`${user.nickname ?? user.account} — 点击退出`}
          onClick={logout}
          type="button"
        >
          <span className={styles.icon}>
            {(user.nickname ?? user.account)?.[0]?.toUpperCase() ?? '?'}
          </span>
        </button>
      )}
    </nav>
  )
}
