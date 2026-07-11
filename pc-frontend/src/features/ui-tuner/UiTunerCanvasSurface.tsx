import type {
  CSSProperties,
  KeyboardEventHandler,
  PointerEvent as ReactPointerEvent,
  RefObject,
} from 'react'
import type { UiTunerFilterResult } from './filtering'
import type { LiveUiFrame } from './live/liveUiApi'
import type { UiTunerDocument, UiTunerElement } from './types'
import styles from './UiTunerPage.module.css'

interface UiTunerCanvasSurfaceProps {
  canvas: UiTunerDocument['canvas']
  filterResult: UiTunerFilterResult
  liveFrame: LiveUiFrame | null
  realRenderer: boolean
  scrollerRef: RefObject<HTMLDivElement>
  selectedId: string | null
  viewScale: number
  onCanvasKeyDown: KeyboardEventHandler<HTMLDivElement>
  onClearSelection: () => void
  onElementPointerDown: (
    event: ReactPointerEvent<HTMLElement>,
    element: UiTunerElement,
    mode: 'move' | 'resize',
  ) => void
  onSelectElement: (id: string) => void
}

export function UiTunerCanvasSurface({
  canvas,
  filterResult,
  liveFrame,
  realRenderer,
  scrollerRef,
  selectedId,
  viewScale,
  onCanvasKeyDown,
  onClearSelection,
  onElementPointerDown,
  onSelectElement,
}: UiTunerCanvasSurfaceProps) {
  return (
    <div className={styles.canvasScroller} ref={scrollerRef}>
      <div
        className={styles.canvasViewport}
        style={{ width: canvas.width * viewScale, height: canvas.height * viewScale }}
      >
        <div
          className={styles.canvas}
          style={{
            width: canvas.width,
            height: canvas.height,
            background: canvas.background,
            transform: `scale(${viewScale})`,
          }}
          tabIndex={0}
          onKeyDown={onCanvasKeyDown}
          onPointerDown={(event) => {
            if (event.target === event.currentTarget) onClearSelection()
          }}
        >
          {!realRenderer && <div className={styles.canvasGrid} aria-hidden="true" />}
          {liveFrame ? (
            <img className={styles.referenceImage} src={liveFrame.dataUrl} alt="真机实时画面" />
          ) : canvas.referenceImage?.visible && (
            <img
              className={styles.referenceImage}
              src={canvas.referenceImage.dataUrl}
              alt=""
              style={{ opacity: canvas.referenceImage.opacity }}
            />
          )}
          {canvas.targetDesign?.visible && (
            <img
              className={styles.targetDesignImage}
              src={canvas.targetDesign.dataUrl}
              alt="目标设计图"
              style={{ opacity: canvas.targetDesign.opacity }}
            />
          )}
          {filterResult.visible.map(({ element, analysis }) => {
            const elementStyle: CSSProperties = realRenderer ? {
              left: element.x,
              top: element.y,
              width: element.width,
              height: element.height,
            } : {
              left: element.x,
              top: element.y,
              width: element.width,
              height: element.height,
              padding: `${element.paddingY}px ${element.paddingX}px`,
              borderRadius: element.borderRadius,
              borderWidth: element.borderWidth,
              borderColor: element.borderColor,
              color: element.color,
              background: element.background,
              opacity: element.opacity,
              fontSize: element.fontSize,
              lineHeight: `${element.lineHeight}px`,
              fontWeight: element.fontWeight,
              letterSpacing: element.letterSpacing,
            }
            return (
              <button
                key={element.id}
                type="button"
                className={[
                  styles.canvasElement,
                  element.id === selectedId ? styles.selectedElement : '',
                  analysis.appearance === 'ghost'
                    ? styles.ghostElement
                    : analysis.appearance === 'outline'
                      ? styles.outlineElement
                      : '',
                  analysis.isLocked ? styles.lockedElement : '',
                  styles[`kind_${element.kind}`],
                  realRenderer ? styles.runtimeHitTarget : '',
                ].join(' ')}
                style={elementStyle}
                onPointerDown={(event) => {
                  if (realRenderer) {
                    event.stopPropagation()
                    onSelectElement(element.id)
                    return
                  }
                  if (analysis.isLocked) {
                    event.stopPropagation()
                    onSelectElement(element.id)
                    return
                  }
                  onElementPointerDown(event, element, 'move')
                }}
              >
                {!realRenderer && <span>{analysis.appearance === 'outline' ? analysis.role : element.text}</span>}
                {realRenderer && element.id === selectedId && (
                  <span className={styles.runtimeSelectionLabel}>{element.name}</span>
                )}
                {!realRenderer && element.id === selectedId && !analysis.isLocked && (
                  <span
                    className={styles.resizeHandle}
                    aria-hidden="true"
                    onPointerDown={(event) => onElementPointerDown(event, element, 'resize')}
                  />
                )}
              </button>
            )
          })}
        </div>
      </div>
    </div>
  )
}
