import { useEffect, useRef, type ReactNode } from 'react'
import styles from './Articles.module.css'

export default function ArticleDialog({ label, onCancel, children }: { label: string; onCancel: () => void; children: ReactNode }) {
  const ref = useRef<HTMLDialogElement>(null)
  useEffect(() => { const dialog = ref.current; dialog?.showModal(); return () => dialog?.close() }, [])
  return <dialog ref={ref} className={styles.overlay} aria-label={label} onCancel={e => { e.preventDefault(); onCancel() }}>{children}</dialog>
}
