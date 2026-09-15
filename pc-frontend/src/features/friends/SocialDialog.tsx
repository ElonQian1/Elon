import { useEffect, useRef, type ReactNode } from 'react'
import styles from './SocialTools.module.css'

export default function SocialDialog({ title, onClose, children, footer, busy = false }: { title: string; onClose: () => void; children: ReactNode; footer?: ReactNode; busy?: boolean }) {
  const dialog = useRef<HTMLDialogElement>(null)
  useEffect(() => { dialog.current?.showModal() }, [])
  return <dialog ref={dialog} className={styles.dialog} aria-label={title} onCancel={event => { event.preventDefault(); if (!busy) onClose() }}>
    <header><h2>{title}</h2><button type="button" onClick={onClose} disabled={busy} aria-label={`关闭${title}`}>关闭</button></header>
    <div className={styles.body}>{children}</div>{footer && <footer>{footer}</footer>}
  </dialog>
}
