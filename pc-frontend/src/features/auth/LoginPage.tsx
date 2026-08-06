import { useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'
import { useAuthStore } from '../../store/auth'
import type { ApiError } from '../../api/client'
import { getPcLegacyUrl } from '../shell/pcLegacyUrl'
import GoogleIdentityButton from './GoogleIdentityButton'
import styles from './LoginPage.module.css'

export default function LoginPage() {
  const navigate = useNavigate()
  const [searchParams] = useSearchParams()
  const login = useAuthStore((s) => s.login)
  const register = useAuthStore((s) => s.register)
  const acceptSession = useAuthStore((s) => s.acceptSession)
  const [mode, setMode] = useState<'login' | 'register'>(
    searchParams.get('mode') === 'register' ? 'register' : 'login',
  )
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [passwordConfirm, setPasswordConfirm] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    setError(null)
    if (mode === 'register' && password !== passwordConfirm) {
      setError('两次输入的密码不一致')
      return
    }
    setLoading(true)
    try {
      if (mode === 'register') {
        await register(username, password)
      } else {
        await login(username, password)
      }
      navigate('/', { replace: true })
    } catch (err) {
      setError((err as ApiError).message ?? (mode === 'register' ? '注册失败，请重试' : '登录失败，请重试'))
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className={styles.page}>
      <div className={styles.card}>
        <h1 className={styles.title}>一龙工作台</h1>
        <p className={styles.subtitle}>{mode === 'register' ? '注册新账号' : 'PC 新版（Beta）'}</p>
        <form onSubmit={handleSubmit} className={styles.form}>
          <input
            className={styles.input}
            type="text"
            placeholder="用户名"
            autoComplete="username"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            required
          />
          <input
            className={styles.input}
            type="password"
            placeholder="密码"
            autoComplete={mode === 'register' ? 'new-password' : 'current-password'}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
          {mode === 'register' && (
            <input
              className={styles.input}
              type="password"
              placeholder="确认密码"
              autoComplete="new-password"
              value={passwordConfirm}
              onChange={(e) => setPasswordConfirm(e.target.value)}
              required
            />
          )}
          {error && <p className={styles.error}>{error}</p>}
          <button className={styles.btn} type="submit" disabled={loading}>
            {loading
              ? (mode === 'register' ? '注册中…' : '登录中…')
              : (mode === 'register' ? '注册并登录' : '登录')}
          </button>
        </form>
        {mode === 'login' && (
          <>
            <div className={styles.divider}><span>或</span></div>
            <GoogleIdentityButton
              mode="login"
              onComplete={(result) => {
                if (!result.session) throw new Error('服务端没有创建登录会话')
                acceptSession(result.session.token, result.session.expires_at, result.user)
                navigate('/', { replace: true })
              }}
            />
          </>
        )}
        <p className={styles.hint}>
          {mode === 'register' ? '已有账号？' : '还没有账号？'}
          <button
            className={styles.linkBtn}
            type="button"
            onClick={() => {
              setError(null)
              setPassword('')
              setPasswordConfirm('')
              setMode(mode === 'register' ? 'login' : 'register')
            }}
          >
            {mode === 'register' ? '登录账号' : '注册新账号'}
          </button>
        </p>
        <p className={styles.hint}>
          如需使用旧版 PC，请访问 <a href={getPcLegacyUrl()}>旧版工作台</a>
        </p>
      </div>
    </div>
  )
}
