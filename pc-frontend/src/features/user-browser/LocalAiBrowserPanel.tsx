import { useEffect, useState } from 'react'
import { LoaderCircle, MonitorUp, ShieldCheck, Trash2 } from 'lucide-react'
import {
  clearLocalAiWebSession,
  isLocalAiBrowserAvailable,
  listLocalAiWebProviders,
  openLocalAiWebSession,
  type LocalAiWebProvider,
} from './localAiBrowserApi'
import styles from './LocalAiBrowserPanel.module.css'

interface LocalAiBrowserPanelProps {
  ownerKey?: string
  ownerLabel: string
}

type PanelState = 'desktop_required' | 'loading' | 'ready' | 'error'

export default function LocalAiBrowserPanel({ ownerKey, ownerLabel }: LocalAiBrowserPanelProps) {
  const desktopAvailable = isLocalAiBrowserAvailable()
  const [state, setState] = useState<PanelState>(desktopAvailable ? 'loading' : 'desktop_required')
  const [providers, setProviders] = useState<LocalAiWebProvider[]>([])
  const [ownerConfirmed, setOwnerConfirmed] = useState(false)
  const [busyProvider, setBusyProvider] = useState<string | null>(null)
  const [openedProvider, setOpenedProvider] = useState<string | null>(null)
  const [message, setMessage] = useState('')

  useEffect(() => {
    if (!desktopAvailable) return
    let cancelled = false
    void listLocalAiWebProviders()
      .then((items) => {
        if (cancelled) return
        setProviders(items)
        setState('ready')
        setMessage(items.length ? '' : '桌面壳尚未登记可用的 AI 网页厂商。')
      })
      .catch((error: unknown) => {
        if (cancelled) return
        setState('error')
        setMessage(errorMessage(error))
      })
    return () => { cancelled = true }
  }, [desktopAvailable])

  async function open(provider: LocalAiWebProvider) {
    if (!ownerKey || !ownerConfirmed || busyProvider) return
    setBusyProvider(provider.id)
    setMessage('')
    try {
      const session = await openLocalAiWebSession(provider.id, ownerKey)
      setOpenedProvider(provider.id)
      setMessage(session.status === 'created'
        ? `${provider.displayName} 本地独立会话已创建。`
        : `${provider.displayName} 本地会话已重新聚焦。`)
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusyProvider(null)
    }
  }

  async function clear(provider: LocalAiWebProvider) {
    if (!ownerKey || busyProvider) return
    const confirmed = window.confirm(
      `清除 ${provider.displayName} 在当前一龙账号下的本地 Cookie、缓存和网页存储？`,
    )
    if (!confirmed) return
    setBusyProvider(provider.id)
    setMessage('')
    try {
      await clearLocalAiWebSession(provider.id, ownerKey)
      setOpenedProvider(null)
      setOwnerConfirmed(false)
      setMessage(`${provider.displayName} 本地网页会话已经清除。`)
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusyProvider(null)
    }
  }

  return (
    <section className={styles.panel} aria-labelledby="local-ai-browser-title">
      <div className={styles.intro}>
        <div className={styles.titleRow}>
          <MonitorUp size={20} aria-hidden="true" />
          <div>
            <span>WINDOWS · LOCAL WEBVIEW2</span>
            <h2 id="local-ai-browser-title">本地 AI 浏览器</h2>
          </div>
        </div>
        <p>
          官方网页在独立 WebView2 Profile 中运行。登录、Cookie 和网页数据只留在本机，
          当前不会上传，也不会转换成 CLI 凭证。
        </p>
        <dl>
          <div><dt>本地所有者</dt><dd>{ownerLabel}</dd></div>
          <div><dt>渲染协议</dt><dd>yilong.ai.ui.v1 · 已预留</dd></div>
        </dl>
      </div>

      <div className={styles.controls}>
        {!desktopAvailable ? (
          <div className={styles.desktopNotice}>
            <MonitorUp size={18} />
            <span>请在一龙 Windows 客户端打开本页；普通浏览器继续使用下方托管模式。</span>
          </div>
        ) : state === 'loading' ? (
          <div className={styles.desktopNotice}>
            <LoaderCircle className={styles.spin} size={18} />
            <span>正在读取桌面端本地厂商注册表…</span>
          </div>
        ) : (
          <>
            <label className={styles.confirmation}>
              <input
                type="checkbox"
                checked={ownerConfirmed}
                onChange={(event) => setOwnerConfirmed(event.target.checked)}
                disabled={!ownerKey || state === 'error'}
              />
              <span>
                <strong>只登录本人账号</strong>
                <small>真人验证由本人完成；一龙不读取密码、Cookie 或访问令牌。</small>
              </span>
            </label>

            <div className={styles.providerList}>
              {providers.map((provider) => {
                const busy = busyProvider === provider.id
                return (
                  <article className={styles.provider} key={provider.id}>
                    <div>
                      <strong>{provider.displayName}</strong>
                      <small>{provider.startHost} · 独立本地 Profile</small>
                    </div>
                    <div className={styles.actions}>
                      <button
                        className={styles.openButton}
                        type="button"
                        onClick={() => void open(provider)}
                        disabled={!ownerKey || !ownerConfirmed || Boolean(busyProvider)}
                      >
                        {busy ? <LoaderCircle className={styles.spin} size={16} /> : <ShieldCheck size={16} />}
                        {busy ? '处理中' : '打开本地会话'}
                      </button>
                      <button
                        className={styles.clearButton}
                        type="button"
                        title="清除当前账号的本地网页会话"
                        aria-label={`清除 ${provider.displayName} 本地网页会话`}
                        onClick={() => void clear(provider)}
                        disabled={openedProvider !== provider.id || Boolean(busyProvider)}
                      >
                        <Trash2 size={16} />
                      </button>
                    </div>
                  </article>
                )
              })}
            </div>
          </>
        )}

        {message && <p className={styles.message} data-error={state === 'error'}>{message}</p>}
      </div>
    </section>
  )
}

function errorMessage(error: unknown): string {
  return error instanceof Error && error.message ? error.message : '本地 AI 浏览器调用失败。'
}
