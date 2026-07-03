import type { KeyboardEvent } from 'react'
import type { PanelSide, WorkspacePanels } from './useWorkspacePanels'
import styles from './ConversationPage.module.css'

interface WorkspacePanelResizeHandleProps {
  side: PanelSide
  panels: WorkspacePanels
}

export default function WorkspacePanelResizeHandle({ side, panels }: WorkspacePanelResizeHandleProps) {
  const isChannel = side === 'channel'
  const label = isChannel ? '调整左侧栏宽度' : '调整右侧栏宽度'
  const title = isChannel ? '拖动调整左侧栏宽度，双击恢复默认' : '拖动调整右侧栏宽度，双击恢复默认'
  const handleClass = isChannel ? styles.channelResizeHandle : styles.memberResizeHandle

  function handleKeyDown(event: KeyboardEvent<HTMLElement>) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault()
      panels.resetPanelWidth(side)
      return
    }
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return
    event.preventDefault()
    const delta = isChannel
      ? event.key === 'ArrowRight' ? 12 : -12
      : event.key === 'ArrowLeft' ? 12 : -12
    panels.adjustPanelWidth(side, delta)
  }

  return (
    <div
      className={[styles.resizeHandle, handleClass].join(' ')}
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      title={title}
      tabIndex={0}
      data-dragging={panels.resizingSide === side ? 'true' : undefined}
      onPointerDown={(event) => panels.startPanelResize(side, event)}
      onDoubleClick={() => panels.resetPanelWidth(side)}
      onKeyDown={handleKeyDown}
    />
  )
}
