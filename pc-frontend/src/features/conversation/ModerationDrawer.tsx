import { useState, useMemo } from 'react'
import { api } from '../../api/client'
import type { ProjectMember } from './types'
import { filterMembers, memberInitial, memberModerationSummary } from './memberUtils'
import styles from './ConversationPage.module.css'

export function ModerationDrawer({
  projectId,
  members,
  onClose,
  onSaved,
}: {
  projectId: string
  members: ProjectMember[]
  onClose: () => void
  onSaved: () => Promise<void>
}) {
  const [query, setQuery] = useState('')
  const [message, setMessage] = useState('')
  const visibleMembers = useMemo(() => filterMembers(members, query), [members, query])

  async function moderate(member: ProjectMember, action: 'mute' | 'unmute' | 'ban' | 'unban', durationMinutes?: number) {
    setMessage('提交中…')
    try {
      await api.patch(`/api/projects/${encodeURIComponent(projectId)}/members/${encodeURIComponent(member.user_id)}/moderation`, {
        action,
        duration_minutes: durationMinutes,
        note: 'PC 成员管理页操作',
      })
      setMessage('已更新')
      await onSaved()
    } catch (err) {
      setMessage((err as { message?: string }).message ?? '操作失败')
    }
  }

  return (
    <div className={styles.drawerBackdrop}>
      <section className={[styles.permissionDrawer, styles.moderationDrawer].join(' ')} role="dialog" aria-modal="true">
        <header className={styles.drawerHeader}>
          <div>
            <strong>禁言与封禁</strong>
            <span>{message}</span>
          </div>
          <button className={styles.drawerCloseBtn} onClick={onClose}>关闭</button>
        </header>
        <div className={styles.drawerBody}>
          <input className={styles.drawerSearchInput} value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索成员" />
          <div className={styles.moderationList}>
            {visibleMembers.map((member) => (
              <article key={member.user_id} className={styles.moderationRow}>
                <span className={styles.memberAvatar}>{memberInitial(member)}</span>
                <div className={styles.moderationInfo}>
                  <strong>{member.account || member.user_id}</strong>
                  <span>{memberModerationSummary(member)}</span>
                </div>
                <div className={styles.moderationActions}>
                  <button className={styles.drawerCloseBtn} onClick={() => moderate(member, 'mute', 60)}>禁言1小时</button>
                  <button className={styles.drawerCloseBtn} onClick={() => moderate(member, 'mute', 1440)}>禁言1天</button>
                  <button className={styles.drawerCloseBtn} onClick={() => moderate(member, 'unmute')} disabled={!member.is_muted}>解禁言</button>
                  <button className={styles.dangerBtn} onClick={() => moderate(member, 'ban')}>封禁</button>
                  <button className={styles.drawerCloseBtn} onClick={() => moderate(member, 'unban')} disabled={!member.is_banned}>解封</button>
                </div>
              </article>
            ))}
            {visibleMembers.length === 0 && <p className={styles.sideHint}>没有匹配成员</p>}
          </div>
        </div>
      </section>
    </div>
  )
}
