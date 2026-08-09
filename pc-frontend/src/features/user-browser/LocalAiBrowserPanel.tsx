import { useEffect, useMemo, useState } from 'react'
import {
  AlertTriangle,
  ArrowLeft,
  Download,
  ExternalLink,
  Home,
  LoaderCircle,
  MessageSquarePlus,
  MonitorUp,
  RefreshCw,
  Send,
  ShieldCheck,
  Square,
  Trash2,
} from 'lucide-react'
import { WIN_CLIENT_DOWNLOAD_URL } from '../node/launchWinClient'
import {
  clearLocalAiWebSession,
  controlLocalAiWebSession,
  getLocalAiWebSessionState,
  isLocalAiMessageSnapshot,
  localAiBrowserErrorMessage,
  openLocalAiWebSession,
  runLocalAiWebAdapterCommand,
  type LocalAiAdapterAction,
  type LocalAiBrowserControlAction,
  type LocalAiWebProvider,
  type LocalAiWebSessionState,
} from './localAiBrowserApi'
import type { LocalAiBrowserCapability } from './useLocalAiBrowserCapability'
import styles from './LocalAiBrowserPanel.module.css'

interface LocalAiBrowserPanelProps {
  ownerKey?: string
  ownerLabel: string
  capability: LocalAiBrowserCapability
}

export default function LocalAiBrowserPanel({ ownerKey, ownerLabel, capability }: LocalAiBrowserPanelProps) {
  const { state, providers, message: capabilityMessage, refresh } = capability
  const [ownerConfirmed, setOwnerConfirmed] = useState(false)
  const [busyProvider, setBusyProvider] = useState<string | null>(null)
  const [sessionState, setSessionState] = useState<LocalAiWebSessionState | null>(null)
  const [message, setMessage] = useState('')
  const [draft, setDraft] = useState('')
  const [draftTouched, setDraftTouched] = useState(false)
  const [selectedProviderId, setSelectedProviderId] = useState('google-ai-mode')
  const provider = providers.find((item) => item.id === selectedProviderId) || providers[0]
  const snapshot = useMemo(
    () => isLocalAiMessageSnapshot(sessionState?.semanticEvent) ? sessionState.semanticEvent : null,
    [sessionState?.semanticEvent],
  )

  useEffect(() => {
    setSessionState(null)
    setOwnerConfirmed(false)
    setDraft('')
    setDraftTouched(false)
    setMessage('')
  }, [provider?.id])

  useEffect(() => {
    if (!ownerKey || !provider || state !== 'ready') return
    let active = true
    const poll = async () => {
      try {
        const next = await getLocalAiWebSessionState(provider.id, ownerKey)
        if (active) setSessionState(next)
      } catch {
        // 能力检查负责显示兼容性错误；短暂轮询失败不覆盖用户正在看的消息。
      }
    }
    void poll()
    const timer = window.setInterval(() => void poll(), 1_200)
    return () => {
      active = false
      window.clearInterval(timer)
    }
  }, [ownerKey, provider, state])

  useEffect(() => {
    if (!draftTouched) setDraft(snapshot?.draft ?? '')
  }, [draftTouched, snapshot?.draft])

  async function open(item: LocalAiWebProvider) {
    if (!ownerKey || !ownerConfirmed || busyProvider) return
    setBusyProvider(item.id)
    setMessage('')
    try {
      const session = await openLocalAiWebSession(item.id, ownerKey)
      const next = await getLocalAiWebSessionState(item.id, ownerKey)
      setSessionState(next)
      setMessage(session.status === 'created'
        ? item.loginMode === 'guest_web_system_login'
          ? `已打开 ${item.displayName} 官方页面。若 Google 要求登录，请使用“系统浏览器”；两边不会共享 Cookie。`
          : `已打开 ${item.displayName} 官方页面，请本人完成登录。登录后可回到这里使用一龙聊天界面。`
        : `已恢复并聚焦 ${item.displayName} 官方页面。`)
    } catch (error) {
      setMessage(localAiBrowserErrorMessage(error))
    } finally {
      setBusyProvider(null)
    }
  }

  async function control(action: LocalAiBrowserControlAction) {
    if (!ownerKey || !provider || busyProvider) return
    setBusyProvider(provider.id)
    setMessage('')
    try {
      const next = await controlLocalAiWebSession(provider.id, ownerKey, action)
      setSessionState(next)
      setMessage(action === 'external'
        ? `已用系统浏览器打开 ${provider.displayName}。系统浏览器与一龙本地窗口不会共享 Cookie。`
        : '')
    } catch (error) {
      setMessage(localAiBrowserErrorMessage(error))
    } finally {
      setBusyProvider(null)
    }
  }

  async function runAdapter(action: LocalAiAdapterAction, value?: string, expectedDraft?: string) {
    if (!ownerKey || !provider || busyProvider) return
    setBusyProvider(provider.id)
    setMessage('')
    try {
      await runLocalAiWebAdapterCommand(provider.id, ownerKey, action, value, expectedDraft)
      const next = await waitForAdapterResult(provider.id, ownerKey, action)
      if (next) setSessionState(next)
      const result = next?.commandResult
      if (result?.action === action && !result.ok) {
        setMessage(result.detail || 'ChatGPT 官方网页没有完成这个动作，请显示官方窗口后重试。')
        return
      }
      if (action === 'send_prompt') {
        setDraft('')
        setDraftTouched(false)
        setMessage(result?.detail || '消息已交给 ChatGPT 官方网页发送。')
      } else if (result?.detail) {
        setMessage(result.detail)
      }
    } catch (error) {
      setMessage(localAiBrowserErrorMessage(error))
    } finally {
      setBusyProvider(null)
    }
  }

  async function clear(item: LocalAiWebProvider) {
    if (!ownerKey || busyProvider) return
    const confirmed = window.confirm(
      `清除 ${item.displayName} 在当前一龙账号下的本地 Cookie、缓存和网页存储？`,
    )
    if (!confirmed) return
    setBusyProvider(item.id)
    setMessage('')
    try {
      await clearLocalAiWebSession(item.id, ownerKey)
      setSessionState(null)
      setOwnerConfirmed(false)
      setDraft('')
      setMessage(`${item.displayName} 本地网页会话已经清除。`)
    } catch (error) {
      setMessage(localAiBrowserErrorMessage(error))
    } finally {
      setBusyProvider(null)
    }
  }

  const sessionOpen = Boolean(sessionState && sessionState.windowStatus !== 'closed')
  const busy = Boolean(busyProvider)

  return (
    <section className={styles.panel} aria-labelledby="local-ai-browser-title">
      <div className={styles.intro}>
        <div className={styles.titleRow}>
          <MonitorUp size={20} aria-hidden="true" />
          <div>
            <span>OFFICIAL AI WEB · 本机隔离</span>
            <h2 id="local-ai-browser-title">{provider?.displayName || '官方 AI 网页'}</h2>
          </div>
        </div>
        {provider?.loginMode === 'guest_web_system_login' ? (
          <p>
            一龙直接打开 Google 官方 AI 模式。访客能力留在本机 WebView2；Google 账号登录
            按官方要求交给系统浏览器，一龙不接收登录 Cookie 或访问令牌。
          </p>
        ) : (
          <p>
            一龙打开 ChatGPT 官方页面，由本人输入账号并完成人机验证。Cookie 和网页数据
            只留在本机 WebView2 Profile；原生聊天区只同步屏幕上可见的消息语义。
          </p>
        )}
        <dl>
          <div><dt>一龙账号</dt><dd>{ownerLabel}</dd></div>
          <div><dt>登录位置</dt><dd>仅当前电脑</dd></div>
          <div><dt>窗口状态</dt><dd>{sessionStatusLabel(sessionState)}</dd></div>
          <div><dt>当前站点</dt><dd>{sessionState?.currentHost || '尚未打开'}</dd></div>
        </dl>
      </div>

      <div className={styles.controls}>
        {state === 'desktop_required' ? (
          <div className={styles.desktopNotice}>
            <MonitorUp size={18} />
            <span>请在一龙 Windows 客户端打开本页；普通浏览器继续使用下方托管模式。</span>
          </div>
        ) : state === 'checking' ? (
          <div className={styles.desktopNotice}>
            <LoaderCircle className={styles.spin} size={18} />
            <span>正在确认当前 Win 客户端是否支持官方 AI 网页…</span>
          </div>
        ) : state === 'upgrade_required' ? (
          <Notice title="当前 Win 客户端需要更新" message={capabilityMessage}>
            <a href={WIN_CLIENT_DOWNLOAD_URL} target="_blank" rel="noopener noreferrer">
              <Download size={15} />下载新版 Win 客户端
            </a>
            <button type="button" onClick={() => void refresh()}>
              <RefreshCw size={15} />更新后重试
            </button>
          </Notice>
        ) : state === 'error' ? (
          <Notice title="本地官方 AI 网页功能暂不可用" message={capabilityMessage}>
            <button type="button" onClick={() => void refresh()}>
              <RefreshCw size={15} />重新检查
            </button>
          </Notice>
        ) : (
          <>
            <ol className={styles.steps} aria-label={`${provider?.displayName || '官方 AI 网页'}使用步骤`}>
              {provider?.loginMode === 'guest_web_system_login' ? (
                <>
                  <li><span>1</span><strong>选择 Google AI 模式</strong></li>
                  <li><span>2</span><strong>在本地窗口使用官方网页</strong></li>
                  <li><span>3</span><strong>需要账号时改用系统浏览器</strong></li>
                </>
              ) : (
                <>
                  <li><span>1</span><strong>确认只登录本人账号</strong></li>
                  <li><span>2</span><strong>打开官方页面并完成登录</strong></li>
                  <li><span>3</span><strong>回到本页使用一龙聊天界面</strong></li>
                </>
              )}
            </ol>

            <div className={styles.providerList} role="list" aria-label="官方 AI 网页提供商">
              {providers.map((item) => (
                <button
                  className={styles.providerChoice}
                  data-selected={item.id === provider?.id}
                  key={item.id}
                  type="button"
                  onClick={() => setSelectedProviderId(item.id)}
                >
                  <span>
                    <strong>{item.displayName}</strong>
                    <small>{item.startHost} · 独立本地 Profile</small>
                  </span>
                  <em>{item.rendererStatus === 'active' ? '一龙界面' : '官方网页'}</em>
                </button>
              ))}
            </div>

            <label className={styles.confirmation}>
              <input
                type="checkbox"
                checked={ownerConfirmed}
                onChange={(event) => setOwnerConfirmed(event.target.checked)}
                disabled={!ownerKey}
              />
              <span>
                <strong>
                  {provider?.loginMode === 'guest_web_system_login'
                    ? '我了解 Google 登录使用系统浏览器'
                    : '只登录本人账号'}
                </strong>
                <small>
                  {provider?.loginMode === 'guest_web_system_login'
                    ? '本地窗口适合访客模式；账号状态不会从系统浏览器复制回来。'
                    : '真人验证由本人完成；一龙不读取密码、Cookie 或访问令牌。'}
                </small>
              </span>
            </label>

            {provider && (
              <div className={styles.actions}>
                <button
                  className={styles.openButton}
                  type="button"
                  onClick={() => void open(provider)}
                  disabled={!ownerKey || !ownerConfirmed || busy}
                >
                  {busy ? <LoaderCircle className={styles.spin} size={16} /> : <ShieldCheck size={16} />}
                  {openButtonLabel(provider, sessionOpen)}
                </button>
                <button
                  className={styles.clearButton}
                  type="button"
                  title="清除当前账号的本地网页会话"
                  aria-label={`清除 ${provider.displayName} 本地网页会话`}
                  onClick={() => void clear(provider)}
                  disabled={!sessionOpen || busy}
                >
                  <Trash2 size={16} />
                </button>
              </div>
            )}

            <div className={styles.browserToolbar} aria-label={`${provider?.displayName || '官方 AI'}窗口控制`}>
              <button type="button" onClick={() => void control('back')} disabled={!sessionOpen || busy}>
                <ArrowLeft size={15} />返回
              </button>
              <button type="button" onClick={() => void control('reload')} disabled={!sessionOpen || busy}>
                <RefreshCw size={15} />刷新
              </button>
              <button type="button" onClick={() => void control('home')} disabled={!sessionOpen || busy}>
                <Home size={15} />主页
              </button>
              <button type="button" onClick={() => void control('restore')} disabled={!sessionOpen || busy}>
                <MonitorUp size={15} />显示窗口
              </button>
              <button type="button" onClick={() => void control('external')} disabled={!ownerKey || busy}>
                <ExternalLink size={15} />系统浏览器
              </button>
            </div>

            {sessionState?.lastError && (
              <div className={styles.sessionError} role="alert">
                <AlertTriangle size={16} />
                <span>{sessionState.lastError}</span>
              </div>
            )}

            {provider?.rendererStatus === 'active' ? <section className={styles.nativeChat} aria-label="一龙 ChatGPT 原生聊天区">
              <header>
                <div>
                  <strong>{snapshot?.title || 'ChatGPT 原生聊天'}</strong>
                  <small>
                    {snapshot?.authenticated
                      ? `${snapshot.currentModel || '官方网页模型'} · 本机同步`
                      : '请先在官方窗口登录'}
                  </small>
                </div>
                <button
                  type="button"
                  title="新建对话"
                  onClick={() => void runAdapter('new_conversation')}
                  disabled={!snapshot?.authenticated || busy}
                >
                  <MessageSquarePlus size={17} />
                </button>
              </header>

              <div className={styles.messageList} aria-live="polite">
                {snapshot?.messages.length ? snapshot.messages.map((item) => (
                  <article className={item.role === 'user' ? styles.userMessage : styles.assistantMessage} key={item.id}>
                    <span>{item.role === 'user' ? '你' : 'ChatGPT'}</span>
                    {item.content.map((part, index) => <p key={`${item.id}-${index}`}>{part.text}</p>)}
                  </article>
                )) : (
                  <div className={styles.emptyChat}>
                    <MonitorUp size={24} />
                    <strong>{sessionOpen ? '等待 ChatGPT 官方页面' : '尚未打开 ChatGPT'}</strong>
                    <p>完成官方登录后，可见对话会自动同步到这里；遇到真人验证请在官方窗口本人点击。</p>
                    {sessionOpen && !snapshot?.authenticated && (
                      <button type="button" onClick={() => void runAdapter('start_google_login')} disabled={busy}>
                        尝试打开官方 Google 登录
                      </button>
                    )}
                  </div>
                )}
              </div>

              <div className={styles.composer}>
                <textarea
                  value={draft}
                  onChange={(event) => {
                    setDraft(event.target.value)
                    setDraftTouched(true)
                  }}
                  placeholder={snapshot?.composerReady ? '向 ChatGPT 发送消息…' : '登录后即可使用原生输入框'}
                  disabled={!snapshot?.authenticated || !snapshot.composerReady || busy}
                  maxLength={20_000}
                />
                {snapshot?.streaming ? (
                  <button type="button" title="停止生成" onClick={() => void runAdapter('stop_generation')} disabled={busy}>
                    <Square size={16} />
                  </button>
                ) : (
                  <button
                    type="button"
                    title="发送"
                    onClick={() => void runAdapter('send_prompt', draft, snapshot?.draft ?? '')}
                    disabled={!snapshot?.composerReady || !draft.trim() || busy}
                  >
                    <Send size={16} />
                  </button>
                )}
              </div>
            </section> : (
              <section className={styles.officialWebOnly} aria-label="Google AI 模式接入说明">
                <ExternalLink size={24} aria-hidden="true" />
                <div>
                  <strong>Google AI 模式官方网页</strong>
                  <p>
                    当前批次直接使用 Google 官方界面，不注入 ChatGPT 适配器，也不读取网页 Cookie、
                    请求或私有接口。是否开放由 Google 的地区、语言、设备和账号灰度决定。
                  </p>
                  <small>需要登录或查看账号历史时，请点击上方“系统浏览器”。</small>
                </div>
              </section>
            )}
          </>
        )}

        {message && <p className={styles.message}>{message}</p>}
      </div>
    </section>
  )
}

function Notice({ title, message, children }: { title: string; message: string; children: React.ReactNode }) {
  return (
    <div className={styles.upgradeNotice}>
      <AlertTriangle size={20} aria-hidden="true" />
      <div>
        <strong>{title}</strong>
        <p>{message}</p>
        <div className={styles.noticeActions}>{children}</div>
      </div>
    </div>
  )
}

function sessionStatusLabel(state: LocalAiWebSessionState | null): string {
  if (!state) return '尚未打开'
  const labels: Record<string, string> = {
    opening: '正在打开',
    loading: '正在加载',
    ready: '已打开',
    minimized: '已最小化',
    blocked: '导航已拦截',
    error: '加载异常',
    closed: '已关闭',
  }
  return labels[state.windowStatus] || state.windowStatus
}

function openButtonLabel(provider: LocalAiWebProvider, sessionOpen: boolean): string {
  if (sessionOpen) return '恢复官方窗口'
  return provider.id === 'chatgpt' ? '登录或打开 ChatGPT' : `打开 ${provider.displayName}`
}

async function waitForAdapterResult(
  providerId: string,
  ownerKey: string,
  action: string,
): Promise<LocalAiWebSessionState | null> {
  for (let attempt = 0; attempt < 12; attempt += 1) {
    await new Promise((resolve) => window.setTimeout(resolve, 200))
    const state = await getLocalAiWebSessionState(providerId, ownerKey)
    if (state.commandResult?.action === action) return state
  }
  return null
}
