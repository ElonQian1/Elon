import type { LocalAiConversationSnapshot } from './localAiBrowserApi'

export interface LocalAiDirectoryRow {
  id: string
  title: string
  path: string
  active: boolean
  kind: 'conversation' | 'project'
  pinned?: boolean | null
  pinnedAt?: number | null
  pinOrder?: number | null
  projectId?: string | null
  updatedAt?: number | null
}

/** Provider-neutral projection: UI never interprets DOM labels or API field names. */
export function localAiDirectoryModel(snapshot: LocalAiConversationSnapshot | null | undefined, query = '') {
  const needle = query.trim().toLocaleLowerCase()
  const matches = (item: LocalAiDirectoryRow) => !needle || item.title.toLocaleLowerCase().includes(needle)
  const unique = new Map<string, LocalAiDirectoryRow>()
  for (const item of snapshot?.conversations ?? []) unique.set(`conversation:${item.id}`, { ...item, kind: 'conversation' })
  for (const item of snapshot?.projects ?? []) unique.set(`project:${item.id}`, { ...item, kind: 'project' })
  const rows = [...unique.values()].filter(matches)
  const pinned = rows.filter(item => item.pinned === true).sort((a, b) =>
    (b.pinnedAt ?? 0) - (a.pinnedAt ?? 0) || (a.pinOrder ?? 1000) - (b.pinOrder ?? 1000))
  const projects = rows.filter(item => item.kind === 'project' && item.pinned !== true)
  const recent = rows.filter(item => item.kind === 'conversation' && item.pinned !== true
    && (!item.projectId || needle)).sort((a, b) => (b.updatedAt ?? 0) - (a.updatedAt ?? 0))
  const children = (projectId: string) => rows.filter(item => item.kind === 'conversation' && item.projectId === projectId)
  return { pinned, projects, recent, children }
}
