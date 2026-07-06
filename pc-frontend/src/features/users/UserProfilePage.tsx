import { Link, useParams } from 'react-router-dom'
import { resolveApiUrl } from '../../api/runtime'
import { useAuthStore } from '../../store/auth'
import styles from './UserProfilePage.module.css'

function initialFrom(value: string): string {
  return value.trim().slice(0, 1).toUpperCase() || '龙'
}

export default function UserProfilePage() {
  const { userId = '' } = useParams()
  const user = useAuthStore((s) => s.user)
  const decodedUserId = decodeURIComponent(userId)
  const isSelf = !!decodedUserId && decodedUserId === user?.id
  const displayName = isSelf ? (user?.nickname || user?.account || decodedUserId) : decodedUserId
  const account = isSelf ? user?.account : undefined
  const avatarSrc = isSelf && user?.avatar_data_url
    ? user.avatar_data_url
    : decodedUserId
      ? resolveApiUrl('/api/users/' + encodeURIComponent(decodedUserId) + '/avatar')
      : ''

  return (
    <main className={styles.page}>
      <section className={styles.card}>
        <div className={styles.banner} />
        <div className={styles.body}>
          <div className={styles.avatar}>
            {avatarSrc ? <img src={avatarSrc} alt="" onError={(event) => { event.currentTarget.style.display = 'none' }} /> : null}
            <span>{initialFrom(displayName)}</span>
          </div>
          <div className={styles.identity}>
            <span className={styles.kicker}>用户主页</span>
            <h1>{displayName}</h1>
            {account && <p>{account}</p>}
            <code>{decodedUserId || 'unknown-user'}</code>
          </div>
          <div className={styles.actions}>
            {isSelf ? <Link to="/account">编辑我的资料</Link> : <Link to="/friends">打开好友列表</Link>}
          </div>
        </div>
        <div className={styles.note}>
          当前先接入头像跳转和基础身份卡；完整 Discord 式个人主页需要后端公开资料接口、好友关系和私聊入口一起补齐。
        </div>
      </section>
    </main>
  )
}
