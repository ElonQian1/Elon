// All native calls for reading tabs; components only describe intent (present here / hide / close).
import { boundsFor, controlInternalBrowserTab, getInternalBrowserTabState, openInternalBrowserTab, resizeInternalBrowserTab, type EmbeddedWebviewBounds } from '../user-browser/internalBrowserApi'
import { applyReadBack } from '../friends/socialReadBack'
import '../friends/socialReadPreview'
import { useReaderTabs } from './readerTabsStore'
import type { ReaderTab } from './readerTabsModel'

const opened = new Set<string>()
const readBackDone = new Set<string>()
let queue: Promise<unknown> = Promise.resolve()

// Native webview commands must not interleave: a late "show" could resurrect a closed tab.
function serialize<T>(work: () => Promise<T>): Promise<T> {
  const result = queue.then(work, work)
  queue = result.then(() => undefined, () => undefined)
  return result
}

function fail(id: string, cause: unknown) {
  const message = cause instanceof Error ? cause.message : String(cause)
  if (/尚未打开|已关闭/.test(message)) {
    // The native side no longer has this tab (for example its pop-out window was closed).
    opened.delete(id); readBackDone.delete(id)
    useReaderTabs.getState().close(id)
    return
  }
  useReaderTabs.getState().patch(id, { error: message })
}

export function presentTab(tab: ReaderTab, viewport: HTMLElement) {
  return serialize(async () => {
    let bounds: EmbeddedWebviewBounds
    try { bounds = boundsFor(viewport) } catch { return }
    try {
      if (!opened.has(tab.id)) {
        await openInternalBrowserTab({ url: tab.url, title: tab.title }, bounds, tab.id)
        opened.add(tab.id)
      } else {
        await controlInternalBrowserTab('show', tab.id, bounds)
      }
      useReaderTabs.getState().patch(tab.id, { error: '' })
    } catch (cause) { fail(tab.id, cause) }
  })
}

export function resizeTab(id: string, viewport: HTMLElement) {
  return serialize(async () => {
    if (!opened.has(id)) return
    try { await resizeInternalBrowserTab(boundsFor(viewport), id) } catch { /* viewport not laid out yet */ }
  })
}

export function hideTab(id: string) {
  return serialize(async () => {
    if (!opened.has(id)) return
    try { await controlInternalBrowserTab('hide', id) } catch (cause) { fail(id, cause) }
  })
}

export function closeTab(id: string) {
  useReaderTabs.getState().close(id)
  return serialize(async () => {
    if (!opened.has(id)) return
    opened.delete(id); readBackDone.delete(id)
    try { await controlInternalBrowserTab('close', id) } catch { /* already gone */ }
  })
}

export function navigateTab(id: string, action: 'back' | 'forward' | 'reload' | 'external') {
  return serialize(async () => {
    if (!opened.has(id)) return
    try { await controlInternalBrowserTab(action, id) } catch (cause) { fail(id, cause) }
  })
}

export function popOutTab(id: string) {
  return serialize(async () => {
    if (!opened.has(id)) return
    try {
      await controlInternalBrowserTab('popout', id)
      useReaderTabs.getState().patch(id, { hosted: 'popout' })
    } catch (cause) { fail(id, cause) }
  })
}

export function dockTab(id: string) {
  return serialize(async () => {
    if (!opened.has(id)) return
    try {
      await controlInternalBrowserTab('dock', id)
      useReaderTabs.getState().patch(id, { hosted: 'main' })
    } catch (cause) { fail(id, cause) }
  })
}

export function focusPopout(id: string) {
  return serialize(async () => {
    if (!opened.has(id)) return
    try { await controlInternalBrowserTab('show', id) } catch (cause) { fail(id, cause) }
  })
}

/** Refresh title/loading/error and, once, feed the original-page read-back to caches and server. */
export async function pollTab(tab: ReaderTab) {
  if (!opened.has(tab.id)) return
  try {
    const wantRead = !readBackDone.has(tab.id) && !!ElonSocialReadAdapter.identity(tab.originalUrl)
    const state = await getInternalBrowserTabState(wantRead ? tab.originalUrl : undefined, tab.id)
    const store = useReaderTabs.getState()
    store.patch(tab.id, { loading: !!state.loading, error: state.lastError || '', ...(state.hosted ? { hosted: state.hosted } : {}), ...(state.title ? { title: state.title } : {}) })
    if (state.readPreview && applyReadBack(tab.scope, tab.preview, state.readPreview)) readBackDone.add(tab.id)
  } catch (cause) { fail(tab.id, cause) }
}
