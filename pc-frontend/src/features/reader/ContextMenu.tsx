import { useEffect, useRef, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import styles from './ReaderTabs.module.css'

export interface ContextMenuItem { label: string; onSelect(): void; disabled?: boolean; danger?: boolean }

/** Minimal right-click menu; closes on outside click, Escape, or selection. */
export default function ContextMenu({ x, y, items, onClose }: { x: number; y: number; items: ContextMenuItem[]; onClose(): void }) {
  const ref = useRef<HTMLDivElement>(null)
  useEffect(() => {
    const away = (event: MouseEvent) => { if (!ref.current?.contains(event.target as Node)) onClose() }
    const key = (event: KeyboardEvent) => { if (event.key === 'Escape') onClose() }
    window.addEventListener('mousedown', away, true); window.addEventListener('keydown', key); window.addEventListener('blur', onClose)
    return () => { window.removeEventListener('mousedown', away, true); window.removeEventListener('keydown', key); window.removeEventListener('blur', onClose) }
  }, [onClose])
  useEffect(() => {
    const node = ref.current; if (!node) return
    // Keep the menu inside the window when opened near the right/bottom edge.
    const rect = node.getBoundingClientRect()
    node.style.left = `${Math.max(4, Math.min(x, window.innerWidth - rect.width - 4))}px`
    node.style.top = `${Math.max(4, Math.min(y, window.innerHeight - rect.height - 4))}px`
    ;(node.querySelector('button:not(:disabled)') as HTMLButtonElement | null)?.focus()
  }, [x, y])
  const content: ReactNode = (
    <div ref={ref} className={styles.menu} role="menu" style={{ left: x, top: y }}>
      {items.map(item => (
        <button key={item.label} type="button" role="menuitem" disabled={item.disabled} data-danger={item.danger || undefined}
          onClick={() => { onClose(); item.onSelect() }}>{item.label}</button>
      ))}
    </div>
  )
  return createPortal(content, document.body)
}
