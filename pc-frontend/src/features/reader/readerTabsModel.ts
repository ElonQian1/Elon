// Pure state transitions for reading tabs; UI and native calls live elsewhere so this is testable.
import type { LinkPreview } from '../friends/socialLinks'

export type ReaderLayout = 'overlay' | 'docked'

export interface ReaderTab {
  id: string
  url: string
  /** The link as it appeared in chat; read-back and caches are keyed by it. */
  originalUrl: string
  title: string
  site: string
  preview: LinkPreview
  /** Server + account scope the read-back cache belongs to. */
  scope: string
  hosted: 'main' | 'popout'
  loading: boolean
  error: string
}

export interface ReaderTabsState {
  tabs: ReaderTab[]
  activeId: string | null
  layout: ReaderLayout
  dockWidth: number
  /** True while the active docked/overlay tab should be presented; false hides it behind chat. */
  presented: boolean
}

export const MIN_DOCK_WIDTH = 360
export const MAX_DOCK_WIDTH_RATIO = 0.7
export const MAX_READER_TABS = 8

export const initialReaderTabsState: ReaderTabsState = { tabs: [], activeId: null, layout: 'docked', dockWidth: 520, presented: false }

let counter = 0
export function nextTabId(now = Date.now()) {
  counter = (counter + 1) % 1000
  return `read-${now.toString(36)}${counter.toString(36)}`
}

export function findByUrl(state: ReaderTabsState, originalUrl: string) {
  return state.tabs.find(tab => tab.originalUrl === originalUrl) ?? null
}

export function openTab(state: ReaderTabsState, tab: Omit<ReaderTab, 'hosted' | 'loading' | 'error'>): ReaderTabsState {
  const existing = findByUrl(state, tab.originalUrl)
  if (existing) return { ...state, activeId: existing.id, presented: true }
  if (state.tabs.length >= MAX_READER_TABS) return state
  const next: ReaderTab = { ...tab, hosted: 'main', loading: true, error: '' }
  return { ...state, tabs: [...state.tabs, next], activeId: next.id, presented: true }
}

export function activateTab(state: ReaderTabsState, id: string | null): ReaderTabsState {
  if (id !== null && !state.tabs.some(tab => tab.id === id)) return state
  return { ...state, activeId: id, presented: id !== null }
}

export function closeTab(state: ReaderTabsState, id: string): ReaderTabsState {
  const index = state.tabs.findIndex(tab => tab.id === id)
  if (index < 0) return state
  const tabs = state.tabs.filter(tab => tab.id !== id)
  let activeId = state.activeId
  if (activeId === id) {
    // Like Chrome: fall back to the right neighbour, then the left one.
    const neighbour = tabs[index] ?? tabs[index - 1] ?? null
    activeId = neighbour ? neighbour.id : null
  }
  return { ...state, tabs, activeId, presented: activeId !== null && state.presented }
}

export function updateTab(state: ReaderTabsState, id: string, patch: Partial<Pick<ReaderTab, 'title' | 'hosted' | 'loading' | 'error' | 'url'>>): ReaderTabsState {
  if (!state.tabs.some(tab => tab.id === id)) return state
  return { ...state, tabs: state.tabs.map(tab => (tab.id === id ? { ...tab, ...patch } : tab)) }
}

export function setLayout(state: ReaderTabsState, layout: ReaderLayout): ReaderTabsState {
  return { ...state, layout, presented: state.activeId !== null }
}

export function setPresented(state: ReaderTabsState, presented: boolean): ReaderTabsState {
  return { ...state, presented: presented && state.activeId !== null }
}

export function clampDockWidth(width: number, viewportWidth: number) {
  const max = Math.max(MIN_DOCK_WIDTH, Math.floor(viewportWidth * MAX_DOCK_WIDTH_RATIO))
  return Math.min(max, Math.max(MIN_DOCK_WIDTH, Math.round(width)))
}

export function badgeFor(site: string) {
  const labels: Record<string, string> = { 微信公众号: '文', 小红书: '红', 抖音: '抖', 哔哩哔哩: 'B', 币安广场: '币', X: 'X' }
  return labels[site] ?? '↗'
}
