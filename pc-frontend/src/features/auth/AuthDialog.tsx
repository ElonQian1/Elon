import { useEffect, useState } from 'react'
import type { ApiError } from '../../api/client'
import { useAuthStore } from '../../store/auth'
import styles from './AuthDialog.module.css'

export type AuthDialogMode = 'login' | 'register'

interface AuthDialogProps {
  open: boolean
  initialMode?: AuthDialogMode
  onClose: () => void
}

export default function AuthDialog({ open, initialMode = 'login', onClose }: AuthDialogProps) {
  const login = useAuthStore((s) => s.login)
  const register = useAuthStore((s) => s.register)
  const [mode, setMode] = useState<AuthDialogMode>(initialMode)
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [passwordConfirm, setPasswordConfirm] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    if (!open) return
    setMode(initialMode)
    setError('')
    setPassword('')
    setPasswordConfirm('')
  }, [initialMode, open])

  useEffect(() => {
    if (!open) return
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [onClose, open])

  if (!open) return null

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    setError('')
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
      setPassword('')
      setPasswordConfirm('')
      onClose()
    } catch (err) {
      setError((err as ApiError).message ?? (mode === 'register' ? '注册失败，请重试' : '登录失败，请重试'))
    } finally {
      setLoading(false)
    }
  }

  function toggleMode() {
    setError('')
    setPassword('')
    setPasswordConfirm('')
    setMode((current) => current === 'register' ? 'login' : 'register')
  }

  return (
    <div
      className={styles.backdrop}
      role="presentation"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose()
      }}
    >
      <form
        className={styles.card}
        role="dialog"
        aria-modal="true"
        aria-labelledby="auth-dialog-title"
        onSubmit={handleSubmit}
      >
        <button
          className={styles.close}
          type="button"
          aria-label="关闭登录框"
          onClick={onClose}
        >
          ×
        </button>
        <h3 id="auth-dialog-title">{mode === 'register' ? '注册新账号' : '登录账号'}</h3>
        <p className={styles.subtitle}>{mode === 'register'
          ? '注册后会自动登录，并同步你的项目、好友和电脑节点。'
          : '登录后即可开始对话，并同步你的项目、好友和电脑节点。'}</p>
        <input
          className={styles.input}
          type="text"
          placeholder="用户名"
          autoComplete="username"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          autoFocus
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
        <button className={styles.submit} type="submit" disabled={loading}>
          {loading
            ? (mode === 'register' ? '注册中...' : '登录中...')
            : (mode === 'register' ? '注册并登录' : '登录')}
        </button>
        <p className={styles.hint}>
          {mode === 'register' ? '已有账号？' : '还没有账号？'}
          <button type="button" onClick={toggleMode}>
            {mode === 'register' ? '登录账号' : '注册新账号'}
          </button>
        </p>
      </form>
    </div>
  )
}
