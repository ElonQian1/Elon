import { useEffect, useState } from 'react'
import type { User } from '../../store/auth'
import styles from './UserAvatar.module.css'

type AvatarSize = 'rail' | 'compact' | 'panel'

interface Props {
  user: User | null
  size?: AvatarSize
  showStatus?: boolean
  className?: string
}

export function userDisplayName(user: User | null): string {
  return user?.nickname || user?.account || '未登录'
}

export function userInitial(user: User | null): string {
  return userDisplayName(user).trim().slice(0, 1).toUpperCase() || '龙'
}

export default function UserAvatar({ user, size = 'compact', showStatus = false, className = '' }: Props) {
  const avatarSrc = user?.avatar_data_url?.trim() || ''
  const [imageFailed, setImageFailed] = useState(false)

  useEffect(() => {
    setImageFailed(false)
  }, [avatarSrc])

  const showImage = !!avatarSrc && !imageFailed
  const classNames = [
    styles.avatar,
    styles[size],
    showImage ? styles.hasImage : '',
    className,
  ].filter(Boolean).join(' ')

  return (
    <span className={classNames} aria-hidden="true">
      {showImage && (
        <img
          src={avatarSrc}
          alt=""
          className={styles.image}
          onError={() => setImageFailed(true)}
        />
      )}
      <span className={styles.initial}>{userInitial(user)}</span>
      {showStatus && <span className={styles.statusDot} />}
    </span>
  )
}
