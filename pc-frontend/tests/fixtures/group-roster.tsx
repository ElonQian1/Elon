import React, { useState } from 'react'
import { createRoot } from 'react-dom/client'
import GroupMembersPanel from '../../src/features/friends/members/GroupMembersPanel'
import '../../src/styles/globals.css'
function Fixture() {
  const [open, setOpen] = useState(true)
  const [group, setGroup] = useState('fixture-group')
  const [notice, setNotice] = useState('')
  return <div style={{ display: 'flex', height: '100dvh', color: 'var(--text)', background: 'var(--bg-base)' }}>
    <main style={{ flex: 1, padding: 24, minWidth: 0 }}><h1>杀蟑螂 · 虚构测试数据</h1><p>当前会话区域</p><button onClick={() => setOpen(true)}>查看群成员</button><button onClick={() => setGroup(group === 'fixture-group' ? 'other-group' : 'fixture-group')}>切换测试群</button><p role="status">{notice}</p></main>
    {open && <GroupMembersPanel key={group} groupId={group} friends={Array.from({ length: 8 }, (_, index) => ({ id: `friend-${index}`, account: `好友${index}`, nickname: `好友${index}` }))} onClose={() => setOpen(false)} onChanged={() => {}} onMention={member => setNotice(`已插入 @${member.display_name}`)} onMessage={member => setNotice(`私聊 ${member.display_name}`)} />}
  </div>
}
createRoot(document.getElementById('root')!).render(<Fixture />)
