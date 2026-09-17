import { useEffect, useLayoutEffect, useRef, useState, type MouseEvent as ReactMouseEvent } from 'react'
import { ArrowLeft, ArrowRight, ExternalLink, Minus, PanelRightClose, PictureInPicture2, RefreshCw, X } from 'lucide-react'
import { closeTab, dockTab, focusPopout, hideTab, navigateTab, pollTab, popOutTab, presentTab, resizeTab } from './readerNative'
import { useReaderTabs } from './readerTabsStore'
import type { ReaderTab } from './readerTabsModel'
import styles from './ReaderTabs.module.css'

/** True while any other dialog is open; the native view must yield because it always paints on top. */
function useDialogObscured(own: HTMLElement | null) {
  const [obscured, setObscured] = useState(false)
  useEffect(() => {
    const check = () => setObscured([...document.querySelectorAll('dialog[open]')].some(node => !own?.contains(node)))
    check()
    const observer = new MutationObserver(check)
    observer.observe(document.body, { subtree: true, childList: true, attributes: true, attributeFilter: ['open'] })
    return () => observer.disconnect()
  }, [own])
  return obscured
}

function ReaderPane({ tab, dragging }: { tab: ReaderTab; dragging: boolean }) {
  const viewport = useRef<HTMLDivElement>(null)
  const root = useRef<HTMLDivElement>(null)
  const obscured = useDialogObscured(root.current)
  const layout = useReaderTabs(s => s.layout)
  const setLayout = useReaderTabs(s => s.setLayout)
  const popout = tab.hosted === 'popout'
  const shown = !popout && !dragging && !obscured

  // Present/hide follows visibility; geometry follows the viewport box (size and position).
  useLayoutEffect(() => {
    const node = viewport.current
    if (!shown || !node) { void hideTab(tab.id); return }
    void presentTab(tab, node)
    let last = ''
    const sync = () => {
      const rect = node.getBoundingClientRect()
      const key = [rect.left, rect.top, rect.width, rect.height].map(Math.round).join(',')
      if (key !== last) { last = key; void resizeTab(tab.id, node) }
    }
    const observer = new ResizeObserver(sync)
    observer.observe(node)
    window.addEventListener('resize', sync)
    const timer = window.setInterval(sync, 400)
    return () => { observer.disconnect(); window.removeEventListener('resize', sync); window.clearInterval(timer); void hideTab(tab.id) }
  }, [tab.id, tab.url, shown])

  useEffect(() => {
    let live = true
    const tick = async () => { if (live) await pollTab(tab) }
    void tick()
    const timer = window.setInterval(() => { void tick() }, 700)
    return () => { live = false; window.clearInterval(timer) }
  }, [tab.id])

  return (
    <div ref={root} className={styles.pane}>
      <div className={styles.toolbar}>
        <button type="button" onClick={() => void navigateTab(tab.id, 'back')} title="后退"><ArrowLeft size={14} /></button>
        <button type="button" onClick={() => void navigateTab(tab.id, 'forward')} title="前进"><ArrowRight size={14} /></button>
        <button type="button" onClick={() => void navigateTab(tab.id, 'reload')} title="刷新"><RefreshCw size={14} /></button>
        <strong title={tab.title}>{tab.title}</strong>
        <button type="button" onClick={() => void navigateTab(tab.id, 'external')} title="在系统浏览器打开"><ExternalLink size={14} /></button>
        {popout
          ? <button type="button" onClick={() => void dockTab(tab.id)} title="收回到主窗口"><PanelRightClose size={14} /></button>
          : <button type="button" onClick={() => void popOutTab(tab.id)} title="弹出为独立窗口"><PictureInPicture2 size={14} /></button>}
        <button type="button" onClick={() => useReaderTabs.getState().setPresented(false)} title="收起，回到聊天"><Minus size={14} /></button>
        <button type="button" onClick={() => void closeTab(tab.id)} title="关闭标签"><X size={14} /></button>
      </div>
      <p className={styles.status} role="status" data-error={tab.error ? 'true' : undefined}>
        {tab.error || (tab.loading ? '正在加载…' : `${new URL(tab.url).hostname} · 内容由原平台提供；右键顶部标签可停靠、覆盖或弹出`)}
      </p>
      <div ref={viewport} className={styles.viewport}>
        {popout && <div className={styles.placeholder}>已在独立窗口中阅读。<button type="button" onClick={() => void focusPopout(tab.id)}>切到该窗口</button><button type="button" onClick={() => void dockTab(tab.id)}>收回这里</button></div>}
        {!popout && obscured && <div className={styles.placeholder}>有弹窗打开时阅读内容暂时收起。</div>}
        {!popout && dragging && <div className={styles.placeholder}>调整宽度中…</div>}
      </div>
      {layout === 'overlay' && !popout && <button type="button" className={styles.dockHint} onClick={() => setLayout('docked')}>停靠到聊天右侧，边读边聊</button>}
    </div>
  )
}

export default function ReaderWorkspace() {
  const { tabs, activeId, presented, layout, dockWidth, setDockWidth } = useReaderTabs()
  const [dragging, setDragging] = useState(false)
  const tab = tabs.find(t => t.id === activeId) ?? null
  // Hidden tabs keep loading off-screen; poll them slowly so titles and read-back still arrive.
  useEffect(() => {
    const timer = window.setInterval(() => {
      const state = useReaderTabs.getState()
      state.tabs.filter(t => !(state.presented && t.id === state.activeId)).forEach(t => { void pollTab(t) })
    }, 3000)
    return () => window.clearInterval(timer)
  }, [])
  const startDrag = (event: ReactMouseEvent) => {
    event.preventDefault()
    setDragging(true)
    const move = (e: MouseEvent) => setDockWidth(window.innerWidth - e.clientX)
    const stop = () => { setDragging(false); window.removeEventListener('mousemove', move); window.removeEventListener('mouseup', stop) }
    window.addEventListener('mousemove', move); window.addEventListener('mouseup', stop)
  }
  if (!presented || !tab) return null
  if (layout === 'overlay') return <div className={styles.overlay}><ReaderPane tab={tab} dragging={false} /></div>
  return (
    <aside className={styles.dock} style={{ width: dockWidth }} aria-label="阅读栏">
      <div className={styles.splitter} role="separator" aria-orientation="vertical" aria-label="调整阅读栏宽度" onMouseDown={startDrag} />
      <ReaderPane tab={tab} dragging={dragging} />
    </aside>
  )
}
