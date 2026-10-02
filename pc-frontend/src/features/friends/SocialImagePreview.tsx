import { useCallback, useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { ArrowDownToLine, ArrowUpToLine, Download, Maximize, MoveHorizontal, Minus, Plus, RotateCcw, X } from 'lucide-react'
import { ManualImageQr } from './source-links/SourceLinkView'
import { clampImage, fitImage, imageScales, resizeImage, zoomImage, type ImageSize, type ImageViewport } from './socialImageGeometry'
import styles from './SocialImagePreview.module.css'

export default function SocialImagePreview({ url, name, onClose, returnFocus }: { url: string; name: string; onClose: () => void; returnFocus: HTMLElement | null }) {
  const dialog = useRef<HTMLDialogElement>(null), stage = useRef<HTMLDivElement>(null)
  const [image, setImage] = useState<ImageSize | null>(null)
  const [size, setSize] = useState<ImageSize>({ width: 1, height: 1 })
  const [view, setView] = useState<ImageViewport>({ scale: 1, x: 0, y: 0, reading: false })
  const [failed, setFailed] = useState(false), [attempt, setAttempt] = useState(0)
  const state = useRef({ image, size, view }); state.current = { image, size, view }
  const pointers = useRef(new Map<number, { x: number; y: number }>())
  const gesture = useRef<{ view: ImageViewport; x: number; y: number; distance: number } | null>(null)

  useEffect(() => {
    const previous = returnFocus || document.activeElement as HTMLElement | null
    const overflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'; dialog.current?.showModal()
    return () => { document.body.style.overflow = overflow; if (previous?.isConnected) previous.focus({ preventScroll: true }) }
  }, [returnFocus])
  useEffect(() => {
    const node = stage.current
    if (!node) return
    const observer = new ResizeObserver(() => {
      const next = { width: Math.max(1, node.clientWidth), height: Math.max(1, node.clientHeight) }
      const current = state.current
      if (current.image) {
        const resized = resizeImage(current.view, current.image, current.size, next)
        state.current = { ...current, size: next, view: resized }; setView(resized)
      } else state.current = { ...current, size: next }
      setSize(next)
    })
    observer.observe(node)
    return () => observer.disconnect()
  }, [])
  const zoom = useCallback((factor: number, point = { x: 0, y: 0 }) => {
    const current = state.current
    if (current.image) setView(v => zoomImage(v, v.scale * factor, point, current.image!, current.size))
  }, [])
  useEffect(() => {
    const node = stage.current
    if (!node) return
    const wheel = (event: WheelEvent) => {
      event.preventDefault()
      const current = state.current, bounds = node.getBoundingClientRect()
      if (!current.image) return
      const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? current.size.height : 1
      if (current.view.reading && !event.ctrlKey && !event.metaKey) {
        setView(v => clampImage({ ...v, x: v.x - event.deltaX * unit, y: v.y - event.deltaY * unit }, current.image!, current.size))
      } else zoom(Math.exp(-Math.max(-400, Math.min(400, event.deltaY * unit)) * .002), { x: event.clientX - bounds.left - bounds.width / 2, y: event.clientY - bounds.top - bounds.height / 2 })
    }
    node.addEventListener('wheel', wheel, { passive: false })
    return () => node.removeEventListener('wheel', wheel)
  }, [zoom])
  const fit = (reading: boolean) => { if (image) setView(fitImage(image, size, reading)) }
  const edge = (bottom: boolean) => { if (image) setView(v => clampImage({ ...v, y: (bottom ? -1 : 1) * image.height * v.scale }, image, size)) }
  const snapshot = () => {
    const [a, b] = [...pointers.current.values()]
    gesture.current = a ? { view: state.current.view, x: b ? (a.x + b.x) / 2 : a.x, y: b ? (a.y + b.y) / 2 : a.y, distance: b ? Math.hypot(a.x - b.x, a.y - b.y) : 0 } : null
  }
  const point = (event: React.PointerEvent) => {
    const bounds = stage.current!.getBoundingClientRect()
    return { x: event.clientX - bounds.left - bounds.width / 2, y: event.clientY - bounds.top - bounds.height / 2 }
  }
  const finishPointer = (event: React.PointerEvent) => { pointers.current.delete(event.pointerId); snapshot() }
  const scales = image ? imageScales(image, size) : null
  return createPortal(<dialog ref={dialog} className={styles.dialog} aria-label={`图片预览：${name}`} onCancel={event => { event.preventDefault(); onClose() }} onClose={onClose}
    onKeyDown={event => {
      if (!image || (event.target as HTMLElement).closest('button,a,input')) return
      if (['+', '=', '-'].includes(event.key)) { event.preventDefault(); zoom(event.key === '-' ? 1 / 1.5 : 1.5) }
      if (event.key === '0') { event.preventDefault(); fit(false) }
      if (event.key === 'Home' || event.key === 'End') { event.preventDefault(); edge(event.key === 'End') }
      const step = ({ ArrowUp: 60, ArrowDown: -60, PageUp: size.height * .8, PageDown: -size.height * .8, ' ': -size.height * .8 } as Record<string, number>)[event.key]
      if (step) { event.preventDefault(); setView(v => clampImage({ ...v, y: v.y + step }, image, size)) }
    }}>
    <header className={styles.header}>
      <strong>{name}</strong>
      <a href={url} download title="下载原图" aria-label="下载原图"><Download size={20} /></a>
      <button type="button" title="关闭图片预览" aria-label="关闭图片预览" autoFocus onClick={onClose}><X size={22} /></button>
    </header>
    <div className={styles.stage} ref={stage} tabIndex={0} aria-label="图片阅读区域" data-reading={view.reading} data-scale={view.scale}
      onPointerDown={event => {
        if (!image || event.button !== 0) return
        event.currentTarget.focus(); pointers.current.set(event.pointerId, point(event)); snapshot(); event.currentTarget.setPointerCapture(event.pointerId)
      }}
      onPointerMove={event => {
        const start = gesture.current
        if (!image || !start || !pointers.current.has(event.pointerId)) return
        pointers.current.set(event.pointerId, point(event))
        const [a, b] = [...pointers.current.values()], x = b ? (a.x + b.x) / 2 : a.x, y = b ? (a.y + b.y) / 2 : a.y
        const next = b && start.distance ? zoomImage(start.view, start.view.scale * Math.hypot(a.x - b.x, a.y - b.y) / start.distance, { x: start.x, y: start.y }, image, size) : start.view
        setView(clampImage({ ...next, x: next.x + x - start.x, y: next.y + y - start.y }, image, size))
      }} onPointerUp={finishPointer} onPointerCancel={finishPointer} onLostPointerCapture={finishPointer}
      onDoubleClick={event => {
        if (!image) return
        const base = view.reading ? scales!.width : scales!.fit
        const bounds = event.currentTarget.getBoundingClientRect()
        setView(zoomImage(view, view.scale > base * 1.1 ? base : Math.max(base * 2, 1), { x: event.clientX - bounds.left - bounds.width / 2, y: event.clientY - bounds.top - bounds.height / 2 }, image, size))
      }}>
      <img key={attempt} src={url} alt={name} draggable={false} hidden={failed} className={styles.image}
        style={image ? { width: image.width, height: image.height, transform: `translate(-50%, -50%) translate(${view.x}px, ${view.y}px) scale(${view.scale})` } : { visibility: 'hidden' }}
        onLoad={event => {
          const next = { width: event.currentTarget.naturalWidth, height: event.currentTarget.naturalHeight }
          const bounds = { width: stage.current!.clientWidth, height: stage.current!.clientHeight }
          setImage(next); setFailed(false); setSize(bounds); setView(fitImage(next, bounds, imageScales(next, bounds).long))
        }} onError={() => { setFailed(true); setImage(null) }} />
      {!image && <div className={styles.status} role="status">{failed ? <>图片加载失败 <button type="button" onClick={() => { setFailed(false); setAttempt(attempt + 1) }}>重新加载</button></> : '正在加载图片…'}</div>}
    </div>
    <footer className={styles.controls}>
      <button type="button" disabled={!image} aria-pressed={view.reading} onClick={() => fit(true)} title="长图阅读：滚轮上下阅读，Ctrl+滚轮缩放"><MoveHorizontal size={18} />长图阅读</button>
      <button type="button" disabled={!image} aria-pressed={!view.reading} onClick={() => fit(false)} title="整图预览：滚轮缩放"><Maximize size={18} />整图</button>
      <button type="button" aria-label="缩小图片" title="缩小图片" disabled={!scales || view.scale <= scales.fit} onClick={() => zoom(1 / 1.5)}><Minus size={18} /></button>
      <output aria-label="缩放比例">{Math.round(view.scale * 100)}%</output>
      <button type="button" aria-label="放大图片" title="放大图片" disabled={!scales || view.scale >= scales.max} onClick={() => zoom(1.5)}><Plus size={18} /></button>
      <button type="button" aria-label="原始比例" title="原始比例" disabled={!image} onClick={() => zoom(1 / view.scale)}><RotateCcw size={18} /></button>
      <button type="button" aria-label="回到图片顶部" title="回到图片顶部" disabled={!image} onClick={() => edge(false)}><ArrowUpToLine size={18} /></button>
      <button type="button" aria-label="跳到图片底部" title="跳到图片底部" disabled={!image} onClick={() => edge(true)}><ArrowDownToLine size={18} /></button>
      <ManualImageQr url={url} />
    </footer>
  </dialog>, document.body)
}
