import { useEffect, type RefObject } from 'react'

export function useRecordWindow(ref: RefObject<HTMLDialogElement>) {
  useEffect(() => {
    const dialog = ref.current
    if (!dialog) return
    let drag: { id: number; x: number; y: number; left: number; top: number; target: Element } | undefined
    const reset = () => { dialog.style.left = ''; dialog.style.top = ''; dialog.style.margin = '' }
    const place = (left: number, top: number) => {
      const rect = dialog.getBoundingClientRect()
      dialog.style.margin = '0'
      dialog.style.left = `${Math.max(0, Math.min(left, innerWidth - rect.width))}px`
      dialog.style.top = `${Math.max(0, Math.min(top, innerHeight - rect.height))}px`
    }
    const handle = (e: Event) => e.target instanceof Element && e.target.closest('[data-record-drag]') && !e.target.closest('button,a,input')
    const down = (e: PointerEvent) => {
      if (innerWidth <= 520 || e.button !== 0 || !handle(e)) return
      const rect = dialog.getBoundingClientRect()
      const target = (e.target as Element).closest('[data-record-drag]')!
      drag = { id: e.pointerId, x: e.clientX, y: e.clientY, left: rect.left, top: rect.top, target }
      target.setPointerCapture(e.pointerId); e.preventDefault()
    }
    const move = (e: PointerEvent) => { if (drag?.id === e.pointerId) place(drag.left + e.clientX - drag.x, drag.top + e.clientY - drag.y) }
    const end = () => { const previous = drag; drag = undefined; if (previous?.target.hasPointerCapture(previous.id)) previous.target.releasePointerCapture(previous.id) }
    const center = (e: MouseEvent) => { if (handle(e)) reset() }
    const resize = () => { end(); if (innerWidth <= 520) reset(); else if (dialog.style.left) { const r = dialog.getBoundingClientRect(); place(r.left, r.top) } }
    dialog.addEventListener('pointerdown', down); dialog.addEventListener('pointermove', move)
    dialog.addEventListener('pointerup', end); dialog.addEventListener('pointercancel', end)
    dialog.addEventListener('lostpointercapture', end); dialog.addEventListener('dblclick', center)
    window.addEventListener('resize', resize)
    return () => {
      end(); dialog.removeEventListener('pointerdown', down); dialog.removeEventListener('pointermove', move)
      dialog.removeEventListener('pointerup', end); dialog.removeEventListener('pointercancel', end)
      dialog.removeEventListener('lostpointercapture', end); dialog.removeEventListener('dblclick', center)
      window.removeEventListener('resize', resize)
    }
  }, [ref])
}
