import { useEffect, useRef, type ReactNode } from 'react'
import styles from './GridShare.module.css'

export default function GridShareReaderDialog({ children, onClose }: { children: ReactNode; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null)
  useEffect(() => { dialog.current?.showModal() }, [])
  return <dialog ref={dialog} className={styles.panel} aria-label="网格快照详情" onCancel={event => { event.preventDefault(); onClose() }}>
    <header><h2>网格快照详情</h2><button type="button" onClick={onClose}>关闭</button></header>
    <div className={styles.panelBody}>{children}</div>
  </dialog>
}
