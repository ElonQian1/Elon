import { useEffect, useState } from 'react'
import { ExternalLink, EyeOff, LoaderCircle, MonitorUp, RefreshCw } from 'lucide-react'
import { useSearchParams } from 'react-router-dom'
import NativeAiWebChat from './NativeAiWebChat'
import { listLocalAiWebProviders, localAiBrowserErrorMessage, type LocalAiWebProvider, type LocalAiWebSessionState } from './localAiBrowserApi'
import { LOCAL_AI_PROVIDER_FALLBACKS } from './localAiWebProviders'
import useLocalAiWebChatController from './useLocalAiWebChatController'
import useLocalAiOwnerIdentity from './useLocalAiOwnerIdentity'
import styles from './NativeAiWebChatWindow.module.css'

export default function NativeAiWebChatWindow() {
  const [searchParams] = useSearchParams()
  const requestedProviderId = searchParams.get('provider') || ''
  const identity = useLocalAiOwnerIdentity()
  const ownerKey = identity.ownerKey
  const [providers, setProviders] = useState<LocalAiWebProvider[]>([])
  const [loading, setLoading] = useState(true)
  const [loadError, setLoadError] = useState('')
  const provider = providers.find((item) => item.id === requestedProviderId)
    || LOCAL_AI_PROVIDER_FALLBACKS[requestedProviderId]
  const controller = useLocalAiWebChatController(provider, ownerKey)

  useEffect(() => {
    let active = true
    void listLocalAiWebProviders()
      .then((items) => { if (active) setProviders(items) })
      .catch((error) => { if (active) setLoadError(localAiBrowserErrorMessage(error)) })
      .finally(() => { if (active) setLoading(false) })
    return () => { active = false }
  }, [])

  if (loading) return <WindowNotice icon="loading" title="正在打开一龙聊天窗" detail="正在读取本机官方 AI 能力…" />
  if (identity.checking) return <WindowNotice icon="loading" title="正在恢复本机会话" detail={identity.detail} />
  if (!ownerKey) return <WindowNotice title="无法识别一龙账号" detail={identity.detail} />
  if (!provider) return <WindowNotice title="厂商窗口无效" detail="请从一龙主窗口的“官方 AI”入口重新打开。" />

  return (
    <main className={styles.window}>
      <header className={styles.toolbar}>
        <div className={styles.identity}>
          <span>YILONG NATIVE AI</span>
          <strong>{provider.displayName}</strong>
          <small>{statusLabel(controller.sessionState)} · Cookie 仅在官方 WebView2 内</small>
        </div>
        <div className={styles.actions}>
          <button type="button" onClick={() => void controller.openOfficial()} disabled={Boolean(controller.busyAction)}>
            {controller.busyAction === 'open' ? <LoaderCircle className={styles.spin} size={15} /> : <MonitorUp size={15} />}
            {controller.sessionOpen ? '显示官方窗口' : '打开官方窗口'}
          </button>
          <button
            type="button"
            onClick={() => void controller.control('background')}
            disabled={!controller.sessionOpen || !controller.sessionState?.windowVisible || Boolean(controller.busyAction)}
          >
            <EyeOff size={15} />收起官方页
          </button>
          <button type="button" onClick={() => void controller.control('reload')} disabled={!controller.sessionOpen || Boolean(controller.busyAction)}>
            <RefreshCw size={15} />刷新官方页
          </button>
          <button type="button" onClick={() => void controller.control('external')} disabled={Boolean(controller.busyAction)}>
            <ExternalLink size={15} />系统浏览器
          </button>
        </div>
      </header>

      {(loadError || controller.message) && <p className={styles.message}>{loadError || controller.message}</p>}
      {controller.sessionState?.lastError && <p className={styles.error}>{controller.sessionState.lastError}</p>}

      <div className={styles.chatFrame}>
        <NativeAiWebChat
          provider={provider}
          snapshot={controller.snapshot}
          sessionOpen={controller.sessionOpen}
          busy={Boolean(controller.busyAction)}
          draft={controller.draft}
          onDraftChange={controller.setDraft}
          onRun={(action, value, expectedDraft) => void controller.run(action, value, expectedDraft)}
          standalone
        />
      </div>
    </main>
  )
}

function WindowNotice({
  icon,
  title,
  detail,
}: {
  icon?: 'loading'
  title: string
  detail: string
}) {
  return (
    <main className={styles.notice}>
      {icon === 'loading' ? <LoaderCircle className={styles.spin} size={24} /> : <MonitorUp size={24} />}
      <strong>{title}</strong>
      <p>{detail}</p>
    </main>
  )
}

function statusLabel(state: LocalAiWebSessionState | null): string {
  if (!state || state.windowStatus === 'closed') return '官方窗口未打开'
  if (state.windowVisible) return '官方页可见'
  if (state.rendererStatus === 'active') return '官方页在后台 · 一龙界面已连接'
  if (state.loading) return '正在同步官方页面'
  return '等待官方页面'
}
