import type {
  CSSProperties,
  KeyboardEventHandler,
  PointerEvent as ReactPointerEvent,
  RefObject,
  ReactNode,
  UIEventHandler,
} from 'react'
import type { UiTunerFilterResult } from './filtering'
import type { LiveUiFrame } from './live/liveUiApi'
import type { LiveUiNode } from './live/liveUiApi'
import { RuntimeDraftLayer } from './live/RuntimeDraftLayer'
import type { RuntimeDraftState, RuntimeDraftStatus } from './live/runtimeDraftModel'
import type { UiTunerDocument, UiTunerElement } from './types'
import styles from './UiTunerPage.module.css'

interface UiTunerCanvasSurfaceProps {
  canvas: UiTunerDocument['canvas']
  filterResult: UiTunerFilterResult
  liveFrame: LiveUiFrame | null
  liveNodes: LiveUiNode[]
  runtimeDraftState: RuntimeDraftState
  runtimeDraftStatus: RuntimeDraftStatus
  realRenderer: boolean
  runtimeConnected: boolean
  runtimeGestureActive: boolean
  runtimeCanMove: boolean
  runtimeCanResize: boolean
  scrollerRef: RefObject<HTMLDivElement>
  selectedId: string | null
  viewScale: number
  overlayLayer?: ReactNode
  onScroll?: UIEventHandler<HTMLDivElement>
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
  liveNodes,
  runtimeDraftState,
  runtimeDraftStatus,
  realRenderer,
  runtimeConnected,
  runtimeGestureActive,
  runtimeCanMove,
  runtimeCanResize,
  scrollerRef,
  selectedId,
  viewScale,
  overlayLayer,
  onScroll,
  onCanvasKeyDown,
  onClearSelection,
  onElementPointerDown,
  onSelectElement,
}: UiTunerCanvasSurfaceProps) {
  return (
    <div className={styles.canvasScroller} ref={scrollerRef} onScroll={onScroll}>
      {realRenderer && (
        <div className={
          runtimeDraftStatus === 'rejected' || !runtimeConnected
            ? styles.runtimeSurfaceFrozen
            : styles.runtimeSurfaceLive
        }>
          {runtimeSurfaceLabel(runtimeConnected, runtimeDraftStatus)}
        </div>
      )}
      <div
        className={styles.canvasViewport}
        style={{ width: canvas.width * viewScale, height: canvas.height * viewScale }}
      >
        <div
          className={[
            styles.canvas,
            realRenderer && runtimeGestureActive ? styles.runtimeGestureCanvas : '',
          ].join(' ')}
          style={{
            width: canvas.width,
            height: canvas.height,
            background: canvas.background,
            transform: `scale(${viewScale})`,
            '--canvas-scale': viewScale,
          } as CSSProperties}
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
          {realRenderer && (
            <RuntimeDraftLayer
              canvasBackground={canvas.background}
              frame={liveFrame}
              nodes={liveNodes}
              state={runtimeDraftState}
            />
          )}
          {overlayLayer}
          {filterResult.visible.map(({ element, analysis }) => {
            const draftRect = realRenderer && element.runtime?.nodeId
              ? runtimeDraftState.nodes[element.runtime.nodeId]?.visual.rect
              : undefined
            const elementStyle: CSSProperties = realRenderer ? {
              left: draftRect?.left ?? element.x,
              top: draftRect?.top ?? element.y,
              width: draftRect?.width ?? element.width,
              height: draftRect?.height ?? element.height,
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
                aria-label={realRenderer ? `真实组件 ${element.name}` : undefined}
                data-runtime-node-id={realRenderer ? element.runtime?.nodeId : undefined}
                className={[
                  styles.canvasElement,
                  element.id === selectedId && !(realRenderer && runtimeGestureActive)
                    ? styles.selectedElement
                    : '',
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
                    if (element.id === selectedId && runtimeCanMove) {
                      onElementPointerDown(event, element, 'move')
                    } else {
                      onSelectElement(element.id)
                    }
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
                {realRenderer && !runtimeGestureActive && element.id === selectedId && runtimeCanResize && (
                  <span
                    className={styles.runtimeResizeHandle}
                    aria-label="拖动缩放真实 Android 组件"
                    onPointerDown={(event) => onElementPointerDown(event, element, 'resize')}
                  />
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

function runtimeSurfaceLabel(
  connected: boolean,
  status: UiTunerCanvasSurfaceProps['runtimeDraftStatus'],
) {
  if (!connected) return 'PC 草稿可继续编辑 · Android 正在重连'
  if (status === 'local') return 'PC 即时重绘 · 等待同步 Android'
  if (status === 'syncing') return 'PC 即时重绘 · Android 后台同步中'
  if (status === 'calibrating') return 'Android 已应用 · 正在校准真机画面'
  if (status === 'rejected') return 'PC 草稿已保留 · Android 同步失败'
  return 'Android LIVE · PC 本地即时渲染'
}
