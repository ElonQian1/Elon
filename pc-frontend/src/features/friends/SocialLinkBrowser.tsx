import { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { boundsFor, controlInternalBrowserTab, getInternalBrowserTabState, openInternalBrowserTab, resizeInternalBrowserTab } from '../user-browser/internalBrowserApi'
import './socialReadPreview'
import type { LinkPreview } from './socialLinks'

// Serialize lifecycle commands so a delayed open cannot resurrect a closed reading window.
let operations = Promise.resolve()
function schedule(work: () => Promise<unknown>) {
  operations = operations.then(work, work).then(() => undefined, () => undefined)
  return operations
}
export default function SocialLinkBrowser({ preview, onClose, onRead }: { preview: LinkPreview; onClose: () => void; onRead?: (value: unknown) => void }) {
  const dialog = useRef<HTMLDialogElement>(null), surface = useRef<HTMLDivElement>(null)
  const [notice, setNotice] = useState('正在打开…')
  const close = useRef(onClose); close.current = onClose
  const read = useRef(onRead); read.current = onRead
  const flush = useRef<() => Promise<unknown>>(async () => undefined)
  const closing = useRef(false)
  const finish = () => { if (!closing.current) { closing.current = true; schedule(async () => { await flush.current(); close.current() }) } }
  useEffect(() => {
    const focus = document.activeElement as HTMLElement | null
    dialog.current?.showModal()
    let stopped = false
    let timer: ReturnType<typeof setTimeout> | undefined, attempts = 0
    const capture = async (repeat = true) => {
      if (stopped || !ElonSocialReadAdapter.identity(preview.url)) return
      try {
        const state = await getInternalBrowserTabState(preview.url)
        if (!stopped && state.readPreview) read.current?.(state.readPreview)
      } catch { /* Older desktop hosts still support the original reading route. */ }
      if (repeat && !stopped && ++attempts < 12) timer = setTimeout(() => schedule(capture), 1500)
    }
    flush.current = () => capture(false)
    schedule(async () => {
      if (stopped || !surface.current) return
      try {
        const original = ElonSocialReadAdapter.readingUrl(preview.url)
        await openInternalBrowserTab({ url: preview.embed?.kind === 'x' ? original : preview.embed?.url || original, title: preview.title || preview.site }, boundsFor(surface.current))
        if (!stopped) setNotice('内容由原平台提供；无法加载时可打开原文。')
        if (!stopped) timer = setTimeout(() => schedule(capture), 1000)
      } catch { if (!stopped) setNotice('页面未能打开，请重试或使用系统浏览器打开原文。') }
    })
    const observer = new ResizeObserver(() => {
      schedule(async () => { if (!stopped && surface.current) await resizeInternalBrowserTab(boundsFor(surface.current)) })
    })
    if (surface.current) observer.observe(surface.current)
    return () => {
      stopped = true; observer.disconnect(); clearTimeout(timer)
      schedule(() => controlInternalBrowserTab('close'))
      if (focus?.isConnected) focus.focus({ preventScroll: true })
    }
  }, [preview])
  const control = (action: 'back' | 'forward' | 'reload') => schedule(async () => {
    try { await controlInternalBrowserTab(action); setNotice('内容由原平台提供；无法加载时可打开原文。') }
    catch { setNotice('操作失败，可使用系统浏览器打开原文。') }
  })
  return createPortal(<dialog ref={dialog} className="social-link-dialog" aria-label={preview.title || preview.site}
    onCancel={event => { event.preventDefault(); finish() }}>
    <div className="social-link-toolbar">
      <button type="button" onClick={() => control('back')}>后退</button><button type="button" onClick={() => control('forward')}>前进</button>
      <strong>{preview.title || preview.site}</strong><button type="button" onClick={() => control('reload')}>刷新</button>
      <a href={preview.url} target="_blank" rel="noopener noreferrer">打开原文</a>
      {preview.embed?.kind === 'x' && <button type="button" onClick={() => schedule(async () => {
        await controlInternalBrowserTab('close'); close.current(); ElonSocialLinkViewer.open(preview)
      })}>嵌入查看</button>}
      <button type="button" onClick={finish} autoFocus>关闭</button>
    </div>
    <p className="social-link-viewer-note" role="status">{notice}</p><div ref={surface} className="social-link-native-surface" />
  </dialog>, document.body)
}
