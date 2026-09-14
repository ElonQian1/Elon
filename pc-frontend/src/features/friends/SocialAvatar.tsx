import { useState } from 'react'
import { cloudResourceUrl } from '../../lib/cloudResourceUrl'
import styles from './SocialMessageAttachments.module.css'

export default function SocialAvatar({ userId, name, avatar }: {
  userId?: string
  name: string
  avatar?: string | null
}) {
  const src = avatar?.startsWith('data:image/') ? avatar
    : cloudResourceUrl(avatar) || cloudResourceUrl(userId ? `/api/users/${encodeURIComponent(userId)}/avatar` : '')
  return <AvatarImage key={src} src={src} name={name} />
}

function AvatarImage({ src, name }: { src: string; name: string }) {
  const [failed, setFailed] = useState(false)
  return src && !failed
    ? <img className={styles.avatarImage} src={src} alt={`${name}的头像`} onError={() => setFailed(true)} />
    : <span aria-label={`${name}的默认头像`}>{Array.from(name.trim())[0] || '友'}</span>
}
