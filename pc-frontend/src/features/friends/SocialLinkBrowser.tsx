import { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { boundsFor, controlInternalBrowserTab, openInternalBrowserTab, resizeInternalBrowserTab } from '../user-browser/internalBrowserApi'
import type { LinkPreview } from './socialLinks'

// Serialize lifecycle commands so a delayed open cannot resurrect a closed reading window.
let operations = Promise.resolve()
function schedule(work: () => Promise<unknown>) {
  operations = operations.then(work, work).then(() => undefined, () => undefined)
  return operations
}
export default function SocialLinkBrowser({ preview, onClose }: { preview: LinkPreview; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null), surface = useRef<HTMLDivElement>(null)
  const [notice, setNotice] = useState('正在打开…')
  const close = useRef(onClose); close.current = onClose
  useEffect(() => {
    const focus = document.activeElement as HTMLElement | null
    dialog.current?.showModal()
    let stopped = false
    schedule(async () => {
      if (stopped || !surface.current) return
      try {
        await openInternalBrowserTab({ url: preview.embed?.url || preview.url, title: preview.title || preview.site }, boundsFor(surface.current))
        if (!stopped) setNotice('内容由原平台提供；无法加载时可打开原文。')
      } catch { if (!stopped) setNotice('页面未能打开，请重试或使用系统浏览器打开原文。') }
    })
    const observer = new ResizeObserver(() => {
      schedule(async () => { if (!stopped && surface.current) await resizeInternalBrowserTab(boundsFor(surface.current)) })
    })
    if (surface.current) observer.observe(surface.current)
    return () => {
      stopped = true; observer.disconnect()
      schedule(() => controlInternalBrowserTab('close'))
      if (focus?.isConnected) focus.focus({ preventScroll: true })
    }
  }, [preview])
  const control = (action: 'back' | 'forward' | 'reload') => schedule(async () => {
    try { await controlInternalBrowserTab(action); setNotice('内容由原平台提供；无法加载时可打开原文。') }
    catch { setNotice('操作失败，可使用系统浏览器打开原文。') }
  })
  return createPortal(<dialog ref={dialog} className="social-link-dialog" aria-label={preview.title || preview.site}
    onCancel={event => { event.preventDefault(); close.current() }}>
    <div className="social-link-toolbar">
      <button type="button" onClick={() => control('back')}>后退</button><button type="button" onClick={() => control('forward')}>前进</button>
      <strong>{preview.title || preview.site}</strong><button type="button" onClick={() => control('reload')}>刷新</button>
      <a href={preview.url} target="_blank" rel="noopener noreferrer">打开原文</a>
      <button type="button" onClick={onClose} autoFocus>关闭</button>
    </div>
    <p className="social-link-viewer-note" role="status">{notice}</p><div ref={surface} className="social-link-native-surface" />
  </dialog>, document.body)
}
