import { useEffect } from 'react'
import { useSearchParams } from 'react-router-dom'
import type { ActiveConversation, FriendGroup } from '../socialMessageTypes'

export default function useGroupDiscussionReturn(groups: FriendGroup[], select: (item: ActiveConversation) => void) {
  const [query, setQuery] = useSearchParams()
  const group = query.get('group')
  useEffect(() => {
    if (!group || !groups.some(item => item.id === group)) return
    select({ kind: 'group', id: group })
    setQuery(previous => { const next = new URLSearchParams(previous); next.delete('group'); return next }, { replace: true })
  }, [group, groups, select, setQuery])
}
