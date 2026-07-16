import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import type { CSSProperties, RefObject } from 'react'
import { useNavigate } from 'react-router-dom'
import { Activity, ExternalLink, FolderPlus, LogOut, Settings, Smartphone } from 'lucide-react'
import type { User } from '../../store/auth'
import { getPcLegacyUrl, rememberPcLegacyToken } from './pcLegacyUrl'
import UserAvatar, { userAccountMeta, userDisplayName } from './UserAvatar'
import type { UserPresenceSettings } from './useMyPresence'
import { presenceSummary } from './useMyPresence'
import styles from './UserAccountMenu.module.css'

const MENU_GAP = 8
const VIEWPORT_MARGIN = 12

interface Props {
  id: string
  user: User
  token: string | null
  presence: UserPresenceSettings | null
  open: boolean
  anchorRef: RefObject<HTMLElement>
  onClose: () => void
  onLogout: () => void
  onOpenPresence: () => void
}

interface MenuPosition {
  left: number
  width: number
  bottom: number
  maxHeight: number
}

export default function UserAccountMenu({
  id,
  user,
  token,
  presence,
  open,
  anchorRef,
  onClose,
  onLogout,
  onOpenPresence,
}: Props) {
  const navigate = useNavigate()
  const menuRef = useRef<HTMLDivElement>(null)
  const [menuPosition, setMenuPosition] = useState<MenuPosition | null>(null)
  const legacyUrl = getPcLegacyUrl()
  const displayName = userDisplayName(user)

  useLayoutEffect(() => {
    if (!open) {
      setMenuPosition(null)
      return
    }

    function updatePosition() {
      const anchor = anchorRef.current
      if (!anchor) return

      const rect = anchor.getBoundingClientRect()
      const availableWidth = Math.max(180, window.innerWidth - VIEWPORT_MARGIN * 2)
      const width = Math.min(rect.width, availableWidth)
      const maxLeft = Math.max(VIEWPORT_MARGIN, window.innerWidth - width - VIEWPORT_MARGIN)

      setMenuPosition({
        left: Math.min(Math.max(rect.left, VIEWPORT_MARGIN), maxLeft),
        width,
        bottom: Math.max(VIEWPORT_MARGIN, window.innerHeight - rect.top + MENU_GAP),
        maxHeight: Math.max(180, rect.top - MENU_GAP - VIEWPORT_MARGIN),
      })
    }

    updatePosition()
    window.addEventListener('resize', updatePosition)
    window.addEventListener('scroll', updatePosition, true)
    return () => {
      window.removeEventListener('resize', updatePosition)
      window.removeEventListener('scroll', updatePosition, true)
    }
  }, [anchorRef, open])

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

  function editPresence() {
    onClose()
    onOpenPresence()
  }

  if (!open || !menuPosition) return null

  const menuStyle: CSSProperties = {
    left: menuPosition.left,
    width: menuPosition.width,
    bottom: menuPosition.bottom,
    maxHeight: menuPosition.maxHeight,
  }

  return createPortal(
    <div
      id={id}
      ref={menuRef}
      className={styles.menu}
      style={menuStyle}
      role="menu"
      aria-label="账号中心"
    >
      <div className={styles.header}>
        <UserAvatar
          user={user}
          size="panel"
          showStatus
          presenceStatus={presence?.status}
          className={styles.headerAvatar}
        />
        <div className={styles.headerCopy}>
          <strong>{displayName}</strong>
          <span>{presence ? presenceSummary(presence) : userAccountMeta(user)}</span>
        </div>
      </div>

      <button
        className={styles.row}
        type="button"
        role="menuitem"
        onClick={editPresence}
      >
        <Activity size={17} aria-hidden="true" />
        <span>
          <strong>我的状态</strong>
          <small>{presence ? presenceSummary(presence) : '设置在线、离开、勿扰或隐身'}</small>
        </span>
      </button>

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
          <strong>电脑与算力</strong>
          <small>节点连接、代码目录和算力共享</small>
        </span>
      </button>

      <a className={styles.row} role="menuitem" href="/app/download" target="_blank" rel="noreferrer">
        <Smartphone size={17} aria-hidden="true" />
        <span>
          <strong>移动端</strong>
          <small>打开手机端下载与连接入口</small>
        </span>
      </a>

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
    </div>,
    document.body,
  )
}
