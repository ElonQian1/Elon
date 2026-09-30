import { useEffect,useRef,useState } from 'react'
import { loginSession,readJson } from '../game-access/session'
import EskComputeCenter from './EskComputeCenter'
import styles from './EskComputeCenter.module.css'

type Session = ReturnType<typeof loginSession>
export default function EskComputePage() {
  const [session,setSession] = useState<Session | null>(null)
  const [account,setAccount] = useState('')
  const [password,setPassword] = useState('')
  const [error,setError] = useState('')
  const [busy,setBusy] = useState(false)
  const pending = useRef<AbortController | null>(null)
  useEffect(() => () => pending.current?.abort(),[])
  useEffect(() => {
    if (!session) return
    const timer = window.setTimeout(() => setSession(null),Math.max(0,Date.parse(session.expires_at)-Date.now()))
    return () => window.clearTimeout(timer)
  },[session])
  async function login(event: React.FormEvent) {
    event.preventDefault()
    if (busy || pending.current || location.protocol !== 'https:') return
    const controller = new AbortController(); pending.current = controller
    const timer = window.setTimeout(() => controller.abort(),15000)
    setBusy(true); setError('')
    try {
      const response = await fetch('/api/esk-compute-center/login', {
        method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({account,password}),
        signal:controller.signal,credentials:'omit',cache:'no-store',redirect:'error',referrerPolicy:'no-referrer',
      })
      if (!response.ok) throw new Error(response.status === 429 ? '尝试次数较多，请一分钟后重试。' : '登录未成功，请检查账号和密码。')
      const result = loginSession(await readJson(response),Date.now())
      if (!controller.signal.aborted) setSession(result)
    } catch (reason) { if (pending.current === controller) setError(reason instanceof Error ? reason.message : '登录未成功，请重试。') }
    finally { window.clearTimeout(timer); setPassword(''); setBusy(false); pending.current = null }
  }
  return <main className={styles.page}>
    {session ? <>
      <div className={styles.session}><span>当前账户：{session.user.nickname || session.user.account}</span>
        <button type="button" onClick={() => { setSession(null); setAccount(''); setPassword('') }}>结束本次查看</button></div>
      <EskComputeCenter session={{token:session.token,userId:session.user.id}} />
    </> : <section className={styles.center}>
      <h1>ESK 与算力</h1><p>使用一龙主账号登录，查看正式登记、参考估值、AI 用量与实际账单。本页仅在内存保存本次会话。</p>
      <form onSubmit={login} className={styles.login}>
        <label>一龙账号<input value={account} onChange={e => setAccount(e.target.value)} autoComplete="username" required maxLength={254} /></label>
        <label>密码<input type="password" value={password} onChange={e => setPassword(e.target.value)} autoComplete="current-password" required maxLength={1024} /></label>
        <button disabled={busy || location.protocol !== 'https:'}>{busy ? '正在登录…' : '登录并查看账户'}</button>
      </form>
      {location.protocol !== 'https:' && <p role="alert">请从主项目的 HTTPS 安全账户入口打开此页。</p>}
      {error && <p role="alert">{error}</p>}
    </section>}
  </main>
}
