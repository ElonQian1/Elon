import { useEffect, useRef, useState } from 'react'
import { api, type ApiError } from '../../../api/client'
import { startSocialRefresh } from '../socialRefreshLoop'
import { memberError, type Roster } from './rosterTypes'

export function useGroupRoster(group: string, query: string, filter: string) {
  const [data, setData] = useState<Roster | null>(null)
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)
  const [moreBusy, setMoreBusy] = useState(false)
  const [notice, setNotice] = useState('')
  const current = useRef<Roster | null>(null)
  const loop = useRef<ReturnType<typeof startSocialRefresh> | null>(null)
  const generation = useRef(0)
  const nextRead = useRef<AbortController | null>(null)
  const path = `/api/me/groups/${encodeURIComponent(group)}/roster?q=${encodeURIComponent(query.trim())}&filter=${filter}`
  useEffect(() => {
    const serial = ++generation.current
    nextRead.current?.abort(); setMoreBusy(false)
    current.current = null; setData(null); setError(''); setNotice(''); setLoading(true)
    const start = setTimeout(() => {
      loop.current = startSocialRefresh(async signal => {
        try {
          const page = await api.get<Roster>(path, { signal, cache: 'no-store' })
          if (signal.aborted || serial !== generation.current) return
          if (!Array.isArray(page.members)) throw new Error('成员名单格式异常')
          const previous = current.current
          if (!previous || previous.revision !== page.revision) {
            if (previous) { nextRead.current?.abort(); setMoreBusy(false); setNotice('成员名单已更新') }
            current.current = page; setData(page)
          } else {
            current.current = { ...previous, pending_count: page.pending_count }
            setData(current.current)
          }
          setError('')
        } catch (reason) {
          if (signal.aborted || serial !== generation.current) return
          if ([401, 403, 404].includes((reason as ApiError).status)) { current.current = null; setData(null) }
          setError(memberError(reason))
        } finally { if (!signal.aborted && serial === generation.current) setLoading(false) }
      }, { interval: 15000 })
    }, query ? 250 : 0)
    return () => { generation.current++; clearTimeout(start); loop.current?.stop(); loop.current = null; nextRead.current?.abort() }
  }, [path, query])
  async function more() {
    if (!current.current?.next_cursor || moreBusy) return
    const previous = current.current
    const serial = generation.current
    const controller = new AbortController(); nextRead.current = controller; setMoreBusy(true)
    try {
      const page = await api.get<Roster>(`${path}&cursor=${encodeURIComponent(previous.next_cursor!)}`, { signal: controller.signal, cache: 'no-store' })
      if (controller.signal.aborted || serial !== generation.current || current.current?.revision !== page.revision) return
      current.current = { ...page, members: [...new Map([...previous.members, ...page.members].map(person => [person.id, person])).values()] }
      setData(current.current); setError('')
    } catch (reason) {
      if (!controller.signal.aborted && serial === generation.current) {
        setError(memberError(reason))
        if ((reason as ApiError).status === 409) void loop.current?.refresh()
      }
    } finally { if (serial === generation.current) setMoreBusy(false) }
  }
  return { data, error, loading, moreBusy, notice, more, refresh: () => { void loop.current?.refresh() } }
}
