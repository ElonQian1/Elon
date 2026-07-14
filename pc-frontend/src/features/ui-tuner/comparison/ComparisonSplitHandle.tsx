import { useRef, type KeyboardEvent, type PointerEvent } from 'react'
import styles from './UiTunerComparisonWorkspace.module.css'

interface ComparisonSplitHandleProps {
  ratio: number
  onChange: (ratio: number) => void
}

interface DragState {
  pointerId: number
  left: number
  width: number
}

export function ComparisonSplitHandle({ ratio, onChange }: ComparisonSplitHandleProps) {
  const drag = useRef<DragState | null>(null)

  const updateFromPointer = (clientX: number) => {
    if (!drag.current) return
    onChange(((clientX - drag.current.left) / drag.current.width) * 100)
  }

  const handlePointerDown = (event: PointerEvent<HTMLDivElement>) => {
    const bounds = event.currentTarget.parentElement?.getBoundingClientRect()
    if (!bounds || bounds.width <= 0) return
    event.currentTarget.setPointerCapture(event.pointerId)
    drag.current = { pointerId: event.pointerId, left: bounds.left, width: bounds.width }
    updateFromPointer(event.clientX)
  }

  const handlePointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (drag.current?.pointerId !== event.pointerId) return
    updateFromPointer(event.clientX)
  }

  const finishPointer = (event: PointerEvent<HTMLDivElement>) => {
    if (drag.current?.pointerId !== event.pointerId) return
    event.currentTarget.releasePointerCapture(event.pointerId)
    drag.current = null
  }

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return
    event.preventDefault()
    onChange(ratio + (event.key === 'ArrowLeft' ? -5 : 5))
  }

  return (
    <div
      className={styles.splitHandle}
      role="separator"
      aria-label="调整设计稿与真机画布比例"
      aria-orientation="vertical"
      aria-valuemin={20}
      aria-valuemax={80}
      aria-valuenow={ratio}
      tabIndex={0}
      onDoubleClick={() => onChange(35)}
      onKeyDown={handleKeyDown}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={finishPointer}
      onPointerCancel={finishPointer}
    >
      <span />
    </div>
  )
}
