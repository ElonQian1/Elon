import { useCallback, useEffect, useState } from 'react'
import { CircleCheck, Cloud, KeyRound, Link2, MonitorUp, ShieldAlert } from 'lucide-react'
import { Link } from 'react-router-dom'
import GoogleIdentityButton from '../auth/GoogleIdentityButton'
import {
  federatedIdentityApi,
  type FederatedProvider,
  type LinkedIdentity,
} from '../auth/federatedIdentityApi'
import type { LocalAiOwnerIdentity } from './useLocalAiOwnerIdentity'
import styles from './LocalAiAccountSessionCard.module.css'

interface LocalAiAccountSessionCardProps {
  identity: LocalAiOwnerIdentity
  cloudAccountAvailable: boolean
}

export default function LocalAiAccountSessionCard({
  identity,
  cloudAccountAvailable,
}: LocalAiAccountSessionCardProps) {
  const [googleProvider, setGoogleProvider] = useState<FederatedProvider | null>(null)
  const [googleIdentity, setGoogleIdentity] = useState<LinkedIdentity | null>(null)
  const [loading, setLoading] = useState(cloudAccountAvailable)
  const [message, setMessage] = useState('')

  const load = useCallback(async () => {
    if (!cloudAccountAvailable) {
      setGoogleProvider(null)
      setGoogleIdentity(null)
      setLoading(false)
      return
    }
    setLoading(true)
    setMessage('')
    try {
      const [providerResult, identityResult] = await Promise.all([
        federatedIdentityApi.providers(),
        federatedIdentityApi.identities(),
      ])
      setGoogleProvider(providerResult.providers.find((item) => item.id === 'google') || null)
      setGoogleIdentity(identityResult.identities.find((item) => item.provider === 'google') || null)
    } catch (error) {
      setMessage(error instanceof Error ? error.message : '暂时无法读取账号绑定状态。')
    } finally {
      setLoading(false)
    }
  }, [cloudAccountAvailable])

  useEffect(() => { void load() }, [load])

  const identityOk = Boolean(identity.ownerKey)
  return (
    <section className={styles.card} aria-labelledby="local-ai-account-session-title">
      <header className={styles.header}>
        <div>
          <span>账号与本机会话中心</span>
          <h2 id="local-ai-account-session-title">先认清三个不同的登录状态</h2>
        </div>
        <Link to="/account"><KeyRound size={15} />管理账号与安全</Link>
      </header>

      <div className={styles.layers}>
        <Layer
          icon={<Cloud size={19} />}
          title="一龙账号"
          status={identityOk ? identity.ownerLabel : identity.checking ? '检查中' : '未登录'}
          tone={identity.source === 'conflict' ? 'danger' : identityOk ? 'ready' : 'muted'}
          detail={identity.detail}
        />
        <Layer
          icon={<Link2 size={19} />}
          title="Google · 一龙登录方式"
          status={loading
            ? '读取中'
            : googleIdentity
              ? `已绑定 ${googleIdentity.email || 'Google 账号'}`
              : googleProvider?.configured
                ? '可以绑定'
                : cloudAccountAvailable ? '管理员尚未配置' : '回到云端账号后查看'}
          tone={googleIdentity ? 'ready' : 'muted'}
          detail="这里只决定能否用 Google 进入同一个一龙账号，不会把 Google 登录态复制给 Google AI 或 ChatGPT 网页。"
        />
        <Layer
          icon={<MonitorUp size={19} />}
          title="ChatGPT / Google AI 官方网页"
          status="每个厂商独立登录"
          tone="local"
          detail="官方 Cookie 留在各自的本机 WebView2 Profile；登录、真人验证和地区开放状态都由厂商官方页面决定。"
        />
      </div>

      {cloudAccountAvailable && !loading && !googleIdentity && googleProvider?.configured && (
        <div className={styles.bind}>
          <div><strong>把 Google 设为一龙登录方式</strong><small>绑定完成后仍需在官方 AI 网页按厂商要求登录。</small></div>
          <GoogleIdentityButton mode="bind" onComplete={load} />
        </div>
      )}
      {identity.source === 'conflict' && (
        <p className={styles.warning}><ShieldAlert size={15} />请先在“本机节点”页重新绑定当前账号，不会自动清除任何厂商网页数据。</p>
      )}
      {message && <p className={styles.warning}><ShieldAlert size={15} />{message}</p>}
    </section>
  )
}

function Layer({
  icon,
  title,
  status,
  tone,
  detail,
}: {
  icon: React.ReactNode
  title: string
  status: string
  tone: 'ready' | 'local' | 'muted' | 'danger'
  detail: string
}) {
  return (
    <article className={styles.layer} data-tone={tone}>
      <span className={styles.icon}>{icon}</span>
      <div><strong>{title}</strong><em>{status}</em><p>{detail}</p></div>
      {tone === 'ready' && <CircleCheck className={styles.check} size={17} />}
    </article>
  )
}
