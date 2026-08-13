import { ExternalLink, EyeOff, LoaderCircle, MonitorUp, RefreshCw, ShieldCheck } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import { useAuthStore } from '../../store/auth'
import AiHomeModeSwitch, { type AiHomeMode } from '../ai/AiHomeModeSwitch'
import SidebarUserStrip from '../shell/SidebarUserStrip'
import NativeAiWebChat from './NativeAiWebChat'
import { isLocalAiBrowserAvailable } from './localAiBrowserApi'
import {
  DEFAULT_LOCAL_AI_PROVIDER_ID,
  LOCAL_AI_PROVIDER_FALLBACKS,
} from './localAiWebProviders'
import useLocalAiBrowserCapability from './useLocalAiBrowserCapability'
import useLocalAiWebChatController from './useLocalAiWebChatController'
import styles from './UnifiedWebChat.module.css'

const PROVIDER_STORAGE_KEY = 'elon.pc.aiChatProvider'

export default function UnifiedWebChat({
  mode,
  onModeChange,
  onLogin,
}: {
  mode: AiHomeMode
  onModeChange: (mode: AiHomeMode) => void
  onLogin: () => void
}) {
  const user = useAuthStore((state) => state.user)
  const capability = useLocalAiBrowserCapability()
  const [providerId, setProviderId] = useState(() => readProviderPreference())
  const visibleProviders = useMemo(() => (
    capability.providers.length
      ? capability.providers
      : Object.values(LOCAL_AI_PROVIDER_FALLBACKS)
  ), [capability.providers])
  const provider = visibleProviders.find((item) => item.id === providerId) || visibleProviders[0]
  const controller = useLocalAiWebChatController(
    capability.state === 'ready' ? provider : undefined,
    user?.id || '',
  )

  useEffect(() => {
    if (!provider || provider.id === providerId) return
    setProviderId(provider.id)
  }, [provider, providerId])

  function selectProvider(id: string) {
    setProviderId(id)
    try { window.localStorage.setItem(PROVIDER_STORAGE_KEY, id) } catch {}
  }

  const ready = capability.state === 'ready' && Boolean(user?.id && provider)
  const officialVisible = Boolean(controller.sessionState?.windowVisible)

  return (
    <div className={styles.layout}>
      <aside className={styles.sidebar}>
        <div className={styles.sideHeader}>
          <span>一龙 AI</span>
          <small>CHAT</small>
        </div>
        <div className={styles.providerIntro}>
          <strong>选择网页 AI</strong>
          <p>使用统一的一龙聊天界面，厂商网页会话只在这台电脑运行。</p>
        </div>
        <div className={styles.providerList}>
          {visibleProviders.map((item) => (
            <button
              type="button"
              key={item.id}
              data-active={item.id === provider?.id}
              onClick={() => selectProvider(item.id)}
            >
              <span>{item.id === 'chatgpt' ? '◎' : 'G'}</span>
              <div>
                <strong>{item.displayName}</strong>
                <small>{item.id === 'chatgpt' ? 'ChatGPT 网页 Chat' : 'Google 搜索 AI 模式'}</small>
              </div>
            </button>
          ))}
        </div>
        <div className={styles.localPrivacy}>
          <ShieldCheck size={15} />
          <span>Cookie 不进入一龙云端</span>
        </div>
        <SidebarUserStrip />
      </aside>

      <main className={styles.chat}>
        <header className={styles.topbar}>
          <div className={styles.identity}>
            <strong>{provider?.displayName || '网页 AI'}</strong>
            <span>{chatStatus(controller.sessionState, capability.state)}</span>
          </div>
          <AiHomeModeSwitch mode={mode} onChange={onModeChange} />
          <div className={styles.actions}>
            <button
              type="button"
              onClick={() => void controller.openOfficial()}
              disabled={!ready || Boolean(controller.busyAction)}
            >
              {controller.busyAction === 'open'
                ? <LoaderCircle className={styles.spin} size={15} />
                : <MonitorUp size={15} />}
              {controller.sessionOpen ? '显示官方页' : '登录 / 打开'}
            </button>
            <button
              type="button"
              onClick={() => void controller.control('background')}
              disabled={!ready || !controller.sessionOpen || !officialVisible || Boolean(controller.busyAction)}
            >
              <EyeOff size={15} />收起到后台
            </button>
            <button
              type="button"
              title="刷新后台官方页面"
              onClick={() => void controller.control('reload')}
              disabled={!ready || !controller.sessionOpen || Boolean(controller.busyAction)}
            >
              <RefreshCw size={15} />刷新
            </button>
            <button
              type="button"
              title="使用不共享 Cookie 的系统浏览器"
              onClick={() => void controller.control('external')}
              disabled={!isLocalAiBrowserAvailable() || Boolean(controller.busyAction)}
            >
              <ExternalLink size={15} />浏览器
            </button>
          </div>
        </header>

        {!user?.id ? (
          <ChatNotice
            title="登录一龙账号后使用 Chat 模式"
            detail="一龙账号只用于隔离本机厂商会话；ChatGPT 或 Google 仍由你本人在官方页面登录。"
            action="登录一龙"
            onAction={onLogin}
          />
        ) : capability.state !== 'ready' ? (
          <ChatNotice
            title={capability.state === 'desktop_required' ? 'Chat 模式需要一龙 Win 客户端' : '正在连接本地网页 AI'}
            detail={capability.message || '正在读取 ChatGPT 与 Google AI 模式能力…'}
            action={capability.state === 'error' || capability.state === 'upgrade_required' ? '重新检查' : undefined}
            onAction={() => void capability.refresh()}
          />
        ) : provider ? (
          <>
            {(controller.message || controller.sessionState?.lastError) && (
              <p className={styles.message} data-error={Boolean(controller.sessionState?.lastError)}>
                {controller.sessionState?.lastError || controller.message}
              </p>
            )}
            <div className={styles.chatFrame}>
              <NativeAiWebChat
                provider={provider}
                snapshot={controller.snapshot}
                sessionOpen={controller.sessionOpen}
                busy={Boolean(controller.busyAction)}
                draft={controller.draft}
                onDraftChange={controller.setDraft}
                onRun={(action, value, expectedDraft) => void controller.run(action, value, expectedDraft)}
                emptyTitle={`在一龙界面使用 ${provider.displayName}`}
                standalone
              />
            </div>
          </>
        ) : null}
      </main>
    </div>
  )
}

function ChatNotice({
  title,
  detail,
  action,
  onAction,
}: {
  title: string
  detail: string
  action?: string
  onAction: () => void
}) {
  return (
    <section className={styles.notice}>
      <MonitorUp size={25} />
      <strong>{title}</strong>
      <p>{detail}</p>
      {action && <button type="button" onClick={onAction}>{action}</button>}
    </section>
  )
}

function chatStatus(
  state: ReturnType<typeof useLocalAiWebChatController>['sessionState'],
  capability: ReturnType<typeof useLocalAiBrowserCapability>['state'],
) {
  if (capability !== 'ready') return '本地 WebView2 未连接'
  if (!state || state.windowStatus === 'closed') return '官方会话未打开'
  if (state.windowVisible) return '官方页可见 · 可完成登录验证'
  if (state.rendererStatus === 'active') return '官方页在后台 · 一龙 UI 已连接'
  return state.loading ? '正在同步官方页面' : '等待官方页面就绪'
}

function readProviderPreference() {
  try {
    const stored = window.localStorage.getItem(PROVIDER_STORAGE_KEY) || ''
    if (stored in LOCAL_AI_PROVIDER_FALLBACKS) return stored
  } catch {}
  return DEFAULT_LOCAL_AI_PROVIDER_ID
}
