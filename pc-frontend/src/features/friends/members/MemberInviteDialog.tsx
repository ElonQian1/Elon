import { useState } from 'react'
import type { Friend } from '../socialMessageTypes'
import SocialDialog from '../SocialDialog'
import type { Confirmation } from './MemberActionDialog'
import styles from './Members.module.css'

export default function MemberInviteDialog({ friends, approval, onClose, onConfirm }: {
  friends: Friend[]; approval: boolean; onClose: () => void; onConfirm: (value: Confirmation) => void
}) {
  const [query, setQuery] = useState('')
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const visible = friends.filter(friend => `${friend.nickname || ''} ${friend.account || ''}`.toLocaleLowerCase().includes(query.toLocaleLowerCase()))
  return <SocialDialog title="邀请群成员" onClose={onClose} footer={<button type="button" disabled={!selected.size} onClick={() => onConfirm({
    title: approval ? '提交邀请' : '邀请成员', explanation: `${approval ? '提交审核，邀请' : '邀请'} ${friends.filter(friend => selected.has(friend.id)).map(friend => friend.nickname || friend.account).join('、')} 加入群聊。已在群中的好友不会重复加入。`, command: { action: 'invite', user_ids: [...selected] },
  })}>{approval ? '提交审核' : '邀请'}（{selected.size}）</button>}>
    <label>搜索好友<input type="search" value={query} onChange={event => setQuery(event.target.value)} /></label>
    <p>最多选择 100 位好友。{approval && '本群邀请需要群主或管理员审核。'}</p>
    {!friends.length && <p>暂无好友，请先在好友列表添加好友。</p>}
    {friends.length > 0 && !visible.length && <p>未找到匹配的好友</p>}
    <div className={styles.inviteList}>{visible.map(friend => <label key={friend.id} className={styles.choice}>
      <input type="checkbox" checked={selected.has(friend.id)} disabled={!selected.has(friend.id) && selected.size >= 100} onChange={event => setSelected(previous => {
        const next = new Set(previous); if (event.target.checked) next.add(friend.id); else next.delete(friend.id); return next
      })} />{friend.nickname || friend.account}
    </label>)}</div>
  </SocialDialog>
}
