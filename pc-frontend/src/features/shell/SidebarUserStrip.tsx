import { useCallback, useId, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Settings } from 'lucide-react'
import { useAuthStore } from '../../store/auth'
import LevelExperienceBar from '../billing/LevelExperienceBar'
import { useUserProgression } from '../billing/useUserProgression'
import { useUserUsage } from '../billing/useUserUsage'
import UserAvatar, { userDisplayName } from './UserAvatar'
import UserAccountMenu from './UserAccountMenu'
import { presenceSummary, useMyPresence } from './useMyPresence'
import { PresenceDrawer } from '../conversation/PresenceDrawer'
import styles from './SidebarUserStrip.module.css'

export default function SidebarUserStrip() {
  const navigate = useNavigate()
  const accountMenuId = useId()
  const stripRef = useRef<HTMLDivElement>(null)
  const [menuOpen, setMenuOpen] = useState(false)
  const [presenceOpen, setPresenceOpen] = useState(false)
  const user = useAuthStore((s) => s.user)
  const token = useAuthStore((s) => s.token)
  const logout = useAuthStore((s) => s.logout)
  const progression = useUserProgression(user?.id, token)
  const { usage, loading: usageLoading } = useUserUsage(user?.id, token)
  const presence = useMyPresence()
  const displayName = user ? userDisplayName(user) : token ? '加载中…' : '未登录'
  const statusText = user ? presenceSummary(presence) : '需要登录'

  const closeMenu = useCallback(() => setMenuOpen(false), [])

  const openAccountSettings = useCallback(() => {
    closeMenu()
    navigate('/account')
  }, [closeMenu, navigate])

  const handleLogout = useCallback(() => {
    closeMenu()
    logout()
  }, [closeMenu, logout])

  return (
    <div
      ref={stripRef}
      className={[styles.strip, menuOpen ? styles.stripOpen : ''].filter(Boolean).join(' ')}
    >
      <div className={styles.progressSlot}>
          <LevelExperienceBar progression={progression} usage={usage} usageLoading={usageLoading} />
      </div>

      <button
        className={styles.profileBtn}
        type="button"
        title={user ? '账号菜单' : '账号加载中'}
        onClick={() => user && setMenuOpen((open) => !open)}
        disabled={!user}
        aria-haspopup={user ? 'menu' : undefined}
        aria-expanded={user ? menuOpen : undefined}
        aria-controls={user ? accountMenuId : undefined}
      >
        <UserAvatar user={user} size="compact" showStatus={!!user} presenceStatus={presence?.status} />
        <span className={styles.userCopy}>
          <strong>{displayName}</strong>
          <small>{statusText}</small>
        </span>
      </button>

      <div className={styles.actions}>
        <button
          className={styles.actionBtn}
          type="button"
          title="账号设置"
          aria-label="账号设置"
          onClick={openAccountSettings}
          disabled={!user}
        >
          <Settings size={16} aria-hidden="true" />
        </button>
      </div>

      {user && (
        <UserAccountMenu
          id={accountMenuId}
          user={user}
          token={token}
          presence={presence}
          open={menuOpen}
          anchorRef={stripRef}
          onClose={closeMenu}
          onLogout={handleLogout}
          onOpenPresence={() => setPresenceOpen(true)}
        />
      )}
      {presenceOpen && (
        <PresenceDrawer onClose={() => setPresenceOpen(false)} onSaved={() => {}} />
      )}
    </div>
  )
}
