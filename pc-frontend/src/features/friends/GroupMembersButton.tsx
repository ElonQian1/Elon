import { useEffect, useState } from 'react'
import { api } from '../../api/client'
import SocialAvatar from './SocialAvatar'
import SocialDialog from './SocialDialog'
import styles from './GroupMembers.module.css'

interface Member { id: string; display_name: string; avatar_data_url?: string | null }

export default function GroupMembersButton({ groupId, count }: { groupId: string; count: number }) {
  const [open, setOpen] = useState(false)
  return <>
    <button type="button" className={styles.open} onClick={() => setOpen(true)} aria-label={`查看全部群成员（${count}人）`}>
      {count} 位成员 · 查看全部
    </button>
    {open && <GroupMembersDialog key={groupId} groupId={groupId} onClose={() => setOpen(false)} />}
  </>
}

function GroupMembersDialog({ groupId, onClose }: { groupId: string; onClose: () => void }) {
  const [members, setMembers] = useState<Member[]>([])
  const [query, setQuery] = useState('')
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [attempt, setAttempt] = useState(0)
  useEffect(() => {
    const controller = new AbortController()
    setMembers([]); setLoading(true); setError('')
    void api.get<{ members: Member[] }>(`/api/me/groups/${encodeURIComponent(groupId)}/members`, { signal: controller.signal, cache: 'no-store' })
      .then(data => {
        if (controller.signal.aborted) return
        if (!Array.isArray(data.members)) throw new Error('成员名单格式异常')
        // This endpoint also includes virtual AI mention targets; only real memberships belong here.
        setMembers([...new Map(data.members.map(member => [member.id, member])).values()]
          .sort((a, b) => a.display_name.localeCompare(b.display_name, 'zh-CN')))
      }).catch(() => { if (!controller.signal.aborted) setError('无法加载群成员，请重试') })
      .finally(() => { if (!controller.signal.aborted) setLoading(false) })
    return () => controller.abort()
  }, [groupId, attempt])
  const needle = query.trim().toLocaleLowerCase()
  const visible = members.filter(member => member.display_name.toLocaleLowerCase().includes(needle))
  return <SocialDialog title="群成员" onClose={onClose}>
    <label className={styles.search}>搜索群成员<input type="search" value={query} disabled={loading || !!error}
      onChange={event => setQuery(event.target.value)} placeholder="输入昵称" /></label>
    {loading ? <p role="status">正在加载群成员…</p> : error ? <div role="alert">
      <p>{error}</p><button type="button" onClick={() => setAttempt(value => value + 1)}>重新加载</button>
    </div> : <>
      <p role="status">{needle ? `找到 ${visible.length} 人 · ` : ''}共 {members.length} 位成员</p>
      {!members.length ? <p>暂无群成员</p> : !visible.length ? <p>未找到匹配的群成员</p> :
        <ul className={styles.list} aria-label="完整群成员名单">{visible.map(member =>
          <li key={member.id} data-member-id={member.id}>
            <span className={styles.avatar}><SocialAvatar userId={member.id} name={member.display_name} avatar={member.avatar_data_url} /></span>
            <span>{member.display_name}</span>
          </li>)}</ul>}
    </>}
  </SocialDialog>
}
