import { useEffect, useState } from 'react'
import type { Friend } from '../socialMessageTypes'
import SocialAvatar from '../SocialAvatar'
import SocialDialog from '../SocialDialog'
import MemberActionDialog, { type Confirmation } from './MemberActionDialog'
import MemberInviteDialog from './MemberInviteDialog'
import MemberProfile from './MemberProfile'
import MemberRequests from './MemberRequests'
import { roleName, type Member } from './rosterTypes'
import { useGroupRoster } from './useGroupRoster'
import styles from './Members.module.css'

interface Props { groupId: string; friends: Friend[]; onClose: () => void; onChanged: () => void; onMention: (member: Member) => void; onMessage: (member: Member) => void }
export default function GroupMembersPanel(props: Props) {
  const [narrow, setNarrow] = useState(() => window.matchMedia('(max-width: 1100px)').matches)
  useEffect(() => {
    const media = window.matchMedia('(max-width: 1100px)')
    const change = () => setNarrow(media.matches)
    media.addEventListener('change', change); return () => media.removeEventListener('change', change)
  }, [])
  return narrow ? <SocialDialog title="群成员" onClose={props.onClose}><MemberDirectory {...props} /></SocialDialog>
    : <aside className={styles.panel} aria-label="当前群成员"><MemberDirectory {...props} /></aside>
}

function MemberDirectory({ groupId, friends, onClose, onChanged, onMention, onMessage }: Props) {
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState('all')
  const roster = useGroupRoster(groupId, query, filter)
  const [profile, setProfile] = useState<Member | null>(null)
  const [invite, setInvite] = useState(false)
  const [manage, setManage] = useState(false)
  const [requests, setRequests] = useState(false)
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null)
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [message, setMessage] = useState('')
  const data = roster.data
  useEffect(() => { setSelected(new Set()) }, [query, filter, data?.revision])
  function confirm(value: Confirmation) { setConfirmation(value); setProfile(null); setInvite(false) }
  return <>
    <header className={styles.header}><div><strong>群成员{data ? ` · ${data.total_count}` : ''}</strong><small>{data?.name || '当前群聊'}</small></div><button type="button" onClick={onClose} aria-label="收起群成员">收起</button></header>
    <div className={styles.toolbar}>
      <label className={styles.search}>搜索群成员<input type="search" value={query} onChange={event => setQuery(event.target.value)} placeholder="搜索全群昵称" /></label>
      <div className={styles.filters} role="group" aria-label="成员筛选">{[['all', '全部'], ['admins', '群主与管理员'], ['recent', '最近加入']].map(([key, label]) => <button type="button" key={key} aria-pressed={filter === key} onClick={() => setFilter(key)}>{label}</button>)}</div>
      <div className={styles.actions}>{data?.permissions.invite && <button type="button" onClick={() => setInvite(true)}>邀请成员</button>}
        {data?.permissions.manage && <button type="button" onClick={() => { setManage(!manage); setSelected(new Set()) }}>{manage ? '完成管理' : '管理成员'}</button>}
        <button type="button" onClick={roster.refresh}>刷新</button></div>
    </div>
    {(message || roster.notice) && <p className={styles.hint} role="status">{message || roster.notice}</p>}
    {roster.error && <p className={styles.error} role="alert">{roster.error}<button type="button" onClick={roster.refresh}>重新加载</button></p>}
    <div className={styles.scroll}>
      {roster.loading && <p role="status">正在加载群成员…</p>}
      {data && <>
        {(query || filter !== 'all') && <p className={styles.hint}>找到 {data.matched_count} 人 · 全群 {data.total_count} 人</p>}
        {!data.members.length && <p>{query || filter !== 'all' ? '未找到匹配的群成员' : '暂无群成员'}</p>}
        <ul className={styles.list} aria-label="群成员名单">{data.members.map(member => {
          const removable = member.id !== data.viewer_id && member.role !== 'owner' && (data.permissions.owner || (data.permissions.manage && member.role === 'member'))
          return <li key={member.id} className={styles.row}>
            {manage && data.permissions.manage && removable && <input aria-label={`选择${member.display_name}`} type="checkbox" checked={selected.has(member.id)} disabled={!selected.has(member.id) && selected.size >= 100} onChange={event => setSelected(previous => {
              const next = new Set(previous); if (event.target.checked) next.add(member.id); else next.delete(member.id); return next
            })} />}
            <button type="button" className={styles.person} onClick={() => setProfile(member)}>
              <span className={styles.avatar}><SocialAvatar userId={member.id} name={member.display_name} avatar={member.avatar_data_url} /></span>
              <span className={styles.copy}><span>{member.display_name}{member.id === data.viewer_id ? '（我）' : ''}</span><small>{member.role !== 'member' ? roleName(member.role) : `入群 ${member.joined_at.slice(0, 10)}`}</small></span>
            </button>
          </li>
        })}</ul>
        {data.next_cursor && <button className={styles.more} type="button" disabled={roster.moreBusy} onClick={() => void roster.more()}>{roster.moreBusy ? '正在加载…' : `加载更多（已显示 ${data.members.length}/${data.matched_count}）`}</button>}
        {data.permissions.manage && <details className={styles.settings} open={requests} onToggle={event => setRequests(event.currentTarget.open)}><summary>待审邀请（{data.pending_count}）</summary>
          {requests && <MemberRequests key={`${data.revision}:${data.pending_count}:${message}`} groupId={groupId} onConfirm={confirm} />}</details>}
        {data.permissions.owner && <details className={styles.settings}><summary>成员邀请规则</summary>
          <label>谁可以邀请<select value={data.invitation_policy} onChange={event => confirm({ title: '更新邀请规则', explanation: `邀请规则将改为“${event.target.selectedOptions[0].text}”。已在群中的成员保持不变。`, command: { action: 'policy', invitation_policy: event.target.value } })}>
            <option value="members">所有成员可邀请好友</option><option value="admins">仅群主和管理员可邀请</option><option value="approval">普通成员邀请需审核</option>
          </select></label>
        </details>}
      </>}
    </div>
    {data && <footer className={styles.footer}>
      {manage && data.permissions.manage && <button type="button" className={styles.danger} disabled={!selected.size} onClick={() => confirm({ title: '移出所选成员', explanation: `将所选 ${selected.size} 人移出 ${data.name}：${data.members.filter(member => selected.has(member.id)).map(member => member.display_name).join('、')}。`, command: { action: 'remove', user_ids: [...selected] } })}>移出所选成员（{selected.size}）</button>}
      <button type="button" className={styles.danger} onClick={() => confirm({ title: '退出群聊', explanation: data.viewer_role === 'owner' && data.total_count > 1 ? '你是群主，请先从成员资料中转让群主，再退出。' : `退出 ${data.name} 后将不再接收群消息。`, command: { action: 'leave' } })}>退出群聊</button>
      {data.permissions.owner && <button type="button" className={styles.danger} onClick={() => confirm({ title: '解散群聊', explanation: `解散 ${data.name}，全部 ${data.total_count} 位成员将失去群聊访问权限。此操作无法撤销。`, command: { action: 'dissolve' } })}>解散群聊</button>}
    </footer>}
    {profile && data && <MemberProfile member={profile} roster={data} onClose={() => setProfile(null)} onConfirm={confirm} onMention={member => { setProfile(null); onMention(member) }} onMessage={member => { setProfile(null); onMessage(member) }} />}
    {invite && data && <MemberInviteDialog friends={friends} approval={data.viewer_role === 'member' && data.invitation_policy === 'approval'} onClose={() => setInvite(false)} onConfirm={confirm} />}
    {confirmation && <MemberActionDialog groupId={groupId} confirmation={confirmation} onClose={() => setConfirmation(null)} onDone={receipt => {
      setConfirmation(null); setSelected(new Set()); setMessage(receipt.message); roster.refresh(); onChanged(); if (receipt.exited) onClose()
    }} />}
  </>
}
