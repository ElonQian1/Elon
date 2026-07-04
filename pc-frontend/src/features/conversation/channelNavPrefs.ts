const PINNED_CHANNELS_STORAGE_KEY = 'elon.pc.pinnedChannels.v1'

type PinnedChannelMap = Record<string, string[]>

export function readPinnedChannelIds(projectId: string): string[] {
  if (!projectId || typeof window === 'undefined') return []
  try {
    const parsed = JSON.parse(window.localStorage.getItem(PINNED_CHANNELS_STORAGE_KEY) || '{}') as PinnedChannelMap
    return Array.isArray(parsed[projectId]) ? parsed[projectId] : []
  } catch {
    return []
  }
}

export function writePinnedChannelIds(projectId: string, channelIds: string[]) {
  if (!projectId || typeof window === 'undefined') return
  try {
    const parsed = JSON.parse(window.localStorage.getItem(PINNED_CHANNELS_STORAGE_KEY) || '{}') as PinnedChannelMap
    parsed[projectId] = Array.from(new Set(channelIds)).filter(Boolean)
    window.localStorage.setItem(PINNED_CHANNELS_STORAGE_KEY, JSON.stringify(parsed))
  } catch {
    // Local navigation preferences are best-effort only.
  }
}
