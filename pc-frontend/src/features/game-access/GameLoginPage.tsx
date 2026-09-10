import { useRef, useState } from 'react'
import { Link, useNavigate, useSearchParams } from 'react-router-dom'
import { useAuthStore } from '../../store/auth'
import { gameLoginReturn } from './contract'
import { loginSession, readJson } from './session'
import styles from './GameAccessPage.module.css'

export default function GameLoginPage() {
  const navigate = useNavigate()
  const [params] = useSearchParams()
  const target = params.getAll('next').length === 1 ? gameLoginReturn(params.get('next')) : null
  const [account, setAccount] = useState('')
  const [password, setPassword] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const inFlight = useRef(false)
  const secure = window.location.protocol === 'https:'

  async function submit(event: React.FormEvent) {
    event.preventDefault()
    if (!target || !secure || inFlight.current) return
    inFlight.current = true
    setBusy(true)
    setError('')
    try {
      // Credentials go only to this main-project origin, never a configurable node API.
      const response = await fetch('/api/game-access/v1/login', {
        method: 'POST', credentials: 'omit', redirect: 'error', cache: 'no-store',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ account, password }), signal: AbortSignal.timeout(10_000),
      })
      if (!response.ok) throw new Error(response.status === 429 ? '尝试次数较多，请一分钟后再试。'
        : response.status === 503 ? '游戏账号服务尚未启用或暂时不可用。' : '登录未成功，请检查账号和密码。')
      const session = loginSession(await readJson(response), Date.now())
      useAuthStore.getState().acceptSession(session.token, session.expires_at, session.user)
      navigate(target, { replace: true })
    } catch (cause) { setError(cause instanceof Error ? cause.message : '登录未成功，请稍后重试。') }
    finally { setPassword(''); setBusy(false); inFlight.current = false }
  }

  return <main className={styles.page}>
    <section className={styles.card} aria-labelledby="game-login-title" aria-busy={busy}>
      <p className={styles.eyebrow}>一龙账号 · ESK 游戏</p>
      <h1 id="game-login-title">登录你的主项目账号</h1>
      <p>使用已有的一龙账号。登录后还需要确认游戏权限。</p>
      {!target && <p role="alert" className={styles.error}>登录链接已失效，请回到游戏重新发起。</p>}
      {!secure && <p role="alert" className={styles.error}>请通过主项目的 HTTPS 地址登录。</p>}
      <form className={styles.form} onSubmit={event => void submit(event)}>
        <label htmlFor="game-main-account">主账号</label>
        <input id="game-main-account" autoComplete="username" maxLength={254} required value={account} onChange={e => setAccount(e.target.value)} disabled={busy} />
        <label htmlFor="game-main-password">密码</label>
        <input id="game-main-password" type="password" autoComplete="current-password" maxLength={1024} required value={password} onChange={e => setPassword(e.target.value)} disabled={busy} />
        {error && <p role="alert" className={styles.error}>{error}</p>}
        <button className={styles.primary} type="submit" disabled={busy || !secure || !target}>{busy ? '正在登录…' : '登录并查看授权'}</button>
      </form>
      {target && <Link className={styles.cancel} to={target}>返回授权页</Link>}
      <p className={styles.hint}>如需注册、找回密码或使用第三方账号，请先在主项目完成账号设置，再回到游戏重新发起登录。</p>
    </section>
  </main>
}
