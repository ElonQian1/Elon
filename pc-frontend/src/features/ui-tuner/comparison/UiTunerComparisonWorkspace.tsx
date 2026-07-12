import type {
  KeyboardEventHandler,
  PointerEvent as ReactPointerEvent,
  RefObject,
  UIEventHandler,
} from 'react'
import { useCallback, useMemo, useState } from 'react'
import type { UiTunerFilterResult } from '../filtering'
import { AutoFitQueuePanel } from '../fit-run/AutoFitQueuePanel'
import type { LivePreviewRequest, LiveUiFrame, LiveUiNode, LiveUiSession } from '../live/liveUiApi'
import type { LiveTargetDesign } from '../live/liveUiIrApi'
import { UiFitRunPanel } from '../fit-run/UiFitRunPanel'
import { useAutoFitQueue } from '../fit-run/useAutoFitQueue'
import { useFitRun } from '../fit-run/useFitRun'
import { useFitRunStore } from '../fit-run/fitRunStore'
import type { UiTunerDocument, UiTunerElement } from '../types'
import { UiTunerCanvasSurface } from '../UiTunerCanvasSurface'
import { ComparisonModeControls } from './ComparisonModeControls'
import { ComparisonOverlayLayer } from './ComparisonOverlayLayer'
import { CalibrationPanel } from './CalibrationPanel'
import { DesignDiffRegionsPanel } from './DesignDiffRegionsPanel'
import type { DesignDiffRegion, DesignDiffRegionAnalysis } from './autoPairApi'
import { createCalibration, unionRects } from './comparisonGeometry'
import { TargetDesignPane } from './TargetDesignPane'
import type { TargetCurrentPair } from './types'
import { useComparisonWorkspace } from './useComparisonWorkspace'
import styles from './UiTunerComparisonWorkspace.module.css'

interface UiTunerComparisonWorkspaceProps {
  document: UiTunerDocument
  filterResult: UiTunerFilterResult
  liveFrame: LiveUiFrame | null
  liveNode: LiveUiNode | null
  liveNodes: LiveUiNode[]
  liveSession: LiveUiSession | null
  previewRequest: LivePreviewRequest | null
  uploadedTarget: LiveTargetDesign | null
  realRenderer: boolean
  runtimeConnected: boolean
  runtimeGestureActive: boolean
  runtimeCanMove: boolean
  runtimeCanResize: boolean
  selectedId: string | null
  viewScale: number
  targetScrollerRef: RefObject<HTMLDivElement>
  currentScrollerRef: RefObject<HTMLDivElement>
  onTargetScroll: UIEventHandler<HTMLDivElement>
  onCurrentScroll: UIEventHandler<HTMLDivElement>
  onCanvasKeyDown: KeyboardEventHandler<HTMLDivElement>
  onClearSelection: () => void
  onElementPointerDown: (
    event: ReactPointerEvent<HTMLElement>,
    element: UiTunerElement,
    mode: 'move' | 'resize',
  ) => void
  onSelectElement: (id: string) => void
  onPairChange?: (pair: TargetCurrentPair | null) => void
  onNotice: (message: string) => void
}

export function UiTunerComparisonWorkspace({
  document,
  filterResult,
  liveFrame,
  liveNode,
  liveNodes,
  liveSession,
  previewRequest,
  uploadedTarget,
  realRenderer,
  runtimeConnected,
  runtimeGestureActive,
  runtimeCanMove,
  runtimeCanResize,
  selectedId,
  viewScale,
  targetScrollerRef,
  currentScrollerRef,
  onTargetScroll,
  onCurrentScroll,
  onCanvasKeyDown,
  onClearSelection,
  onElementPointerDown,
  onSelectElement,
  onPairChange,
  onNotice,
}: UiTunerComparisonWorkspaceProps) {
  const [autoAnalysis, setAutoAnalysis] = useState<DesignDiffRegionAnalysis | null>(null)
  const selected = document.elements.find((element) => element.id === selectedId) ?? null
  const currentSize = useMemo(() => ({
    width: liveFrame?.width ?? document.canvas.width,
    height: liveFrame?.height ?? document.canvas.height,
  }), [document.canvas.height, document.canvas.width, liveFrame?.height, liveFrame?.width])
  const currentCanvas = useMemo(() => ({
    ...document.canvas,
    width: currentSize.width,
    height: currentSize.height,
  }), [currentSize.height, currentSize.width, document.canvas])
  const setSharedPair = useFitRunStore((state) => state.setPair)
  const publishPair = useCallback((pair: TargetCurrentPair | null) => {
    setSharedPair(pair)
    onPairChange?.(pair)
  }, [onPairChange, setSharedPair])
  const comparison = useComparisonWorkspace({
    document,
    selected,
    liveNode,
    currentSize,
    onPairChange: publishPair,
  })
  const target = document.canvas.targetDesign
  const targetSize = useMemo(() => ({
    width: target?.width ?? document.canvas.width,
    height: target?.height ?? document.canvas.height,
  }), [document.canvas.height, document.canvas.width, target?.height, target?.width])
  const calibration = comparison.calibration ?? createCalibration(targetSize, currentSize)
  const fitInput = useMemo(() => {
    const pair = comparison.pair
    if (!pair || !target || !liveFrame || !liveNode || !uploadedTarget) return undefined
    return {
      pair: {
        targetDesignId: uploadedTarget.id,
        targetSha256: uploadedTarget.sha256,
        targetRect: pair.targetRect,
        runtimeNodeId: liveNode.runtimeNodeId,
        definitionId: liveNode.definitionId,
        componentKind: liveNode.kind,
        parentLayoutKind: parentLayoutKind(liveNode, liveNodes),
        instanceKey: liveNode.instanceKey,
        currentRect: pair.currentRect,
        projectedTargetRect: pair.projectedTargetRect,
        calibrationId: pair.calibrationId,
        confidence: 1,
      },
      environment: {
        screenId: liveNode.screenId,
        scenario: previewRequest?.scenario,
        theme: previewRequest?.theme,
        locale: previewRequest?.locale,
        viewportWidth: liveFrame.width,
        viewportHeight: liveFrame.height,
        density: liveNode.geometry.density,
        fontScale: previewRequest?.fontScale ?? liveNode.geometry.fontScale,
        rotation: liveNode.geometry.rotation,
      },
      properties: editableSolverProperties(liveNode),
      autoStart: true,
    }
  }, [comparison.pair, liveFrame, liveNode, liveNodes, previewRequest, target, uploadedTarget])
  const fitRun = useFitRun({ sessionId: liveSession?.id, input: fitInput, onNotice })
  const chooseAutoRegion = useCallback((region: DesignDiffRegion) => {
    const candidate = region.candidates.find((item) => (
      item.runtimeNodeId === region.recommendedRuntimeNodeId
    )) ?? region.candidates[0]
    if (!candidate) {
      onNotice('这个差异区域尚未找到可编辑的真实 Android 节点')
      return false
    }
    const element = document.elements.find((item) => item.runtime?.nodeId === candidate.runtimeNodeId)
      ?? document.elements.find((item) => item.runtime?.xpath === candidate.definitionId)
    if (!element) {
      onNotice(`已识别 ${candidate.definitionId}，但当前画布还没有对应元素，请刷新 Runtime 节点`)
      return false
    }
    // 差异框只包含发生变化的像素，而 FitRun 的 currentRect 是完整组件。
    // 自动配对时合并候选组件范围，保证目标和当前画面比较的是同一语义区域。
    comparison.setTargetRect(unionRects(region.targetRect, candidate.targetBounds))
    onSelectElement(element.id)
    onNotice(`已自动配对 ${candidate.definitionId}，可以开始自动拟合`)
    return true
  }, [comparison, document.elements, onNotice, onSelectElement])
  const autoQueue = useAutoFitQueue({
    sessionId: liveSession?.id,
    fitRun,
    fitInput,
    activateRegion: chooseAutoRegion,
    onNotice,
  })
  const overlay = target && comparison.mode !== 'split'
    ? (
        <ComparisonOverlayLayer
          mode={comparison.mode}
          image={target}
          currentSize={currentSize}
          opacity={comparison.overlayOpacity}
          targetRect={comparison.targetRect}
          calibration={calibration}
        />
      )
    : undefined

  const currentSurface = (
    <UiTunerCanvasSurface
      canvas={currentCanvas}
      filterResult={filterResult}
      liveFrame={liveFrame}
      realRenderer={realRenderer}
      runtimeConnected={runtimeConnected}
      runtimeGestureActive={runtimeGestureActive}
      runtimeCanMove={runtimeCanMove}
      runtimeCanResize={runtimeCanResize}
      scrollerRef={currentScrollerRef}
      selectedId={selectedId}
      viewScale={viewScale}
      overlayLayer={overlay}
      onScroll={onCurrentScroll}
      onCanvasKeyDown={onCanvasKeyDown}
      onClearSelection={onClearSelection}
      onElementPointerDown={onElementPointerDown}
      onSelectElement={onSelectElement}
    />
  )

  return (
    <div className={styles.workspace}>
      <ComparisonModeControls
        mode={comparison.mode}
        opacity={comparison.overlayOpacity}
        pair={comparison.pair}
        targetReady={Boolean(target)}
        onModeChange={comparison.setMode}
        onOpacityChange={comparison.setOverlayOpacity}
        onClearPair={comparison.clearPair}
      />
      <DesignDiffRegionsPanel
        sessionId={liveSession?.id}
        targetReady={Boolean(target)}
        onChooseRegion={chooseAutoRegion}
        onAnalysisChange={setAutoAnalysis}
      />
      <AutoFitQueuePanel analysis={autoAnalysis} queue={autoQueue} />
      <UiFitRunPanel fitRun={fitRun} pairReady={Boolean(fitInput)} />
      {target && (
        <CalibrationPanel
          calibration={calibration}
          targetSize={targetSize}
          currentSize={currentSize}
          onChange={comparison.setCalibration}
        />
      )}
      {comparison.mode === 'split' ? (
        <div className={styles.splitGrid}>
          <section className={styles.pane} aria-label="目标设计稿">
            <header className={styles.paneHeader}>
              <strong>设计稿</strong>
              <span>{target ? `${target.width} × ${target.height}` : '等待导入'}</span>
            </header>
            <TargetDesignPane
              image={target}
              viewScale={viewScale}
              selectedRect={comparison.targetRect}
              scrollerRef={targetScrollerRef}
              onScroll={onTargetScroll}
              onSelectRect={comparison.setTargetRect}
            />
          </section>
          <section className={styles.pane} aria-label="真实 Android 现状">
            <header className={styles.paneHeader}>
              <strong>真实 Android</strong>
              <span>{liveFrame ? `${liveFrame.width} × ${liveFrame.height} · 实时` : '等待真机画面'}</span>
            </header>
            {currentSurface}
          </section>
        </div>
      ) : (
        <section className={styles.singlePane} aria-label="设计稿与真实 Android 对比">
          {currentSurface}
        </section>
      )}
    </div>
  )
}

const SOLVER_PROPERTIES = new Set([
  'width', 'height', 'opacity', 'padding.start', 'padding.top', 'padding.end',
  'padding.bottom', 'margin.start', 'margin.top', 'margin.end', 'margin.bottom',
  'cornerRadius.all', 'textSize', 'fontWeight', 'lineHeight', 'letterSpacing',
  'borderWidth', 'backgroundColor', 'contentColor', 'borderColor',
])

function parentLayoutKind(node: LiveUiNode, nodes: LiveUiNode[]) {
  const parent = node.parentRuntimeNodeId
    ? nodes.find((candidate) => candidate.runtimeNodeId === node.parentRuntimeNodeId)
    : undefined
  const value = `${parent?.kind ?? ''} ${parent?.className ?? ''}`.toLowerCase()
  if (value.includes('column')) return 'column'
  if (value.includes('row')) return 'row'
  if (value.includes('constraint')) return 'constraint'
  if (value.includes('grid')) return 'grid'
  if (value.includes('lazy')) return 'lazy-list'
  if (value.includes('linear')) return 'linear'
  if (value.includes('frame') || value.includes('box')) return 'box'
  return parent?.kind || undefined
}

function editableSolverProperties(node: LiveUiNode) {
  return Object.entries(node.properties)
    .filter(([property, snapshot]) => (
      SOLVER_PROPERTIES.has(property)
      && snapshot.changeLevel === 'LIVE'
      && snapshot.commitMode !== 'READ_ONLY'
    ))
    .map(([property]) => property)
}
