import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuthStore } from '../../store/auth'
import type { ApiError } from '../../api/client'
import { getPcLegacyUrl } from '../shell/pcLegacyUrl'
import styles from './LoginPage.module.css'

export default function LoginPage() {
  const navigate = useNavigate()
  const login = useAuthStore((s) => s.login)
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    setError(null)
    setLoading(true)
    try {
      await login(username, password)
      navigate('/', { replace: true })
    } catch (err) {
      setError((err as ApiError).message ?? '登录失败，请重试')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className={styles.page}>
      <div className={styles.card}>
        <h1 className={styles.title}>一龙工作台</h1>
        <p className={styles.subtitle}>PC 新版（Beta）</p>
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
            autoComplete="current-password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
          {error && <p className={styles.error}>{error}</p>}
          <button className={styles.btn} type="submit" disabled={loading}>
            {loading ? '登录中…' : '登录'}
          </button>
        </form>
        <p className={styles.hint}>
          如需使用旧版 PC，请访问 <a href={getPcLegacyUrl()}>旧版工作台</a>
        </p>
      </div>
    </div>
  )
}
