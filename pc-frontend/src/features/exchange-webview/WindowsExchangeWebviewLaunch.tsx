import { useRef, useState } from 'react'
import { ExternalLink, Loader2, ShieldCheck } from 'lucide-react'
import useLocalAiOwnerIdentity from '../user-browser/useLocalAiOwnerIdentity'
import {
  isExchangeWebviewAvailable,
  listExchangeWebProviders,
  openExchangeWebSession,
} from './exchangeWebviewApi'
import { exchangeWebviewErrorMessage } from './exchangeWebviewErrors.js'
import styles from './WindowsExchangeWebviewLaunch.module.css'

export interface WindowsWebviewLaunchContract {
  schema: 'yilong.windows_webview_launch.v1'
  provider_id: string
  label?: string
  description?: string
}

export default function WindowsExchangeWebviewLaunch({
  launch,
}: {
  launch: WindowsWebviewLaunchContract
}) {
  const identity = useLocalAiOwnerIdentity()
  const desktop = isExchangeWebviewAvailable()
  const openingRef = useRef(false)
  const [opening, setOpening] = useState(false)
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')

  async function open() {
    if (openingRef.current) return
    setMessage('')
    setError('')
    if (!desktop) return setError('请在一龙 Windows 客户端中打开此项目介绍。')
    if (identity.checking) return setError('正在确认本机会话身份，请稍后再试。')
    if (!identity.ownerKey) return setError(identity.detail || '当前本机会话身份不可用。')
    openingRef.current = true
    setOpening(true)
    try {
      const providers = await listExchangeWebProviders()
      const provider = providers.find((item) => item.providerId === launch.provider_id)
      if (!provider || provider.providerId !== launch.provider_id) {
        throw new Error('当前 Win 客户端尚未安装这个交易所官网入口，请先更新客户端。')
      }
      await openExchangeWebSession(provider.providerId, identity.ownerKey)
      setMessage(`${provider.displayName} 已在独立窗口打开。`)
    } catch (caught) {
      setError(exchangeWebviewErrorMessage(caught))
    } finally {
      openingRef.current = false
      setOpening(false)
    }
  }

  const disabled = opening || identity.checking || !identity.ownerKey
  return (
    <section className={styles.card} aria-label="Windows 交易所官网入口">
      <div className={styles.icon}><ShieldCheck size={20} aria-hidden="true" /></div>
      <div className={styles.content}>
        <strong>Win 官网操作入口</strong>
        <span>{launch.description || '在一龙 Win 客户端的独立官网窗口中操作。'}</span>
        <small>登录、验证和交易确认只在 Binance 官网完成；一龙不会读取或保存你的密码。</small>
        {message && <p className={styles.success} role="status">{message}</p>}
        {error && <p className={styles.error} role="alert">{error}</p>}
      </div>
      <button type="button" disabled={disabled} onClick={() => void open()}>
        {opening ? <Loader2 className={styles.spinner} size={15} aria-hidden="true" /> : <ExternalLink size={15} aria-hidden="true" />}
        {opening ? '正在打开…' : launch.label || '打开官网 WebView'}
      </button>
    </section>
  )
}
