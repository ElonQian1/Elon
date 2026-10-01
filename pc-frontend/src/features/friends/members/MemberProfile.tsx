import { useState } from 'react'
import { api } from '../../../api/client'
import SocialDialog from '../SocialDialog'
import SocialAvatar from '../SocialAvatar'
import { memberError, roleName, type Member, type Roster } from './rosterTypes'
import type { Confirmation } from './MemberActionDialog'
import styles from './Members.module.css'

export default function MemberProfile({ member, roster, onClose, onMention, onMessage, onConfirm }: {
  member: Member; roster: Roster; onClose: () => void; onMention: (member: Member) => void
  onMessage: (member: Member) => void; onConfirm: (value: Confirmation) => void
}) {
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const self = member.id === roster.viewer_id
  const removable = !self && member.role !== 'owner' && (roster.permissions.owner || (roster.permissions.manage && member.role === 'member'))
  async function message() {
    setBusy(true); setError('')
    try { await api.post('/api/me/friends', { query: member.id, search_type: 'user_id' }); onMessage(member) }
    catch (reason) { setError(memberError(reason)) } finally { setBusy(false) }
  }
  return <SocialDialog title="群成员资料" onClose={onClose} busy={busy}>
    <div className={styles.profile}><span className={styles.avatar}><SocialAvatar userId={member.id} name={member.display_name} avatar={member.avatar_data_url} /></span>
      <strong>{member.display_name}{self ? '（我）' : ''}</strong><span>{roleName(member.role)}</span></div>
    <p>入群时间：{member.joined_at.slice(0, 10)}</p>
    {!self && <div className={styles.actions}><button type="button" onClick={() => onMention(member)}>在群里 @TA</button><button type="button" disabled={busy} onClick={() => void message()}>添加好友并私聊</button></div>}
    {roster.permissions.owner && !self && <div className={styles.actions}>
      <button type="button" onClick={() => onConfirm({ title: member.role === 'admin' ? '取消管理员' : '设为管理员', explanation: `${member.display_name} ${member.role === 'admin' ? '将恢复为普通成员。' : '将获得邀请审核和移除普通成员的权限。'}`, command: { action: 'role', user_ids: [member.id], role: member.role === 'admin' ? 'member' : 'admin' } })}>{member.role === 'admin' ? '取消管理员' : '设为管理员'}</button>
      <button type="button" onClick={() => onConfirm({ title: '转让群主', explanation: `将群主转让给 ${member.display_name}，你将成为普通成员。`, command: { action: 'transfer', user_ids: [member.id] } })}>转让群主</button>
    </div>}
    {removable && <button type="button" className={styles.danger} onClick={() => onConfirm({ title: '移出群聊', explanation: `将 ${member.display_name} 移出 ${roster.name}。`, command: { action: 'remove', user_ids: [member.id] } })}>移出群聊</button>}
    {error && <p role="alert">{error}</p>}
  </SocialDialog>
}
