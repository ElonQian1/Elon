import { useEffect, useState } from 'react'
import type { User } from '../../store/auth'
import { visiblePresenceStatus } from './useMyPresence'
import styles from './UserAvatar.module.css'

type AvatarSize = 'rail' | 'compact' | 'panel'

interface Props {
  user: User | null
  size?: AvatarSize
  showStatus?: boolean
  presenceStatus?: string | null
  className?: string
}

export function userDisplayName(user: User | null): string {
  return user?.nickname || user?.account || userField(user, 'phone') || userField(user, 'email') || '未登录'
}

export function userInitial(user: User | null): string {
  return userDisplayName(user).trim().slice(0, 1).toUpperCase() || '龙'
}

export function userAvatarUrl(user: User | null): string {
  const fields = [
    'avatar_data_url',
    'avatarDataUrl',
    'sender_avatar_data_url',
    'senderAvatarDataUrl',
    'sender_avatar_url',
    'senderAvatarUrl',
    'user_avatar',
    'userAvatar',
    'user_avatar_url',
    'userAvatarUrl',
    'member_avatar_url',
    'memberAvatarUrl',
    'profile_avatar_url',
    'profileAvatarUrl',
    'avatar_url',
    'avatarUrl',
    'icon_data_url',
    'iconDataUrl',
    'logo_url',
    'logoUrl',
    'photo_url',
    'photoUrl',
    'head_img_url',
    'headImgUrl',
    'portrait_url',
    'portraitUrl',
    'image_url',
    'imageUrl',
    'avatar',
  ]
  for (const field of fields) {
    const value = userField(user, field)
    if (value) return value
  }
  return ''
}

export function userAccountMeta(user: User | null): string {
  const name = userDisplayName(user)
  const account = user?.account || userField(user, 'phone') || userField(user, 'email')
  if (account && account !== name) return `账号：${account}`
  if (user?.id) return `用户 ID：${shortUserId(user.id)}`
  return user ? '在线' : '需要登录'
}

function userField(user: User | null, field: string): string {
  if (!user) return ''
  const value = (user as unknown as Record<string, unknown>)[field]
  return typeof value === 'string' ? value.trim() : ''
}

function shortUserId(id: string): string {
  const value = id.trim()
  if (value.length <= 12) return value
  return `${value.slice(0, 8)}…${value.slice(-4)}`
}

export default function UserAvatar({ user, size = 'compact', showStatus = false, presenceStatus, className = '' }: Props) {
  const avatarSrc = userAvatarUrl(user)
  const [imageFailed, setImageFailed] = useState(false)
  const status = visiblePresenceStatus(presenceStatus ?? user?.status ?? 'online')

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
      {showStatus && <span className={styles.statusDot} data-status={status} />}
    </span>
  )
}
