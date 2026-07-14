import type { CSSProperties } from 'react'
import type { LiveUiFrame, LiveUiNode } from './liveUiApi'
import {
  nearestRuntimeSurfaceColor,
  type RuntimeDraftNode,
  type RuntimeDraftState,
} from './runtimeDraftModel'
import styles from './RuntimeDraftLayer.module.css'

interface RuntimeDraftLayerProps {
  canvasBackground: string
  frame: LiveUiFrame | null
  nodes: LiveUiNode[]
  state: RuntimeDraftState
}

export function RuntimeDraftLayer({
  canvasBackground,
  frame,
  nodes,
  state,
}: RuntimeDraftLayerProps) {
  const nodeById = new Map(nodes.map((node) => [node.runtimeNodeId, node]))
  return (
    <div className={styles.layer} aria-label="PC 本地即时预览层">
      {Object.values(state.nodes).map((draft) => {
        const node = nodeById.get(draft.runtimeNodeId)
        if (!node || !draft.visual.visible) return null
        const maskColor = nearestRuntimeSurfaceColor(node, nodes, canvasBackground)
        return (
          <RuntimeDraftPreview
            key={draft.runtimeNodeId}
            draft={draft}
            frame={frame}
            maskColor={maskColor}
          />
        )
      })}
    </div>
  )
}

function RuntimeDraftPreview({
  draft,
  frame,
  maskColor,
}: {
  draft: RuntimeDraftNode
  frame: LiveUiFrame | null
  maskColor: string
}) {
  const { baseRect, rect } = draft.visual
  const geometryOnly = Object.keys(draft.operations).every((property) => (
    property === 'width'
    || property === 'height'
    || property === 'translationX'
    || property === 'translationY'
  ))
  const baseStyle = {
    left: baseRect.left,
    top: baseRect.top,
    width: baseRect.width,
    height: baseRect.height,
    background: maskColor,
  }
  const previewStyle: CSSProperties = {
    left: rect.left,
    top: rect.top,
    width: rect.width,
    height: rect.height,
    color: draft.visual.color,
    background: draft.visual.background,
    borderColor: draft.visual.borderColor,
    borderWidth: draft.visual.borderWidth,
    borderRadius: draft.visual.borderRadius,
    opacity: draft.visual.opacity,
    padding: `${draft.visual.paddingTop}px ${draft.visual.paddingRight}px ${draft.visual.paddingBottom}px ${draft.visual.paddingLeft}px`,
    fontSize: draft.visual.fontSize,
    lineHeight: `${draft.visual.lineHeight}px`,
    fontWeight: draft.visual.fontWeight,
    letterSpacing: draft.visual.letterSpacing,
  }
  if (geometryOnly && frame) {
    Object.assign(previewStyle, {
      backgroundImage: `url(${frame.dataUrl})`,
      backgroundPosition: `${-baseRect.left}px ${-baseRect.top}px`,
      backgroundSize: `${frame.width}px ${frame.height}px`,
      backgroundRepeat: 'no-repeat',
    })
  }
  return (
    <>
      <span className={styles.baseMask} style={baseStyle} aria-hidden="true" />
      <span
        className={[
          styles.preview,
          geometryOnly ? styles.bitmapPreview : styles.stylePreview,
          draft.phase === 'rejected' ? styles.rejected : '',
        ].join(' ')}
        style={previewStyle}
        data-runtime-draft-node={draft.runtimeNodeId}
        data-runtime-draft-revision={draft.localRevision}
        aria-label={`${draft.definitionId} 本地预览`}
      >
        {!geometryOnly && draft.visual.text}
      </span>
    </>
  )
}
