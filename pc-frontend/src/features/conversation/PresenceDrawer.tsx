import { useState, useEffect, useMemo } from 'react'
import { api } from '../../api/client'
import type { UserPresenceSettings } from './types'
import { PRESENCE_OPTIONS, presenceLabel } from './memberUtils'
import styles from './ConversationPage.module.css'

const PRESENCE_DESCRIPTIONS: Record<string, string> = {
  online: '正常接收消息',
  idle: '暂时离开',
  dnd: '减少打扰',
  invisible: '显示离线',
}

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
  const preview = useMemo(() => {
    const custom = customStatus.trim()
    const doing = activity.trim()
    return {
      label: presenceLabel(status),
      subtitle: doing || custom || PRESENCE_DESCRIPTIONS[status] || '在线',
      custom,
      doing,
    }
  }, [activity, customStatus, status])

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
            <span>{message || `${preview.label} · ${preview.subtitle}`}</span>
          </div>
          <button className={styles.drawerCloseBtn} onClick={onClose}>关闭</button>
        </header>
        <div className={styles.drawerBody}>
          <section className={styles.presencePreview}>
            <div className={styles.presencePreviewAvatar} data-status={status}>我</div>
            <div>
              <strong>{preview.label}</strong>
              <span>{preview.subtitle}</span>
              {(preview.custom || preview.doing) && (
                <p>
                  {preview.custom && <em>{preview.custom}</em>}
                  {preview.doing && <em>{preview.doing}</em>}
                </p>
              )}
            </div>
          </section>

          <section className={styles.drawerSection}>
            <strong className={styles.sectionTitle}>展示状态</strong>
            <div className={styles.presenceOptionGrid}>
              {PRESENCE_OPTIONS.map((option) => (
                <button
                  key={option.value}
                  type="button"
                  data-status={option.value}
                  data-active={status === option.value ? 'true' : undefined}
                  onClick={() => setStatus(option.value)}
                >
                  <span className={styles.presenceOptionDot} data-status={option.value} />
                  <strong>{option.label}</strong>
                  <em>{PRESENCE_DESCRIPTIONS[option.value] ?? option.label}</em>
                </button>
              ))}
            </div>
          </section>

          <section className={styles.drawerSection}>
            <strong className={styles.sectionTitle}>状态文案</strong>
            <label className={styles.field}>
              <span>自定义状态</span>
              <input value={customStatus} onChange={(event) => setCustomStatus(event.target.value)} maxLength={80} placeholder="例如：写代码中" />
            </label>
            <label className={styles.field}>
              <span>正在做</span>
              <input value={activity} onChange={(event) => setActivity(event.target.value)} maxLength={80} placeholder="例如：调试 PC 网页版" />
            </label>
          </section>
          <div className={styles.actionRow}>
            <button className={styles.primaryBtn} onClick={save} disabled={saving}>保存</button>
          </div>
        </div>
      </section>
    </div>
  )
}
