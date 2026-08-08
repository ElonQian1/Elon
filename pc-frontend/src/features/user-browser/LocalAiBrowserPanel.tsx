import { useState } from 'react'
import { AlertTriangle, Download, LoaderCircle, MonitorUp, RefreshCw, ShieldCheck, Trash2 } from 'lucide-react'
import { WIN_CLIENT_DOWNLOAD_URL } from '../node/launchWinClient'
import {
  clearLocalAiWebSession,
  openLocalAiWebSession,
  type LocalAiWebProvider,
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
  const [openedProvider, setOpenedProvider] = useState<string | null>(null)
  const [message, setMessage] = useState('')

  async function open(provider: LocalAiWebProvider) {
    if (!ownerKey || !ownerConfirmed || busyProvider) return
    setBusyProvider(provider.id)
    setMessage('')
    try {
      const session = await openLocalAiWebSession(provider.id, ownerKey)
      setOpenedProvider(provider.id)
      setMessage(session.status === 'created'
        ? `已打开 ${provider.displayName} 官方页面，请本人完成登录后开始聊天。`
        : `已回到 ${provider.displayName}，可以继续聊天。`)
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
            <span>CHATGPT.COM · 本机登录</span>
            <h2 id="local-ai-browser-title">登录本人 ChatGPT</h2>
          </div>
        </div>
        <p>
          一龙会打开 ChatGPT 官方页面。由本人输入账号并完成人机验证；登录、Cookie 和
          网页数据只留在本机，不会上传，也不会转换成 CLI 凭证。
        </p>
        <dl>
          <div><dt>一龙账号</dt><dd>{ownerLabel}</dd></div>
          <div><dt>登录位置</dt><dd>仅当前电脑</dd></div>
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
            <span>正在确认当前 Win 客户端是否支持 ChatGPT…</span>
          </div>
        ) : state === 'upgrade_required' ? (
          <div className={styles.upgradeNotice}>
            <AlertTriangle size={20} aria-hidden="true" />
            <div>
              <strong>当前 Win 客户端需要更新</strong>
              <p>{capabilityMessage}</p>
              <div className={styles.noticeActions}>
                <a href={WIN_CLIENT_DOWNLOAD_URL} target="_blank" rel="noopener noreferrer">
                  <Download size={15} />下载新版 Win 客户端
                </a>
                <button type="button" onClick={() => void refresh()}>
                  <RefreshCw size={15} />更新后重试
                </button>
              </div>
            </div>
          </div>
        ) : state === 'error' ? (
          <div className={styles.upgradeNotice}>
            <AlertTriangle size={20} aria-hidden="true" />
            <div>
              <strong>本地 ChatGPT 功能暂不可用</strong>
              <p>{capabilityMessage}</p>
              <div className={styles.noticeActions}>
                <button type="button" onClick={() => void refresh()}>
                  <RefreshCw size={15} />重新检查
                </button>
              </div>
            </div>
          </div>
        ) : (
          <>
            <ol className={styles.steps} aria-label="ChatGPT 登录与使用步骤">
              <li><span>1</span><strong>确认只登录本人账号</strong></li>
              <li><span>2</span><strong>打开官方页面并完成登录</strong></li>
              <li><span>3</span><strong>在打开的窗口直接聊天</strong></li>
            </ol>
            <label className={styles.confirmation}>
              <input
                type="checkbox"
                checked={ownerConfirmed}
                onChange={(event) => setOwnerConfirmed(event.target.checked)}
                disabled={!ownerKey}
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
                        {busy ? '处理中' : '登录或打开 ChatGPT'}
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

        {message && <p className={styles.message}>{message}</p>}
      </div>
    </section>
  )
}

function errorMessage(error: unknown): string {
  return error instanceof Error && error.message ? error.message : '本地 AI 浏览器调用失败。'
}
