import { useEffect, useMemo, useState } from 'react'
import { ExternalLink, LoaderCircle, MonitorUp, RefreshCw } from 'lucide-react'
import { useSearchParams } from 'react-router-dom'
import { useAuthStore } from '../../store/auth'
import NativeAiWebChat from './NativeAiWebChat'
import {
  controlLocalAiWebSession,
  getLocalAiWebSessionState,
  isLocalAiMessageSnapshot,
  listLocalAiWebProviders,
  localAiBrowserErrorMessage,
  openLocalAiWebSession,
  runLocalAiWebAdapterCommand,
  waitForLocalAiAdapterResult,
  type LocalAiAdapterAction,
  type LocalAiBrowserControlAction,
  type LocalAiWebProvider,
  type LocalAiWebSessionState,
} from './localAiBrowserApi'
import styles from './NativeAiWebChatWindow.module.css'

const PROVIDER_FALLBACKS: Record<string, LocalAiWebProvider> = {
  'google-ai-mode': {
    id: 'google-ai-mode',
    displayName: 'Google AI 模式',
    startHost: 'google.com/aimode',
    loginMode: 'guest_web_system_login',
    profileScope: 'local_owner_provider',
    rendererProtocol: 'yilong.ai.ui.v1',
    rendererStatus: 'active',
  },
  chatgpt: {
    id: 'chatgpt',
    displayName: 'ChatGPT',
    startHost: 'chatgpt.com',
    loginMode: 'manual_web',
    profileScope: 'local_owner_provider',
    rendererProtocol: 'yilong.ai.ui.v1',
    rendererStatus: 'active',
  },
}

export default function NativeAiWebChatWindow() {
  const [searchParams] = useSearchParams()
  const requestedProviderId = searchParams.get('provider') || ''
  const user = useAuthStore((state) => state.user)
  const ownerKey = user?.id || ''
  const [providers, setProviders] = useState<LocalAiWebProvider[]>([])
  const [sessionState, setSessionState] = useState<LocalAiWebSessionState | null>(null)
  const [draft, setDraft] = useState('')
  const [draftTouched, setDraftTouched] = useState(false)
  const [busyAction, setBusyAction] = useState('')
  const [message, setMessage] = useState('')
  const [loading, setLoading] = useState(true)
  const provider = providers.find((item) => item.id === requestedProviderId)
    || PROVIDER_FALLBACKS[requestedProviderId]
  const snapshot = useMemo(
    () => isLocalAiMessageSnapshot(sessionState?.semanticEvent) ? sessionState.semanticEvent : null,
    [sessionState?.semanticEvent],
  )
  const sessionOpen = Boolean(sessionState && sessionState.windowStatus !== 'closed')

  useEffect(() => {
    let active = true
    void listLocalAiWebProviders()
      .then((items) => { if (active) setProviders(items) })
      .catch((error) => { if (active) setMessage(localAiBrowserErrorMessage(error)) })
      .finally(() => { if (active) setLoading(false) })
    return () => { active = false }
  }, [])

  useEffect(() => {
    if (!provider || !ownerKey) return
    let active = true
    const poll = async () => {
      try {
        const next = await getLocalAiWebSessionState(provider.id, ownerKey)
        if (active) setSessionState(next)
      } catch (error) {
        if (active) setMessage(localAiBrowserErrorMessage(error))
      }
    }
    void poll()
    const timer = window.setInterval(() => void poll(), 900)
    return () => {
      active = false
      window.clearInterval(timer)
    }
  }, [ownerKey, provider])

  useEffect(() => {
    if (!draftTouched) setDraft(snapshot?.draft ?? '')
  }, [draftTouched, snapshot?.draft])

  async function openOfficial() {
    if (!provider || !ownerKey || busyAction) return
    setBusyAction('open')
    setMessage('')
    try {
      await openLocalAiWebSession(provider.id, ownerKey)
      setSessionState(await getLocalAiWebSessionState(provider.id, ownerKey))
      setMessage(`已显示 ${provider.displayName} 官方窗口，请在那里完成登录或真人验证。`)
    } catch (error) {
      setMessage(localAiBrowserErrorMessage(error))
    } finally {
      setBusyAction('')
    }
  }

  async function control(action: LocalAiBrowserControlAction) {
    if (!provider || !ownerKey || busyAction) return
    setBusyAction(action)
    setMessage('')
    try {
      setSessionState(await controlLocalAiWebSession(provider.id, ownerKey, action))
      if (action === 'external') {
        setMessage('已打开系统浏览器；系统浏览器不会与一龙本地窗口共享 Cookie。')
      }
    } catch (error) {
      setMessage(localAiBrowserErrorMessage(error))
    } finally {
      setBusyAction('')
    }
  }

  async function run(action: LocalAiAdapterAction, value?: string, expectedDraft?: string) {
    if (!provider || !ownerKey || busyAction) return
    setBusyAction(action)
    setMessage('')
    try {
      await runLocalAiWebAdapterCommand(provider.id, ownerKey, action, value, expectedDraft)
      const next = await waitForLocalAiAdapterResult(provider.id, ownerKey, action)
      if (next) setSessionState(next)
      const result = next?.commandResult
      if (result?.action === action && !result.ok) {
        setMessage(result.detail || '官方网页没有完成这个动作，请显示官方窗口后重试。')
      } else if (action === 'send_prompt') {
        setDraft('')
        setDraftTouched(false)
        setMessage(result?.detail || '消息已交给官方网页发送。')
      } else if (result?.detail) {
        setMessage(result.detail)
      }
    } catch (error) {
      setMessage(localAiBrowserErrorMessage(error))
    } finally {
      setBusyAction('')
    }
  }

  if (loading) return <WindowNotice icon="loading" title="正在打开一龙聊天窗" detail="正在读取本机官方 AI 能力…" />
  if (!ownerKey) return <WindowNotice title="请先登录一龙账号" detail="关闭此窗口，在一龙主窗口登录后重新打开。" />
  if (!provider) return <WindowNotice title="厂商窗口无效" detail="请从一龙主窗口的“官方 AI”入口重新打开。" />

  return (
    <main className={styles.window}>
      <header className={styles.toolbar}>
        <div className={styles.identity}>
          <span>YILONG NATIVE AI</span>
          <strong>{provider.displayName}</strong>
          <small>{statusLabel(sessionState)} · Cookie 仅在官方 WebView2 内</small>
        </div>
        <div className={styles.actions}>
          <button type="button" onClick={() => void openOfficial()} disabled={Boolean(busyAction)}>
            {busyAction === 'open' ? <LoaderCircle className={styles.spin} size={15} /> : <MonitorUp size={15} />}
            {sessionOpen ? '显示官方窗口' : '打开官方窗口'}
          </button>
          <button type="button" onClick={() => void control('reload')} disabled={!sessionOpen || Boolean(busyAction)}>
            <RefreshCw size={15} />刷新官方页
          </button>
          <button type="button" onClick={() => void control('external')} disabled={Boolean(busyAction)}>
            <ExternalLink size={15} />系统浏览器
          </button>
        </div>
      </header>

      {message && <p className={styles.message}>{message}</p>}
      {sessionState?.lastError && <p className={styles.error}>{sessionState.lastError}</p>}

      <div className={styles.chatFrame}>
        <NativeAiWebChat
          provider={provider}
          snapshot={snapshot}
          sessionOpen={sessionOpen}
          busy={Boolean(busyAction)}
          draft={draft}
          onDraftChange={(value) => {
            setDraft(value)
            setDraftTouched(true)
          }}
          onRun={(action, value, expectedDraft) => void run(action, value, expectedDraft)}
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
  if (state.rendererStatus === 'active') return '一龙界面已连接'
  if (state.loading) return '正在同步官方页面'
  return '等待官方页面'
}
