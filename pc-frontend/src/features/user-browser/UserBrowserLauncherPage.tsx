import { useCallback, useEffect, useState } from 'react'
import {
  CircleCheck,
  ExternalLink,
  Fingerprint,
  LoaderCircle,
  RefreshCw,
  ShieldAlert,
} from 'lucide-react'
import { useAuthStore } from '../../store/auth'
import {
  discoverChatGptBrowser,
  launchChatGptBrowser,
} from './userBrowserLauncherApi'
import type { ConsumerDiscoveryMatch } from '../open-commerce/openCommerceClientTypes'
import LocalAiBrowserPanel from './LocalAiBrowserPanel'
import useLocalAiBrowserCapability, { type LocalAiBrowserCapabilityState } from './useLocalAiBrowserCapability'
import styles from './UserBrowserLauncherPage.module.css'

type Availability = 'checking' | 'ready' | 'unavailable' | 'error'

export default function UserBrowserLauncherPage() {
  const token = useAuthStore((state) => state.token)
  const user = useAuthStore((state) => state.user)
  const [availability, setAvailability] = useState<Availability>('checking')
  const [match, setMatch] = useState<ConsumerDiscoveryMatch | null>(null)
  const [ownerConfirmed, setOwnerConfirmed] = useState(false)
  const [launching, setLaunching] = useState(false)
  const [message, setMessage] = useState('')
  const localBrowser = useLocalAiBrowserCapability()
  const localBrowserAvailable = localBrowser.state === 'ready'

  const checkAvailability = useCallback(async () => {
    if (!token || !user) {
      setAvailability('unavailable')
      setMessage('登录一龙账号后才能启动本人会话。')
      return
    }
    setAvailability('checking')
    setMessage('')
    try {
      const discovered = await discoverChatGptBrowser()
      setMatch(discovered)
      setAvailability(discovered ? 'ready' : 'unavailable')
      setMessage(discovered ? '' : '模块服务尚未发布 ChatGPT 浏览器能力。')
    } catch (error) {
      setMatch(null)
      setAvailability('error')
      setMessage(errorMessage(error))
    }
  }, [token, user])

  useEffect(() => {
    void checkAvailability()
  }, [checkAvailability])

  async function launch() {
    if (!match || !ownerConfirmed || launching) return
    const pendingWindow = window.open('', '_blank')
    if (pendingWindow) {
      pendingWindow.opener = null
      pendingWindow.document.title = '正在启动本人浏览器会话'
    }
    setLaunching(true)
    setMessage('')
    try {
      const session = await launchChatGptBrowser(match)
      if (pendingWindow) pendingWindow.location.replace(session.launchUrl)
      else window.open(session.launchUrl, '_blank', 'noopener')
      setMessage('本人浏览器会话已启动。')
    } catch (error) {
      pendingWindow?.close()
      setMessage(errorMessage(error))
    } finally {
      setLaunching(false)
    }
  }

  const ready = availability === 'ready' && Boolean(match)

  return (
    <section className={styles.page}>
      <header className={styles.header}>
        <div className={styles.identityMark} aria-hidden="true">
          <Fingerprint size={30} strokeWidth={1.8} />
        </div>
        <div>
          <span className={styles.eyebrow}>官方 AI 网页</span>
          <h1>ChatGPT 与 Google AI 模式</h1>
          <p>本地隔离会话 · Cookie 和网页数据仅保存在这台电脑</p>
        </div>
        <Status availability={availability} localBrowserState={localBrowser.state} />
      </header>

      <div className={styles.rule} />

      <LocalAiBrowserPanel
        ownerKey={user?.id}
        ownerLabel={user?.nickname || user?.account || '未登录'}
        capability={localBrowser}
      />

      {!localBrowserAvailable && <main className={styles.workspace}>
        <div className={styles.sessionSummary}>
          <span>普通浏览器备用模式</span>
          <strong>{ready ? '可以启动' : '当前不可启动'}</strong>
          <small>会话所有者：{user?.id || '未识别'}</small>
          <small>模块服务：{match?.merchant.display_name || '未发现'}</small>
        </div>

        <div className={styles.actionPanel}>
          <label className={styles.ownershipCheck}>
            <input
              type="checkbox"
              checked={ownerConfirmed}
              onChange={(event) => setOwnerConfirmed(event.target.checked)}
              disabled={!ready || launching}
            />
            <span>
              <strong>本人账号确认</strong>
              <small>
                我只会登录由本人创建、持有并获准使用的 ChatGPT 账号，并确认由
                {match?.merchant.display_name || '可信模块服务'}启动独立会话。
              </small>
            </span>
          </label>

          <button
            className={styles.launchButton}
            type="button"
            onClick={() => void launch()}
            disabled={!ready || !ownerConfirmed || launching}
          >
            {launching ? <LoaderCircle className={styles.spin} size={18} /> : <ExternalLink size={18} />}
            {launching ? '正在创建会话' : '启动本人会话'}
          </button>
        </div>

        {message && (
          <div className={styles.message} data-error={availability === 'error' || message.includes('失败')}>
            {message}
          </div>
        )}

        {(availability === 'error' || availability === 'unavailable') && (
          <button className={styles.refreshButton} type="button" onClick={() => void checkAvailability()}>
            <RefreshCw size={15} />
            重新检查
          </button>
        )}
      </main>}
    </section>
  )
}

function Status({
  availability,
  localBrowserState,
}: {
  availability: Availability
  localBrowserState: LocalAiBrowserCapabilityState
}) {
  const content = localBrowserState === 'ready'
    ? { icon: <CircleCheck size={15} />, label: 'Win 本地可用' }
    : localBrowserState === 'checking'
      ? { icon: <LoaderCircle className={styles.spin} size={15} />, label: '检查 Win 能力' }
    : localBrowserState === 'upgrade_required'
      ? { icon: <ShieldAlert size={15} />, label: '需更新 Win 客户端' }
    : localBrowserState === 'error'
      ? { icon: <ShieldAlert size={15} />, label: '本地功能异常' }
    : availability === 'checking'
    ? { icon: <LoaderCircle className={styles.spin} size={15} />, label: '检查中' }
    : availability === 'ready'
      ? { icon: <CircleCheck size={15} />, label: '模块在线' }
      : { icon: <ShieldAlert size={15} />, label: '不可用' }
  return (
    <span
      className={styles.status}
      data-state={localBrowserState === 'ready'
        ? 'ready'
        : localBrowserState === 'upgrade_required' || localBrowserState === 'error'
          ? 'error'
          : availability}
    >
      {content.icon}{content.label}
    </span>
  )
}

function errorMessage(error: unknown): string {
  return error instanceof Error && error.message ? error.message : '浏览器模块调用失败。'
}
