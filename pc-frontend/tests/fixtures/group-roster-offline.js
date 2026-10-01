// Deterministic offline renderer. Enabled only by the local test fixture's query string.
export async function offlineRoster(path, init = {}) {
  const url = new URL(path, location.origin), params = new URLSearchParams(location.search)
  const role = params.get('role') || 'owner', state = params.get('state') || 'directory'
  if (state === 'error') return Response.json({ error: '加载失败，请重试' }, { status: 503 })
  if (state === 'denied') return Response.json({ error: '你已不在这个群聊中' }, { status: 403 })
  if (url.pathname.endsWith('/roster')) {
    const all = Array.from({ length: 172 }, (_, index) => ({ id: `member-${index}`, display_name: `成员${String(index).padStart(4, '0')}`, role: index === 0 ? 'owner' : index === 1 ? 'admin' : 'member', joined_at: '2026-10-01T10:00:00Z' }))
    const rows = all.filter(person => (state !== 'empty') && person.display_name.includes(url.searchParams.get('q') || '') && (url.searchParams.get('filter') !== 'admins' || person.role !== 'member'))
    const offset = Number(url.searchParams.get('cursor') || 0)
    return Response.json({ group_id: 'fixture-group', name: '杀蟑螂 · 离线预览', total_count: 172, matched_count: rows.length, revision: 1,
      viewer_id: role === 'owner' ? 'member-0' : 'member-2', viewer_role: role, invitation_policy: 'members', permissions: { invite: true, manage: role !== 'member', owner: role === 'owner' }, pending_count: 0,
      members: rows.slice(offset, offset + 50), next_cursor: offset + 50 < rows.length ? String(offset + 50) : null })
  }
  if (url.pathname.endsWith('/friends')) return Response.json({ friends: [{ id: 'friend-1', nickname: '待邀请好友' }] })
  if (url.pathname.endsWith('/invitations')) return Response.json({ requests: [], total_count: 0, next_offset: null })
  if (init.method === 'POST') return Response.json({ ok: true, exited: false, message: '离线预览操作已完成' })
  return Response.json({ error: 'Offline fixture has no such endpoint' }, { status: 404 })
}
