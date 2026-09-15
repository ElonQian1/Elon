import { useLayoutEffect, useRef, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import type { SocialMenuRequest } from './socialMessageContext'
import styles from './SocialMessageMenu.module.css'

export interface SocialMenuItem {
  label: string; icon: ReactNode; action: () => void
  hint?: string; danger?: boolean; disabled?: boolean
}

export default function SocialContextMenu({ request, groups, onClose }: {
  request: SocialMenuRequest; groups: SocialMenuItem[][]; onClose: () => void
}) {
  const menu = useRef<HTMLDivElement>(null)
  const close = useRef(onClose); close.current = onClose
  useLayoutEffect(() => {
    const node = menu.current!
    const zoom = node.getBoundingClientRect().width / parseFloat(getComputedStyle(node).width) || 1
    node.style.maxHeight = `${(innerHeight - 16) / zoom}px`
    node.style.maxWidth = `${(innerWidth - 16) / zoom}px`
    const rect = node.getBoundingClientRect()
    node.style.left = `${Math.max(8, Math.min(request.x, innerWidth - rect.width - 8)) / zoom}px`
    node.style.top = `${Math.max(8, Math.min(request.y, innerHeight - rect.height - 8)) / zoom}px`
    node.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus({ preventScroll: true })
    const positions = new Map<Element, string>()
    for (let ancestor: Element | null = request.origin; ancestor; ancestor = ancestor.parentElement) positions.set(ancestor, `${ancestor.scrollLeft}:${ancestor.scrollTop}`)
    const outside = (event: PointerEvent) => { if (!node.contains(event.target as Node)) close.current() }
    const dismiss = () => close.current()
    const scroll = (event: Event) => {
      const target = event.target === document ? document.scrollingElement : event.target
      if (target instanceof Element && !node.contains(target) && positions.get(target) !== `${target.scrollLeft}:${target.scrollTop}`) dismiss()
    }
    document.addEventListener('pointerdown', outside, true)
    document.addEventListener('scroll', scroll, true)
    window.addEventListener('resize', dismiss)
    window.addEventListener('blur', dismiss)
    return () => {
      document.removeEventListener('pointerdown', outside, true)
      document.removeEventListener('scroll', scroll, true)
      window.removeEventListener('resize', dismiss)
      window.removeEventListener('blur', dismiss)
    }
  }, [request])
  function dismiss() {
    onClose()
    if (request.origin.isConnected) request.origin.focus({ preventScroll: true })
  }
  return createPortal(<div ref={menu} className={styles.menu} role="menu" aria-label="消息操作"
    style={{ left: request.x, top: request.y }} onContextMenu={event => { event.preventDefault(); event.stopPropagation() }}
    onKeyDown={event => {
      event.stopPropagation()
      if (event.key === 'Escape' || event.key === 'Tab') { event.preventDefault(); dismiss() }
      if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
        event.preventDefault()
        const buttons = Array.from(menu.current!.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'))
        const index = buttons.indexOf(document.activeElement as HTMLButtonElement)
        buttons[event.key === 'Home' ? 0 : event.key === 'End' ? buttons.length - 1 : (index + (event.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length]?.focus()
      }
    }}>
    {groups.filter(group => group.length).map((group, index) => <div role="group" key={index}>
      {index > 0 && <div role="separator" className={styles.separator} />}
      {group.map(item => <button type="button" role="menuitem" key={item.label} className={item.danger ? styles.danger : undefined}
        disabled={item.disabled} onClick={() => { dismiss(); item.action() }}>
        <span className={styles.icon} aria-hidden="true">{item.icon}</span><span>{item.label}</span>
        {item.hint && <span className={styles.hint} aria-hidden="true">{item.hint}</span>}
      </button>)}
    </div>)}
  </div>, document.body)
}
