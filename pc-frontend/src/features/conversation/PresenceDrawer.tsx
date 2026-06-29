import { useState, useEffect } from 'react'
import { api } from '../../api/client'
import type { UserPresenceSettings } from './types'
import { PRESENCE_OPTIONS } from './memberUtils'
import styles from './ConversationPage.module.css'

export function PresenceDrawer({
  onClose,
  onSaved,
}: {
  onClose: () => void
  onSaved: () => Promise<void>
}) {
  const [status, setStatus] = useState('online')
  const [customStatus, setCustomStatus] = useState('')
  const [activity, setActivity] = useState('')
  const [message, setMessage] = useState('')
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    setMessage('读取中…')
    api.get<UserPresenceSettings>('/api/me/presence')
      .then((data) => {
        setStatus(data.status || 'online')
        setCustomStatus(data.custom_status ?? '')
        setActivity(data.activity ?? '')
        setMessage('')
      })
      .catch((err: { message?: string }) => setMessage(err.message ?? '状态读取失败'))
  }, [])

  async function save() {
    setSaving(true)
    setMessage('保存中…')
    try {
      const data = await api.patch<UserPresenceSettings>('/api/me/presence', {
        status,
        custom_status: customStatus,
        activity,
      })
      setStatus(data.status || status)
      setCustomStatus(data.custom_status ?? '')
      setActivity(data.activity ?? '')
      setMessage('已保存')
      await onSaved()
    } catch (err) {
      setMessage((err as { message?: string }).message ?? '保存失败')
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className={styles.drawerBackdrop}>
      <section className={[styles.permissionDrawer, styles.compactDrawer].join(' ')} role="dialog" aria-modal="true">
        <header className={styles.drawerHeader}>
          <div>
            <strong>在线状态</strong>
            <span>{message}</span>
          </div>
          <button className={styles.drawerCloseBtn} onClick={onClose}>关闭</button>
        </header>
        <div className={styles.drawerBody}>
          <label className={styles.field}>
            <span>展示状态</span>
            <select value={status} onChange={(event) => setStatus(event.target.value)}>
              {PRESENCE_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>{option.label}</option>
              ))}
            </select>
          </label>
          <label className={styles.field}>
            <span>自定义状态</span>
            <input value={customStatus} onChange={(event) => setCustomStatus(event.target.value)} maxLength={80} placeholder="例如：写代码中" />
          </label>
          <label className={styles.field}>
            <span>正在做</span>
            <input value={activity} onChange={(event) => setActivity(event.target.value)} maxLength={80} placeholder="例如：调试 PC 网页版" />
          </label>
          <div className={styles.actionRow}>
            <button className={styles.primaryBtn} onClick={save} disabled={saving}>保存</button>
          </div>
        </div>
      </section>
    </div>
  )
}
