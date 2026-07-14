import { useCallback, useEffect, useState } from 'react'
import { Copy, Minus, Square, X } from 'lucide-react'
import { getDesktopWindow } from './desktopShell'
import styles from './DesktopTitleBar.module.css'

/**
 * 一龙桌面壳（无边框窗口）专用标题栏：拖拽区 + 最小化/最大化/关闭。
 * 只在 `isDesktopShellFrameless()` 为 true 时由 Shell 渲染；浏览器里不出现。
 * 关闭按钮走壳的 CloseRequested 流程（最小化到托盘），不会杀进程。
 */
export default function DesktopTitleBar() {
  const [maximized, setMaximized] = useState(false)

  const refreshMaximized = useCallback(() => {
    const win = getDesktopWindow()
    if (!win) return
    win.isMaximized().then(setMaximized).catch(() => {})
  }, [])

  useEffect(() => {
    refreshMaximized()
    // 窗口尺寸变化（含拖边最大化/还原、双击拖拽区）时同步按钮图标。
    window.addEventListener('resize', refreshMaximized)
    return () => window.removeEventListener('resize', refreshMaximized)
  }, [refreshMaximized])

  const minimize = () => {
    getDesktopWindow()?.minimize().catch(() => {})
  }
  const toggleMaximize = () => {
    const win = getDesktopWindow()
    if (!win) return
    win.toggleMaximize().then(refreshMaximized).catch(() => {})
  }
  const close = () => {
    getDesktopWindow()?.close().catch(() => {})
  }

  return (
    <header className={styles.titleBar}>
      <div className={styles.dragArea} data-tauri-drag-region>
        <span className={styles.appTitle}>一龙工作台</span>
      </div>
      <div className={styles.controls}>
        <button
          type="button"
          className={styles.controlButton}
          onClick={minimize}
          aria-label="最小化"
          title="最小化"
        >
          <Minus size={15} aria-hidden="true" />
        </button>
        <button
          type="button"
          className={styles.controlButton}
          onClick={toggleMaximize}
          aria-label={maximized ? '还原' : '最大化'}
          title={maximized ? '还原' : '最大化'}
        >
          {maximized ? <Copy size={13} aria-hidden="true" /> : <Square size={13} aria-hidden="true" />}
        </button>
        <button
          type="button"
          className={`${styles.controlButton} ${styles.closeButton}`}
          onClick={close}
          aria-label="关闭（最小化到托盘）"
          title="关闭（最小化到托盘）"
        >
          <X size={15} aria-hidden="true" />
        </button>
      </div>
    </header>
  )
}
