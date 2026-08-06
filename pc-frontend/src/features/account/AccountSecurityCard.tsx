import { useCallback, useEffect, useState } from 'react'
import type { ApiError } from '../../api/client'
import { useAuthStore } from '../../store/auth'
import {
  accountSecurityApi,
  type AccountSecuritySnapshot,
  type AccountSession,
} from './accountSecurityApi'
import styles from './AccountSecurityCard.module.css'

export default function AccountSecurityCard() {
  const logout = useAuthStore((state) => state.logout)
  const [snapshot, setSnapshot] = useState<AccountSecuritySnapshot | null>(null)
  const [currentPassword, setCurrentPassword] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [recoveryCodes, setRecoveryCodes] = useState<string[]>([])
  const [busy, setBusy] = useState('')
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')

  const load = useCallback(async () => {
    try {
      setSnapshot(await accountSecurityApi.status())
    } catch (reason) {
      setError(errorMessage(reason, '无法读取账号安全状态'))
    }
  }, [])

  useEffect(() => { void load() }, [load])

  async function savePassword(event: React.FormEvent) {
    event.preventDefault()
    setError(''); setMessage('')
    if (newPassword.length < 6) return setError('新密码至少需要 6 个字符')
    if (newPassword !== confirmPassword) return setError('两次输入的新密码不一致')
    if (snapshot?.password.enabled && !currentPassword) return setError('请输入当前密码')
    setBusy('password')
    try {
      await accountSecurityApi.changePassword(currentPassword, newPassword)
      setCurrentPassword(''); setNewPassword(''); setConfirmPassword('')
      setMessage('密码已更新，其他设备的旧会话已撤销。')
      await load()
    } catch (reason) {
      setError(errorMessage(reason, '密码更新失败'))
    } finally {
      setBusy('')
    }
  }

  async function rotateRecoveryCodes() {
    if (!window.confirm('旧恢复码会立即失效。新恢复码只显示一次，确定继续？')) return
    if (snapshot?.password.enabled && !currentPassword) {
      setError('生成恢复码前请输入当前密码')
      return
    }
    setBusy('recovery'); setError(''); setMessage('')
    try {
      const response = await accountSecurityApi.rotateRecoveryCodes(currentPassword)
      setRecoveryCodes(response.result.codes)
      setCurrentPassword('')
      setMessage(response.result.replayed
        ? '该请求已经处理；为避免泄露，恢复码不会再次显示。请重新生成一组。'
        : '请立即离线保存这些恢复码；服务器只保存摘要。')
      await load()
    } catch (reason) {
      setError(errorMessage(reason, '生成恢复码失败'))
    } finally {
      setBusy('')
    }
  }

  async function revokeSession(session: AccountSession) {
    const label = session.current ? '退出当前设备' : `退出 ${session.device_name || '这个设备'}`
    if (!window.confirm(`${label}？该会话会立即失效。`)) return
    setBusy(session.id); setError(''); setMessage('')
    try {
      const result = await accountSecurityApi.revokeSession(session.id)
      if (result.result.current_session) {
        logout()
        return
      }
      setMessage('设备会话已撤销。')
      await load()
    } catch (reason) {
      setError(errorMessage(reason, '撤销会话失败'))
    } finally {
      setBusy('')
    }
  }

  async function revokeOthers() {
    if (!window.confirm('确定退出除当前设备之外的全部设备？')) return
    setBusy('sessions'); setError(''); setMessage('')
    try {
      const response = await accountSecurityApi.revokeOtherSessions()
      setMessage(`已撤销 ${response.revoked_session_count} 个其他设备会话。`)
      await load()
    } catch (reason) {
      setError(errorMessage(reason, '退出其他设备失败'))
    } finally {
      setBusy('')
    }
  }

  async function copyRecoveryCodes() {
    try {
      await navigator.clipboard.writeText(recoveryCodes.join('\n'))
      setMessage('恢复码已复制，请保存到可信的离线位置。')
    } catch {
      setError('浏览器未允许复制，请手动保存。')
    }
  }

  if (!snapshot) return <div className={styles.card}>正在读取密码和设备会话…</div>

  return (
    <div className={styles.card}>
      <section className={styles.block}>
        <div className={styles.heading}>
          <div>
            <strong>{snapshot.password.enabled ? '修改密码' : '设置密码'}</strong>
            <p>{snapshot.password.enabled
              ? '修改后会保留当前设备，并撤销其他设备的旧会话。'
              : '当前账号只使用联合身份登录；可增加一个独立的一龙密码。'}</p>
          </div>
          <span>{snapshot.password.enabled ? '已启用' : '未设置'}</span>
        </div>
        <form className={styles.form} onSubmit={savePassword}>
          {snapshot.password.enabled && (
            <input type="password" autoComplete="current-password" value={currentPassword}
              onChange={(event) => setCurrentPassword(event.target.value)} placeholder="当前密码" />
          )}
          <input type="password" autoComplete="new-password" value={newPassword}
            onChange={(event) => setNewPassword(event.target.value)} placeholder="新密码（至少 6 位）" />
          <input type="password" autoComplete="new-password" value={confirmPassword}
            onChange={(event) => setConfirmPassword(event.target.value)} placeholder="再次输入新密码" />
          <button type="submit" disabled={busy === 'password'}>{busy === 'password' ? '保存中…' : '保存密码'}</button>
        </form>
      </section>

      <section className={styles.block}>
        <div className={styles.heading}>
          <div><strong>离线恢复码</strong><p>邮件/短信恢复尚未配置；恢复码可在无法使用其他登录方式时重置密码。</p></div>
          <span>{snapshot.recovery.available_code_count} 个可用</span>
        </div>
        <button className={styles.secondary} type="button" onClick={rotateRecoveryCodes} disabled={busy === 'recovery'}>
          {busy === 'recovery' ? '生成中…' : '生成并替换恢复码'}
        </button>
        {recoveryCodes.length > 0 && (
          <div className={styles.codes}>
            <div><strong>仅显示一次</strong><button type="button" onClick={copyRecoveryCodes}>复制全部</button></div>
            <code>{recoveryCodes.join('\n')}</code>
          </div>
        )}
      </section>

      <section className={styles.block}>
        <div className={styles.heading}>
          <div><strong>登录设备</strong><p>这里只显示一龙应用会话，不包含厂商 CLI 或网页会话。</p></div>
          <button type="button" className={styles.link} onClick={revokeOthers} disabled={busy === 'sessions'}>退出其他设备</button>
        </div>
        <div className={styles.sessions}>
          {snapshot.sessions.map((session) => (
            <div className={styles.session} key={session.id}>
              <div>
                <strong>{session.device_name || '未命名设备'}{session.current ? ' · 当前设备' : ''}</strong>
                <span>最近活动 {formatDateTime(session.last_seen_at || session.created_at)}{session.trusted_device ? ' · 已信任' : ''}</span>
              </div>
              <button type="button" onClick={() => revokeSession(session)} disabled={busy === session.id}>
                {session.current ? '退出' : '撤销'}
              </button>
            </div>
          ))}
        </div>
      </section>
      {message && <p className={styles.message}>{message}</p>}
      {error && <p className={styles.error}>{error}</p>}
    </div>
  )
}

function formatDateTime(value?: string | null) {
  if (!value) return '未知'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { dateStyle: 'short', timeStyle: 'short' })
}

function errorMessage(reason: unknown, fallback: string) {
  return (reason as ApiError)?.message || fallback
}
