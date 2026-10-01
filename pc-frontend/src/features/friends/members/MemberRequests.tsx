import { useEffect, useState } from 'react'
import { api } from '../../../api/client'
import { memberError } from './rosterTypes'
import type { Confirmation } from './MemberActionDialog'
interface Invitation { id: string; actor_name: string; members: { display_name: string }[] }
export default function MemberRequests({ groupId, onConfirm }: { groupId: string; onConfirm: (value: Confirmation) => void }) {
  const [data, setData] = useState<{ requests: Invitation[]; total_count: number; next_offset: number | null } | null>(null)
  const [offset, setOffset] = useState(0)
  const [attempt, setAttempt] = useState(0)
  const [error, setError] = useState('')
  useEffect(() => {
    const controller = new AbortController(); setData(null); setError('')
    void api.get<NonNullable<typeof data>>(`/api/me/groups/${encodeURIComponent(groupId)}/invitations?offset=${offset}`, { signal: controller.signal, cache: 'no-store' })
      .then(value => { if (!controller.signal.aborted) setData(value) })
      .catch(reason => { if (!controller.signal.aborted) setError(memberError(reason)) })
    return () => controller.abort()
  }, [groupId, offset, attempt])
  return <section aria-label="待审邀请"><h3>待审邀请{data ? `（${data.total_count}）` : ''}</h3>
    {error ? <p role="alert">{error}<button onClick={() => setAttempt(value => value + 1)}>重试</button></p> : !data ? <p role="status">正在加载…</p> : <>
      {!data.requests.length && <p>暂无待审邀请</p>}{data.requests.map(request => <div key={request.id}>
        <p>{request.actor_name} 邀请 {request.members.map(person => person.display_name).join('、')}</p>
        {(['approve', 'reject'] as const).map(action => <button key={action} type="button" onClick={() => onConfirm({ title: action === 'approve' ? '通过邀请' : '拒绝邀请', explanation: `处理 ${request.actor_name} 的这条邀请。`, command: { action, invitation_id: request.id } })}>{action === 'approve' ? '通过' : '拒绝'}</button>)}
      </div>)}
      {offset > 0 && <button type="button" onClick={() => setOffset(Math.max(0, offset - 50))}>上一页</button>}
      {data.next_offset != null && <button type="button" onClick={() => setOffset(data.next_offset!)}>下一页</button>}
    </>}
  </section>
}
