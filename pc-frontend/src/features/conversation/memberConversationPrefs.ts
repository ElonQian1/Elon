export interface MemberConversationPrefs {
  pinnedIds: string[]
  archivedIds: string[]
  renamedTitles: Record<string, string>
}

const EMPTY_PREFS: MemberConversationPrefs = {
  pinnedIds: [],
  archivedIds: [],
  renamedTitles: {},
}

const STORAGE_KEY = 'elon.pc.memberConversationPrefs.v1'

export function memberConversationPrefsScope(projectId?: string | null, targetUserId?: string | null): string {
  return `${projectId || 'project'}::${targetUserId || 'member'}`
}

export function readMemberConversationPrefs(
  projectId?: string | null,
  targetUserId?: string | null,
): MemberConversationPrefs {
  if (typeof window === 'undefined') return clonePrefs(EMPTY_PREFS)
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (!raw) return clonePrefs(EMPTY_PREFS)
    const stored = JSON.parse(raw) as Record<string, Partial<MemberConversationPrefs>>
    return normalizePrefs(stored[memberConversationPrefsScope(projectId, targetUserId)])
  } catch {
    return clonePrefs(EMPTY_PREFS)
  }
}

export function writeMemberConversationPrefs(
  projectId: string | null | undefined,
  targetUserId: string | null | undefined,
  prefs: MemberConversationPrefs,
) {
  if (typeof window === 'undefined') return
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    const stored = raw ? JSON.parse(raw) as Record<string, Partial<MemberConversationPrefs>> : {}
    stored[memberConversationPrefsScope(projectId, targetUserId)] = normalizePrefs(prefs)
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(stored))
  } catch {
    // Local preferences are best-effort only; conversation data still comes from the server.
  }
}

export function cleanMemberConversationPrefs(
  prefs: MemberConversationPrefs,
  validIds: string[],
): MemberConversationPrefs {
  const valid = new Set(validIds)
  const renamedTitles: Record<string, string> = {}
  for (const [id, title] of Object.entries(prefs.renamedTitles)) {
    const normalized = title.trim()
    if (valid.has(id) && normalized) renamedTitles[id] = normalized.slice(0, 34)
  }
  return {
    pinnedIds: prefs.pinnedIds.filter((id, index, ids) => valid.has(id) && ids.indexOf(id) === index),
    archivedIds: prefs.archivedIds.filter((id, index, ids) => valid.has(id) && ids.indexOf(id) === index),
    renamedTitles,
  }
}

function normalizePrefs(input?: Partial<MemberConversationPrefs>): MemberConversationPrefs {
  if (!input) return clonePrefs(EMPTY_PREFS)
  const renamedTitles: Record<string, string> = {}
  for (const [id, title] of Object.entries(input.renamedTitles ?? {})) {
    const normalized = String(title || '').trim()
    if (id && normalized) renamedTitles[id] = normalized.slice(0, 34)
  }
  return {
    pinnedIds: uniqueStrings(input.pinnedIds),
    archivedIds: uniqueStrings(input.archivedIds),
    renamedTitles,
  }
}

function uniqueStrings(values?: string[]): string[] {
  if (!Array.isArray(values)) return []
  return values.map((value) => String(value || '').trim()).filter(Boolean)
    .filter((value, index, items) => items.indexOf(value) === index)
}

function clonePrefs(prefs: MemberConversationPrefs): MemberConversationPrefs {
  return {
    pinnedIds: [...prefs.pinnedIds],
    archivedIds: [...prefs.archivedIds],
    renamedTitles: { ...prefs.renamedTitles },
  }
}
