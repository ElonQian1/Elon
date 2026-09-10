import { useMemo, useRef, useState } from 'react'
import { Link, useLocation } from 'react-router-dom'
import { useAuthStore } from '../../store/auth'
import { authorizedCallback, parseGameRequest, scopeLabels } from './contract'
import { readJson } from './session'
import styles from './GameAccessPage.module.css'

export default function GameAccessPage() {
  const location = useLocation()
  const token = useAuthStore(s => s.token)
  const user = useAuthStore(s => s.user)
  const [consentFor, setConsentFor] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const inFlight = useRef(false)
  const parsed = useMemo(() => {
    try { return { request: parseGameRequest(location.search), error: '' } }
    catch { return { request: null, error: '授权链接无效，请回到游戏重新选择“登录主账号”。' } }
  }, [location.search])
  const request = parsed.request
  const consentBinding = `${location.search}\0${token || ''}`
  const consent = consentFor === consentBinding
  const login = `/game-login?next=${encodeURIComponent(`/game-access${location.search}`)}`
  const secure = window.location.protocol === 'https:'

  async function authorize() {
    if (!request || !token || !consent || !secure || inFlight.current) return
    const originalToken = token
    inFlight.current = true
    setBusy(true)
    setError('')
    try {
      // Deliberately same-origin: a local node API override must not receive this bearer.
      const response = await fetch('/api/me/game-access/authorize', {
        method: 'POST', redirect: 'error', cache: 'no-store', credentials: 'omit',
        headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${originalToken}` },
        body: JSON.stringify({ ...request, explicit_consent: true, confirmation: '授权此游戏使用我的主账号及所选权限' }),
        signal: AbortSignal.timeout(8000),
      })
      if (!response.ok) {
        throw new Error(response.status === 401 ? '主账号登录已失效，请重新登录后授权。'
          : response.status === 503 ? '游戏授权服务尚未启用或暂时不可用，请稍后重试。'
            : '本次授权未成功，请回到游戏重新发起登录。')
      }
      const body = await readJson(response)
      if (useAuthStore.getState().token !== originalToken) throw new Error('账号发生变化，请重新确认。')
      const target = authorizedCallback(body, request, Date.now())
      window.location.replace(target)
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : '暂时无法授权，请重新尝试。')
      setConsentFor(null)
    } finally { inFlight.current = false; setBusy(false) }
  }

  return <main className={styles.page}>
    <section className={styles.card} aria-labelledby="game-access-title" aria-busy={busy}>
      <p className={styles.eyebrow}>一龙账号 · ESK 游戏</p>
      <h1 id="game-access-title">使用主账号进入你的小岛</h1>
      <p>确认后，游戏将使用你的主账号身份，保存角色和个人小岛。</p>
      {parsed.error && <p role="alert" className={styles.error}>{parsed.error}</p>}
      {!secure && <p role="alert" className={styles.error}>请通过主项目的 HTTPS 地址打开授权页。</p>}
      {request && <>
        <dl className={styles.details}>
          <div><dt>游戏服务</dt><dd>{new URL(request.redirect_uri).origin}</dd></div>
          <div><dt>本次有效期</dt><dd>最多 {Math.ceil(request.expires_in_seconds / 60)} 分钟</dd></div>
          {token && <div><dt>当前主账号</dt><dd>{user?.nickname || user?.account || '已登录账号'}</dd></div>}
        </dl>
        <h2>本次请求的权限</h2>
        <ul className={styles.permissions}>{request.scopes.map(scope => <li key={scope}>{scopeLabels[scope]}</li>)}</ul>
        <p className={styles.hint}>主账号会话失效或撤销这次授权后，游戏会话将失效。钱包连接与资产交易需要分别确认。</p>
        {!token ? <Link className={styles.primary} to={login}>登录主账号后继续</Link> : <>
          <label className={styles.consent}>
            <input type="checkbox" checked={consent} disabled={busy} onChange={e => setConsentFor(e.target.checked ? consentBinding : null)} aria-describedby="game-access-consent-help" />
            <span>允许此游戏使用上面列出的权限</span>
          </label>
          <p id="game-access-consent-help" className={styles.hint}>勾选后可确认授权。</p>
          <button type="button" className={styles.primary} disabled={!consent || busy || !secure} onClick={() => void authorize()}>
            {busy ? '正在授权…' : '确认并返回游戏'}
          </button>
          <Link className={styles.cancel} to={login}>重新登录或切换主账号</Link>
        </>}
      </>}
      {error && <p role="alert" className={styles.error}>{error}</p>}
      <p className={styles.hint}>不想授权，可以关闭此页并回到游戏。</p>
    </section>
  </main>
}
