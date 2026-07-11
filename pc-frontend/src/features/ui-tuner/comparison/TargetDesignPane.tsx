import type { RefObject, UIEventHandler } from 'react'
import type { UiTunerReferenceImage } from '../types'
import { useTargetRegionSelection } from './useTargetRegionSelection'
import type { PixelRect } from './types'
import styles from './UiTunerComparisonWorkspace.module.css'

interface TargetDesignPaneProps {
  image?: UiTunerReferenceImage
  viewScale: number
  selectedRect: PixelRect | null
  scrollerRef: RefObject<HTMLDivElement>
  onScroll: UIEventHandler<HTMLDivElement>
  onSelectRect: (rect: PixelRect) => void
}

function rectStyle(rect: PixelRect) {
  return {
    left: rect.left,
    top: rect.top,
    width: rect.right - rect.left,
    height: rect.bottom - rect.top,
  }
}

export function TargetDesignPane({
  image,
  viewScale,
  selectedRect,
  scrollerRef,
  onScroll,
  onSelectRect,
}: TargetDesignPaneProps) {
  const size = { width: image?.width ?? 1, height: image?.height ?? 1 }
  const selection = useTargetRegionSelection(size, onSelectRect)

  if (!image) {
    return (
      <div className={styles.targetScroller} ref={scrollerRef} onScroll={onScroll}>
        <div className={styles.emptyPane}>
          <strong>尚未导入设计稿</strong>
          <span>点击顶部“导入设计图/截图”，设计稿会固定显示在左侧。</span>
        </div>
      </div>
    )
  }

  const activeRect = selection.draftRect ?? selectedRect
  return (
    <div className={styles.targetScroller} ref={scrollerRef} onScroll={onScroll}>
      <div
        className={styles.targetViewport}
        style={{ width: image.width * viewScale, height: image.height * viewScale }}
      >
        <div
          className={styles.targetCanvas}
          style={{ width: image.width, height: image.height, transform: `scale(${viewScale})` }}
          role="application"
          aria-label="目标设计稿，可拖动框选目标区域"
          {...selection.handlers}
        >
          <img
            className={styles.targetImage}
            src={image.dataUrl}
            alt={image.name}
            draggable={false}
          />
          {activeRect && <span className={styles.targetSelection} style={rectStyle(activeRect)} />}
        </div>
      </div>
    </div>
  )
}
