import { create } from 'zustand'
import '../friends/socialReadPreview'
import type { LinkPreview } from '../friends/socialLinks'
import {
  activateTab, clampDockWidth, closeTab, initialReaderTabsState, nextTabId, openTab, setLayout, setPresented, updateTab,
  type ReaderLayout, type ReaderTab, type ReaderTabsState,
} from './readerTabsModel'

const LAYOUT_KEY = 'elon.pc.readerLayout'
const WIDTH_KEY = 'elon.pc.readerDockWidth'

function restored(): ReaderTabsState {
  try {
    const layout = window.localStorage.getItem(LAYOUT_KEY)
    const width = Number(window.localStorage.getItem(WIDTH_KEY))
    return {
      ...initialReaderTabsState,
      layout: layout === 'overlay' ? 'overlay' : 'docked',
      dockWidth: Number.isFinite(width) && width > 0 ? clampDockWidth(width, window.innerWidth) : initialReaderTabsState.dockWidth,
    }
  } catch { return initialReaderTabsState }
}

interface ReaderTabsStore extends ReaderTabsState {
  open(preview: LinkPreview, scope: string): ReaderTab | null
  activate(id: string | null): void
  close(id: string): void
  patch(id: string, patch: Partial<Pick<ReaderTab, 'title' | 'hosted' | 'loading' | 'error' | 'url'>>): void
  setLayout(layout: ReaderLayout): void
  setPresented(presented: boolean): void
  setDockWidth(width: number): void
}

export const useReaderTabs = create<ReaderTabsStore>((set, get) => ({
  ...(typeof window === 'undefined' ? initialReaderTabsState : restored()),
  open(preview, scope) {
    const before = get()
    const url = readingUrlFor(preview)
    const next = openTab(before, { id: nextTabId(), url, originalUrl: preview.url, title: preview.title || preview.site, site: preview.site, preview, scope })
    set(next)
    return next.tabs.find(tab => tab.originalUrl === preview.url) ?? null
  },
  activate: id => set(state => activateTab(state, id)),
  close: id => set(state => closeTab(state, id)),
  patch: (id, patch) => set(state => updateTab(state, id, patch)),
  setLayout(layout) {
    try { window.localStorage.setItem(LAYOUT_KEY, layout) } catch { /* preference only */ }
    set(state => setLayout(state, layout))
  },
  setPresented: presented => set(state => setPresented(state, presented)),
  setDockWidth(width) {
    const dockWidth = clampDockWidth(width, window.innerWidth)
    try { window.localStorage.setItem(WIDTH_KEY, String(dockWidth)) } catch { /* preference only */ }
    set({ dockWidth })
  },
}))

// X posts open the official page (the adapter reads it); other embeds use their player URL.
function readingUrlFor(preview: LinkPreview) {
  const original = ElonSocialReadAdapter.readingUrl(preview.url)
  return preview.embed?.kind === 'x' ? original : preview.embed?.url || original
}
