import { useEffect, useMemo, useRef, useState } from 'react'
import type { LiveUiNode } from '../live/liveUiApi'
import type { UiTunerDocument, UiTunerElement } from '../types'
import {
  createCalibration,
  hasUsableArea,
  mapTargetRectWithCalibration,
  rectEquals,
} from './comparisonGeometry'
import {
  comparisonStorageKey,
  loadComparisonState,
  saveComparisonState,
  targetDesignId,
} from './comparisonStorage'
import type {
  ComparisonMode,
  ComparisonWorkspaceState,
  PixelRect,
  TargetCurrentPair,
  PixelSize,
  ComparisonCalibration,
} from './types'

function defaultState(
  designId: string,
  targetSize: PixelSize,
  currentSize: PixelSize,
): ComparisonWorkspaceState {
  return {
    version: 1,
    targetDesignId: designId,
    mode: 'split',
    overlayOpacity: 0.5,
    targetRect: null,
    pair: null,
    calibration: createCalibration(targetSize, currentSize),
  }
}

function currentRect(element: UiTunerElement): PixelRect {
  return {
    left: Math.round(element.x),
    top: Math.round(element.y),
    right: Math.round(element.x + element.width),
    bottom: Math.round(element.y + element.height),
  }
}

function samePair(left: TargetCurrentPair | null, right: TargetCurrentPair) {
  return Boolean(left
    && left.targetDesignId === right.targetDesignId
    && left.runtimeNodeId === right.runtimeNodeId
    && left.definitionId === right.definitionId
    && left.calibrationId === right.calibrationId
    && rectEquals(left.targetRect, right.targetRect)
    && rectEquals(left.projectedTargetRect, right.projectedTargetRect))
}

interface UseComparisonWorkspaceOptions {
  document: UiTunerDocument
  selected: UiTunerElement | null
  liveNode?: LiveUiNode | null
  currentSize: PixelSize
  onPairChange?: (pair: TargetCurrentPair | null) => void
}

export function useComparisonWorkspace({
  document,
  selected,
  liveNode,
  currentSize,
  onPairChange,
}: UseComparisonWorkspaceOptions) {
  const designId = useMemo(() => targetDesignId(document.canvas.targetDesign), [document.canvas.targetDesign])
  const targetSize = useMemo(() => ({
    width: document.canvas.targetDesign?.width ?? document.canvas.width,
    height: document.canvas.targetDesign?.height ?? document.canvas.height,
  }), [document.canvas.height, document.canvas.targetDesign, document.canvas.width])
  const storageKey = useMemo(() => comparisonStorageKey(document), [
    designId,
    document.runtimeSnapshot?.packageName,
    document.runtimeSnapshot?.sourceFingerprint,
    document.source?.signature,
  ])
  const [state, setState] = useState<ComparisonWorkspaceState>(() => (
    loadComparisonState(storageKey, designId) ?? defaultState(designId, targetSize, currentSize)
  ))
  const activeStorageKeyRef = useRef(storageKey)
  const onPairChangeRef = useRef(onPairChange)

  useEffect(() => {
    onPairChangeRef.current = onPairChange
  }, [onPairChange])

  useEffect(() => {
    if (activeStorageKeyRef.current === storageKey) return
    activeStorageKeyRef.current = storageKey
    setState(loadComparisonState(storageKey, designId) ?? defaultState(designId, targetSize, currentSize))
  }, [currentSize, designId, storageKey, targetSize])

  useEffect(() => {
    const calibration = state.calibration
    if (calibration
      && calibration.targetSize.width === targetSize.width
      && calibration.targetSize.height === targetSize.height
      && calibration.currentSize.width === currentSize.width
      && calibration.currentSize.height === currentSize.height) return
    setState((current) => ({
      ...current,
      calibration: createCalibration(targetSize, currentSize),
      pair: null,
    }))
  }, [
    currentSize.height,
    currentSize.width,
    state.calibration,
    targetSize.height,
    targetSize.width,
  ])

  useEffect(() => {
    if (state.targetDesignId !== designId) return
    saveComparisonState(storageKey, state)
  }, [designId, state, storageKey])

  useEffect(() => {
    if (state.targetDesignId !== designId) return
    const runtime = selected?.runtime
    if ((!runtime && !liveNode) || !selected || !hasUsableArea(state.targetRect)) return
    const runtimeNodeId = liveNode?.runtimeNodeId ?? runtime!.nodeId
    const definitionId = liveNode?.definitionId ?? runtime!.xpath
    const liveBounds = liveNode?.geometry.boundsInDisplayPx
    const selectedCurrentRect = liveBounds ? {
      left: liveBounds.left,
      top: liveBounds.top,
      right: liveBounds.right,
      bottom: liveBounds.bottom,
    } : currentRect(selected)
    const now = new Date().toISOString()
    const id = `${designId}:${runtimeNodeId}`
    const calibration = state.calibration ?? createCalibration(targetSize, currentSize)
    const next: TargetCurrentPair = {
      id,
      targetDesignId: designId,
      targetImageName: document.canvas.targetDesign?.name ?? '目标设计图',
      targetRect: state.targetRect!,
      runtimeNodeId,
      definitionId,
      currentRect: selectedCurrentRect,
      projectedTargetRect: mapTargetRectWithCalibration(
        state.targetRect!,
        targetSize,
        currentSize,
        calibration,
      ),
      calibrationId: calibration.id,
      createdAt: state.pair?.id === id ? state.pair.createdAt : now,
      updatedAt: now,
    }
    setState((current) => samePair(current.pair, next) ? current : { ...current, pair: next })
  }, [
    designId,
    document.canvas.targetDesign?.name,
    selected?.height,
    selected?.runtime?.nodeId,
    selected?.runtime?.xpath,
    liveNode?.definitionId,
    liveNode?.geometry.boundsInDisplayPx.bottom,
    liveNode?.geometry.boundsInDisplayPx.left,
    liveNode?.geometry.boundsInDisplayPx.right,
    liveNode?.geometry.boundsInDisplayPx.top,
    liveNode?.runtimeNodeId,
    selected?.width,
    selected?.x,
    selected?.y,
    state.pair?.createdAt,
    state.pair?.id,
    state.targetDesignId,
    state.targetRect,
    state.calibration,
    currentSize,
    targetSize,
  ])

  useEffect(() => {
    onPairChangeRef.current?.(state.pair)
  }, [state.pair])

  const setMode = (mode: ComparisonMode) => setState((current) => ({ ...current, mode }))
  const setOverlayOpacity = (overlayOpacity: number) => setState((current) => ({
    ...current,
    overlayOpacity: Math.min(Math.max(overlayOpacity, 0), 1),
  }))
  const setTargetRect = (targetRect: PixelRect) => setState((current) => ({
    ...current,
    targetRect,
    pair: null,
  }))
  const clearPair = () => setState((current) => ({ ...current, targetRect: null, pair: null }))
  const setCalibration = (calibration: ComparisonCalibration) => setState((current) => ({
    ...current,
    calibration,
    pair: null,
  }))

  return {
    ...state,
    setMode,
    setOverlayOpacity,
    setTargetRect,
    clearPair,
    setCalibration,
  }
}
