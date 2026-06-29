import { NavLink, useNavigate } from 'react-router-dom'
import { LogOut, Settings } from 'lucide-react'
import { useAuthStore } from '../../store/auth'
import { ModelPickerButton } from '../models/ModelPicker'
import { getPcLegacyUrl, rememberPcLegacyToken } from './pcLegacyUrl'
import UserAvatar, { userDisplayName } from './UserAvatar'
import styles from './Sidebar.module.css'

interface NavItem {
  to: string
  label: string
  icon: string
}

const NAV_ITEMS: NavItem[] = [
  { to: '/', icon: '💬', label: '项目对话' },
  { to: '/voice', icon: '🎙️', label: '声音' },
  { to: '/doctor', icon: '🩺', label: '电脑医生' },
  { to: '/node', icon: '🖥️', label: '本机节点' },
]

export default function Sidebar() {
  const navigate = useNavigate()
  const user = useAuthStore((s) => s.user)
  const token = useAuthStore((s) => s.token)
  const logout = useAuthStore((s) => s.logout)
  const legacyUrl = getPcLegacyUrl()
  const displayName = userDisplayName(user)

  return (
    <nav className={styles.sidebar}>
      <div className={styles.brand}>
        <span className={styles.brandName}>一龙</span>
        <span className={styles.brandBadge}>Beta</span>
      </div>

      <ul className={styles.navList}>
        {NAV_ITEMS.map((item) => (
          <li key={item.to}>
            <NavLink
              to={item.to}
              end={item.to === '/'}
              className={({ isActive }) =>
                [styles.navItem, isActive ? styles.active : ''].join(' ')
              }
            >
              <span className={styles.icon}>{item.icon}</span>
              <span className={styles.label}>{item.label}</span>
            </NavLink>
          </li>
        ))}
      </ul>

      <div className={styles.footer}>
        <ModelPickerButton />
        {user && (
          <div className={styles.userCard}>
            <button
              className={styles.userProfile}
              type="button"
              onClick={() => navigate('/account')}
              title="账号中心"
            >
              <UserAvatar user={user} size="compact" showStatus />
              <span className={styles.userCopy}>
                <strong>{displayName}</strong>
                <small>{user.account}</small>
              </span>
            </button>
            <div className={styles.userActions}>
              <button
                className={styles.userActionBtn}
                type="button"
                onClick={() => navigate('/account')}
                title="账号设置"
                aria-label="账号设置"
              >
                <Settings size={16} aria-hidden="true" />
              </button>
              <button
                className={styles.userActionBtn}
                type="button"
                onClick={logout}
                title="退出登录"
                aria-label="退出登录"
              >
                <LogOut size={16} aria-hidden="true" />
              </button>
            </div>
          </div>
        )}
        <a
          className={styles.legacyLink}
          href={legacyUrl}
          onClick={() => rememberPcLegacyToken(token)}
        >
          切换旧版
        </a>
      </div>
    </nav>
  )
}
