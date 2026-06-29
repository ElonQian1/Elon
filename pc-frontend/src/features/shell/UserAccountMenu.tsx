import { useEffect, useRef } from 'react'
import type { RefObject } from 'react'
import { useNavigate } from 'react-router-dom'
import { ExternalLink, FolderPlus, LogOut, Settings } from 'lucide-react'
import type { User } from '../../store/auth'
import { getPcLegacyUrl, rememberPcLegacyToken } from './pcLegacyUrl'
import UserAvatar, { userAccountMeta, userDisplayName } from './UserAvatar'
import styles from './UserAccountMenu.module.css'

interface Props {
  id: string
  user: User
  token: string | null
  open: boolean
  anchorRef: RefObject<HTMLElement>
  onClose: () => void
  onLogout: () => void
}

export default function UserAccountMenu({
  id,
  user,
  token,
  open,
  anchorRef,
  onClose,
  onLogout,
}: Props) {
  const navigate = useNavigate()
  const menuRef = useRef<HTMLDivElement>(null)
  const legacyUrl = getPcLegacyUrl()
  const displayName = userDisplayName(user)

  useEffect(() => {
    if (!open) return

    function handlePointerDown(event: PointerEvent) {
      const target = event.target
      if (!(target instanceof Node)) return
      if (menuRef.current?.contains(target)) return
      if (anchorRef.current?.contains(target)) return
      onClose()
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') onClose()
    }

    document.addEventListener('pointerdown', handlePointerDown)
    document.addEventListener('keydown', handleKeyDown)
    return () => {
      document.removeEventListener('pointerdown', handlePointerDown)
      document.removeEventListener('keydown', handleKeyDown)
    }
  }, [anchorRef, onClose, open])

  function go(to: string) {
    onClose()
    navigate(to)
  }

  function handleLegacyClick() {
    rememberPcLegacyToken(token)
    onClose()
  }

  function handleLogout() {
    onClose()
    onLogout()
  }

  return (
    <div
      id={id}
      ref={menuRef}
      className={styles.menu}
      role="menu"
      aria-label="账号中心"
      hidden={!open}
    >
      <div className={styles.header}>
        <UserAvatar user={user} size="panel" showStatus className={styles.headerAvatar} />
        <div className={styles.headerCopy}>
          <strong>{displayName}</strong>
          <span>{userAccountMeta(user)}</span>
        </div>
      </div>

      <button
        className={styles.row}
        type="button"
        role="menuitem"
        onClick={() => go('/account')}
      >
        <Settings size={17} aria-hidden="true" />
        <span>
          <strong>账户设置</strong>
          <small>账号信息和登录状态</small>
        </span>
      </button>

      <button
        className={styles.row}
        type="button"
        role="menuitem"
        onClick={() => go('/node')}
      >
        <FolderPlus size={17} aria-hidden="true" />
        <span>
          <strong>导入电脑代码</strong>
          <small>把已有代码文件夹加入工作台</small>
        </span>
      </button>

      <a
        className={styles.row}
        role="menuitem"
        href={legacyUrl}
        onClick={handleLegacyClick}
      >
        <ExternalLink size={17} aria-hidden="true" />
        <span>
          <strong>切换旧版</strong>
          <small>打开旧版 PC 工作台对照</small>
        </span>
      </a>

      <button
        className={[styles.row, styles.danger].join(' ')}
        type="button"
        role="menuitem"
        onClick={handleLogout}
      >
        <LogOut size={17} aria-hidden="true" />
        <span>
          <strong>退出登录</strong>
          <small>清除本机网页版登录态</small>
        </span>
      </button>
    </div>
  )
}
