import { useState } from 'react'
import type { ApiError } from '../../api/client'
import { accountSecurityApi } from '../account/accountSecurityApi'
import styles from './PasswordRecoveryPanel.module.css'

export default function PasswordRecoveryPanel({ initialAccount }: { initialAccount: string }) {
  const [open, setOpen] = useState(false)
  const [account, setAccount] = useState(initialAccount)
  const [code, setCode] = useState('')
  const [password, setPassword] = useState('')
  const [confirm, setConfirm] = useState('')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')

  async function recover(event: React.FormEvent) {
    event.preventDefault()
    setError(''); setMessage('')
    if (password.length < 6) return setError('新密码至少需要 6 个字符')
    if (password !== confirm) return setError('两次输入的新密码不一致')
    setBusy(true)
    try {
      await accountSecurityApi.recoverPassword(account, code, password)
      setCode(''); setPassword(''); setConfirm('')
      setMessage('密码已重置，所有旧设备会话已退出。请使用新密码登录。')
    } catch (reason) {
      setError((reason as ApiError).message || '恢复失败')
    } finally {
      setBusy(false)
    }
  }

  async function requestExternalRecovery() {
    if (!account.trim()) return setError('请先输入账号')
    setBusy(true); setError(''); setMessage('')
    try {
      const response = await accountSecurityApi.startExternalRecovery(account)
      setMessage(response.message)
    } catch (reason) {
      setError((reason as ApiError).message || '恢复服务不可用')
    } finally {
      setBusy(false)
    }
  }

  if (!open) {
    return <button type="button" className={styles.open} onClick={() => { setAccount(initialAccount); setOpen(true) }}>无法登录？使用恢复码</button>
  }
  return (
    <div className={styles.panel}>
      <div className={styles.heading}><strong>恢复账号密码</strong><button type="button" onClick={() => setOpen(false)}>关闭</button></div>
      <p>输入此前离线保存的恢复码。恢复成功后，所有旧设备会话都会失效。</p>
      <form onSubmit={recover}>
        <input value={account} onChange={(event) => setAccount(event.target.value)} placeholder="一龙账号" autoComplete="username" required />
        <input value={code} onChange={(event) => setCode(event.target.value)} placeholder="ELON-… 恢复码" autoComplete="one-time-code" required />
        <input type="password" value={password} onChange={(event) => setPassword(event.target.value)} placeholder="新密码" autoComplete="new-password" required />
        <input type="password" value={confirm} onChange={(event) => setConfirm(event.target.value)} placeholder="确认新密码" autoComplete="new-password" required />
        <button type="submit" disabled={busy}>{busy ? '处理中…' : '使用恢复码重置'}</button>
      </form>
      <button type="button" className={styles.external} onClick={requestExternalRecovery} disabled={busy}>没有恢复码？检查邮件/短信恢复</button>
      {message && <p className={styles.message}>{message}</p>}
      {error && <p className={styles.error}>{error}</p>}
    </div>
  )
}
