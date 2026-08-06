import { useCallback, useEffect, useMemo, useState } from 'react'
import { ExternalLink, KeyRound, LogOut, RefreshCw, ShieldCheck } from 'lucide-react'
import { nodeApi } from './localNodeApi'
import type {
  AiProviderAccount,
  AiProviderAccountsResponse,
  AiProviderLoginAttempt,
  AiProviderLoginResponse,
} from './types'
import styles from './AiProviderAccountsCard.module.css'

const ACTIVE_STATES = new Set(['starting', 'waiting_for_user'])

export default function AiProviderAccountsCard({ adminUrl }: { adminUrl: string }) {
  const [catalog, setCatalog] = useState<AiProviderAccountsResponse | null>(null)
  const [attempt, setAttempt] = useState<AiProviderLoginAttempt | null>(null)
  const [busyProvider, setBusyProvider] = useState('')
  const [notice, setNotice] = useState('')
  const [error, setError] = useState('')

  const load = useCallback(async (quiet = false) => {
    if (!quiet) { setNotice('正在读取官方账号状态…'); setError('') }
    try {
      const data = await nodeApi<AiProviderAccountsResponse>(adminUrl, '/api/ai-provider-accounts', {}, 12000)
      setCatalog(data)
      const activeAttempt = data.providers
        .map((provider) => provider.active_login)
        .find((value): value is AiProviderLoginAttempt => !!value && ACTIVE_STATES.has(value.state))
      if (activeAttempt) setAttempt(activeAttempt)
      if (!quiet) setNotice('官方账号状态已刷新。')
    } catch (loadError) {
      if (!quiet) setError((loadError as Error).message)
    }
  }, [adminUrl])

  useEffect(() => { void load(true) }, [load])

  useEffect(() => {
    if (!attempt || !ACTIVE_STATES.has(attempt.state)) return
    const timer = window.setInterval(async () => {
      try {
        const data = await nodeApi<AiProviderLoginResponse>(
          adminUrl,
          `/api/ai-provider-accounts/${attempt.provider_id}/logins/${attempt.login_id}`,
          {},
          8000,
        )
        setAttempt(data.attempt)
        if (!ACTIVE_STATES.has(data.attempt.state)) {
          window.clearInterval(timer)
          setBusyProvider('')
          if (data.attempt.state === 'completed') {
            setNotice(`${providerLabel(data.attempt.provider_id)} 官方登录已完成。`)
            setError('')
          } else {
            setError(data.attempt.error || '官方登录没有完成，请重新发起。')
          }
          await load(true)
        }
      } catch (pollError) {
        window.clearInterval(timer)
        setBusyProvider('')
        setError((pollError as Error).message)
      }
    }, 1600)
    return () => window.clearInterval(timer)
  }, [adminUrl, attempt, load])

  const providers = useMemo(() => catalog?.providers ?? [], [catalog])

  async function startLogin(provider: AiProviderAccount) {
    const flow = provider.id === 'codex_cli' ? 'device_code' : 'agent'
    const requestId = `pc:${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`}`
    const pendingWindow = provider.id === 'codex_cli' ? window.open('', '_blank') : null
    setBusyProvider(provider.id); setNotice('正在启动厂商官方登录…'); setError('')
    try {
      const data = await nodeApi<AiProviderLoginResponse>(
        adminUrl,
        `/api/ai-provider-accounts/${provider.id}/login`,
        { method: 'POST', body: JSON.stringify({ flow, request_id: requestId }) },
        25000,
      )
      setAttempt(data.attempt)
      const officialUrl = safeOfficialUrl(data.attempt.verification_url || data.attempt.auth_url)
      if (pendingWindow && officialUrl) {
        pendingWindow.opener = null
        pendingWindow.location.href = officialUrl
      } else {
        pendingWindow?.close()
      }
      setNotice(loginNotice(provider.id))
    } catch (loginError) {
      pendingWindow?.close()
      setBusyProvider('')
      setError((loginError as Error).message)
    }
  }

  async function cancelLogin() {
    if (!attempt) return
    try {
      await nodeApi(
        adminUrl,
        `/api/ai-provider-accounts/${attempt.provider_id}/logins/${attempt.login_id}/cancel`,
        { method: 'POST' },
      )
      setAttempt({ ...attempt, state: 'canceled' })
      setBusyProvider('')
      setNotice('已取消本次官方登录。')
    } catch (cancelError) {
      setError((cancelError as Error).message)
    }
  }

  async function logout(provider: AiProviderAccount) {
    setBusyProvider(provider.id); setNotice(`正在通过官方协议退出 ${provider.label}…`); setError('')
    try {
      const data = await nodeApi<{ message?: string }>(
        adminUrl,
        `/api/ai-provider-accounts/${provider.id}/logout`,
        { method: 'POST' },
        25000,
      )
      setNotice(data.message || `${provider.label} 已退出。`)
      await load(true)
    } catch (logoutError) {
      setError((logoutError as Error).message)
    } finally {
      setBusyProvider('')
    }
  }

  async function copyUserCode() {
    if (!attempt?.user_code) return
    try {
      await navigator.clipboard.writeText(attempt.user_code)
      setNotice('验证码已复制。')
    } catch {
      setError('浏览器未允许复制，请手动选择验证码。')
    }
  }

  return (
    <section className={styles.card}>
      <header className={styles.header}>
        <div>
          <span>AI 厂商账号</span>
          <h4>官方 CLI 登录中心</h4>
        </div>
        <button type="button" className={styles.iconButton} onClick={() => load()} aria-label="刷新厂商账号">
          <RefreshCw size={15} />
        </button>
      </header>
      <p className={styles.intro}>
        登录由各厂商公开的官方 CLI 流程完成。一龙不接管 OAuth token，也不会把 CLI 登录伪装成网页版聊天接口。
      </p>

      <div className={styles.providerList}>
        {providers.map((provider) => (
          <ProviderRow
            key={provider.id}
            provider={provider}
            busy={busyProvider === provider.id}
            onLogin={() => startLogin(provider)}
            onLogout={() => logout(provider)}
          />
        ))}
        {!catalog && <div className={styles.loading}>正在读取厂商接入能力…</div>}
      </div>

      {attempt && ACTIVE_STATES.has(attempt.state) && (
        <div className={styles.loginBox}>
          <div>
            <strong>{providerLabel(attempt.provider_id)} 正在等待登录</strong>
            <span>{attempt.remote_compatible ? '可在手机打开官方页面完成' : '需要在这台 Win 电脑完成浏览器回调'}</span>
          </div>
          {attempt.user_code && (
            <button type="button" className={styles.code} onClick={copyUserCode} title="复制验证码">
              {attempt.user_code}
            </button>
          )}
          {safeOfficialUrl(attempt.verification_url || attempt.auth_url) && (
            <a
              className={styles.linkButton}
              href={safeOfficialUrl(attempt.verification_url || attempt.auth_url)!}
              target="_blank"
              rel="noreferrer"
            >
              打开官方登录 <ExternalLink size={13} />
            </a>
          )}
          <button type="button" className={styles.cancelButton} onClick={cancelLogin}>取消</button>
        </div>
      )}

      <div className={styles.safetyNote}>
        <ShieldCheck size={15} />
        <span>Codex 可显式进入现有加密保险箱；Gemini、Claude 和 Copilot 凭据继续留在各自官方 CLI 或系统凭据存储。</span>
      </div>
      {notice && <p className={styles.notice}>{notice}</p>}
      {error && <p className={styles.error}>{error}</p>}
    </section>
  )
}

function ProviderRow({
  provider,
  busy,
  onLogin,
  onLogout,
}: {
  provider: AiProviderAccount
  busy: boolean
  onLogin: () => void
  onLogout: () => void
}) {
  const reserved = provider.implementation_state === 'reserved'
  const installed = provider.cli?.installed === true
  const runnable = provider.cli?.runnable === true
  const loggedIn = provider.cli?.logged_in === true
  const active = provider.active_login && ACTIVE_STATES.has(provider.active_login.state)
  const state = reserved ? '接口保留' : !installed ? '未安装' : !runnable ? '不可运行' : loggedIn ? '已登录' : active ? '登录中' : '未登录'
  return (
    <article className={styles.provider} data-reserved={reserved || undefined}>
      <div className={styles.providerMain}>
        <div className={styles.providerIcon}><KeyRound size={15} /></div>
        <div>
          <strong>{provider.label}</strong>
          <span>{reserved ? provider.reason : providerDescription(provider)}</span>
        </div>
      </div>
      <div className={styles.providerAction}>
        <em data-ready={loggedIn || undefined}>{state}</em>
        {!reserved && !loggedIn && (
          <button type="button" onClick={onLogin} disabled={busy || !runnable || !!active}>
            {busy || active ? '等待中…' : '官方登录'}
          </button>
        )}
        {!reserved && loggedIn && provider.logout_supported !== false && (
          <button type="button" onClick={onLogout} disabled={busy} title={`退出 ${provider.label}`}>
            <LogOut size={13} /> 退出
          </button>
        )}
        {!reserved && loggedIn && provider.logout_supported === false && (
          <button type="button" disabled title="该厂商仅公开了 CLI 内交互式退出命令">
            请在 CLI 退出
          </button>
        )}
      </div>
    </article>
  )
}

function providerDescription(provider: AiProviderAccount): string {
  if (provider.id === 'codex_cli') return 'App Server · 设备码/浏览器登录 · 可接 Codex 保险箱'
  if (provider.id === 'gemini_cli') return 'ACP v1 · Google 登录由 Gemini CLI 保管'
  if (provider.id === 'claude_cli') return '官方 auth login/status/logout · Claude Code 自主管理凭据'
  if (provider.id === 'copilot_cli') return '官方 OAuth Web Flow · Windows Credential Manager'
  return provider.protocol
}

function providerLabel(providerId: string): string {
  if (providerId === 'gemini_cli') return 'Gemini CLI'
  if (providerId === 'claude_cli') return 'Claude Code'
  if (providerId === 'copilot_cli') return 'GitHub Copilot CLI'
  return 'Codex CLI'
}

function loginNotice(providerId: string): string {
  if (providerId === 'codex_cli') return '请在官方页面输入验证码；Win 端会继续等待登录结果。'
  if (providerId === 'gemini_cli') return 'Gemini CLI 已发起 Google 官方浏览器登录，请在 Win 端完成。'
  if (providerId === 'claude_cli') return 'Claude Code 已发起 Anthropic 官方浏览器登录，请在 Win 端完成。'
  return 'Copilot CLI 已发起 GitHub 官方浏览器登录，请在 Win 端完成。'
}

function safeOfficialUrl(raw?: string | null): string | null {
  if (!raw) return null
  try {
    const url = new URL(raw)
    return url.protocol === 'https:' ? url.toString() : null
  } catch {
    return null
  }
}
