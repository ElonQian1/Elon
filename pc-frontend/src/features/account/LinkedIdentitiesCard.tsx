import { useCallback, useEffect, useState } from 'react'
import type { ApiError } from '../../api/client'
import GoogleIdentityButton from '../auth/GoogleIdentityButton'
import {
  federatedIdentityApi,
  type LinkedIdentity,
} from '../auth/federatedIdentityApi'
import styles from './LinkedIdentitiesCard.module.css'

export default function LinkedIdentitiesCard() {
  const [identities, setIdentities] = useState<LinkedIdentity[]>([])
  const [loading, setLoading] = useState(true)
  const [message, setMessage] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const result = await federatedIdentityApi.identities()
      setIdentities(result.identities)
    } catch (reason) {
      setMessage((reason as ApiError).message ?? '无法读取已绑定身份')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { void load() }, [load])

  async function unlink(identity: LinkedIdentity) {
    if (!window.confirm(`确定解绑 ${identity.email ?? 'Google 账号'}？`)) return
    setMessage(null)
    try {
      await federatedIdentityApi.unlink(identity.id)
      setMessage('已解绑 Google 账号')
      await load()
    } catch (reason) {
      setMessage((reason as ApiError).message ?? '解绑失败')
    }
  }

  return (
    <div className={styles.card}>
      <div className={styles.heading}>
        <div>
          <strong>登录方式</strong>
          <p>多个登录方式可以进入同一个一龙账号；不会复制厂商凭据。</p>
        </div>
      </div>
      {loading ? <p className={styles.muted}>读取中…</p> : identities.map((identity) => (
        <div className={styles.identity} key={identity.id}>
          {identity.avatar_url
            ? <img src={identity.avatar_url} alt="" referrerPolicy="no-referrer" />
            : <span className={styles.providerIcon}>G</span>}
          <div className={styles.identityText}>
            <strong>{identity.display_name || 'Google'}</strong>
            <span>{identity.email || '已绑定 Google 身份'}</span>
          </div>
          <button type="button" onClick={() => void unlink(identity)}>解绑</button>
        </div>
      ))}
      {!loading && !identities.some((identity) => identity.provider === 'google') && (
        <GoogleIdentityButton
          mode="bind"
          onComplete={async () => {
            setMessage('Google 账号已绑定')
            await load()
          }}
        />
      )}
      {message && <p className={styles.message}>{message}</p>}
      <p className={styles.notice}>为安全起见，同邮箱账号不会自动合并；请先登录原账号后主动绑定。</p>
    </div>
  )
}
