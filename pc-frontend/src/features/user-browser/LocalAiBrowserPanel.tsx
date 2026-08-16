import { useEffect, useMemo, useState } from 'react'
import {
  AlertTriangle,
  ArrowLeft,
  Download,
  ExternalLink,
  Home,
  LoaderCircle,
  MonitorUp,
  RefreshCw,
  ShieldCheck,
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
  type LocalAiBrowserControlAction,
  type LocalAiWebProvider,
  type LocalAiWebSessionState,
} from './localAiBrowserApi'
import type { LocalAiBrowserCapability } from './useLocalAiBrowserCapability'
import type { LocalAiOwnerSource } from './useLocalAiOwnerIdentity'
import AiProviderSessionStatus from './AiProviderSessionStatus'
import { deriveLocalAiUserState } from './localAiUserState'
import styles from './LocalAiBrowserPanel.module.css'

interface LocalAiBrowserPanelProps {
  ownerKey?: string
  ownerLabel: string
  ownerSource: LocalAiOwnerSource
  capability: LocalAiBrowserCapability
}

export default function LocalAiBrowserPanel({ ownerKey, ownerLabel, ownerSource, capability }: LocalAiBrowserPanelProps) {
  const { state, providers, message: capabilityMessage, refresh } = capability
  const [busyProvider, setBusyProvider] = useState<string | null>(null)
  const [sessionState, setSessionState] = useState<LocalAiWebSessionState | null>(null)
  const [message, setMessage] = useState('')
  const [selectedProviderId, setSelectedProviderId] = useState('google-ai-mode')
  const provider = providers.find((item) => item.id === selectedProviderId) || providers[0]
  const semanticSnapshot = useMemo(
    () => isLocalAiMessageSnapshot(sessionState?.semanticEvent) ? sessionState.semanticEvent : null,
    [sessionState?.semanticEvent],
  )
  const userState = useMemo(
    () => deriveLocalAiUserState(state, provider, sessionState, semanticSnapshot),
    [provider, semanticSnapshot, sessionState, state],
  )

  useEffect(() => {
    setSessionState(null)
    setMessage('')
  }, [provider?.id])

  useEffect(() => {
    if (!ownerKey || !provider || state !== 'ready') return
    let active = true
    let timer = 0
    const poll = async () => {
      try {
        const next = await getLocalAiWebSessionState(provider.id, ownerKey)
        if (active) setSessionState(next)
      } catch {
        // 能力检查负责显示兼容性错误；短暂轮询失败不覆盖用户正在看的消息。
      } finally {
        if (active) timer = window.setTimeout(() => void poll(), 1_500)
      }
    }
    void poll()
    return () => {
      active = false
      window.clearTimeout(timer)
    }
  }, [ownerKey, provider, state])

  async function open(item: LocalAiWebProvider) {
    if (!ownerKey || busyProvider) return
    setBusyProvider(item.id)
    setMessage('')
    try {
      const session = await openLocalAiWebSession(item.id, ownerKey)
      try {
        setSessionState(await getLocalAiWebSessionState(item.id, ownerKey))
      } catch {
        // 官方窗口已成功恢复时，状态刷新失败不能把打开动作判为失败。
      }
      setMessage(session.status === 'created'
        ? `已打开 ${item.displayName} 官方页面。访客模式优先，登录用于历史与增强能力。`
        : `已恢复 ${item.displayName} 官方页面。`)
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
        <p>
          一龙先尝试厂商官方网页的访客能力；官网输入框可用就直接进入原生聊天。
          登录只用于历史、项目、个性化或厂商要求的验证，Cookie 仍只留在本机 WebView2 Profile。
        </p>
        <dl>
          <div><dt>一龙账号</dt><dd>{ownerLabel}</dd></div>
          <div><dt>身份来源</dt><dd>{ownerSourceLabel(ownerSource)}</dd></div>
          <div><dt>网页登录</dt><dd>仅当前电脑</dd></div>
          <div><dt>使用状态</dt><dd>{userState.badge}</dd></div>
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
              <li><span>1</span><strong>选择 ChatGPT 或 Google AI</strong></li>
              <li><span>2</span><strong>直接使用官网访客模式</strong></li>
              <li><span>3</span><strong>需要历史或增强能力时再登录</strong></li>
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

            <AiProviderSessionStatus state={userState} />
            {provider && (
              <div className={styles.actions}>
                <button
                  className={styles.openButton}
                  type="button"
                  onClick={() => void open(provider)}
                  disabled={!ownerKey || busy}
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

            <section className={styles.officialWebOnly} aria-label={`${provider?.displayName || '官方 AI'}接入说明`}>
              <ExternalLink size={24} aria-hidden="true" />
              <div>
                <strong>{provider?.displayName || '官方 AI 网页'}</strong>
                <p>
                  生产聊天已统一到“一龙 AI”的 Chat 界面；这里仅管理和恢复同一官方网页会话。
                  一龙不会读取网页凭证、请求或私有接口，厂商能力仍由其地区、语言、设备和账号策略决定。
                </p>
                <small>可继续使用上方“显示窗口”或“系统浏览器”。</small>
              </div>
            </section>
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

function openButtonLabel(provider: LocalAiWebProvider, sessionOpen: boolean): string {
  if (sessionOpen) return '恢复官方页'
  return `打开 ${provider.displayName}（访客可用）`
}

function ownerSourceLabel(source: LocalAiOwnerSource): string {
  if (source === 'cloud_account') return '一龙云端账号'
  if (source === 'local_node') return '本机节点恢复'
  if (source === 'anonymous_device') return '本机访客隔离身份'
  if (source === 'conflict') return '账号不一致 · 已暂停'
  return '尚未识别'
}
