import { useCallback, useEffect, useRef, useState, type UIEventHandler } from 'react'
import type { PixelSize } from './types'

const MIN_SCALE = 0.08
const MAX_SCALE = 2
const SCALE_STEP = 0.1
const PANE_PADDING = 40

function normalizeScale(value: number) {
  if (!Number.isFinite(value)) return 1
  return Math.round(Math.min(Math.max(value, MIN_SCALE), MAX_SCALE) * 100) / 100
}

function fitScale(scroller: HTMLDivElement | null, size: PixelSize | null) {
  if (!scroller || !size || size.width <= 0 || size.height <= 0) return null
  return Math.min(
    Math.max(scroller.clientWidth - PANE_PADDING, 24) / size.width,
    Math.max(scroller.clientHeight - PANE_PADDING, 24) / size.height,
    1,
  )
}

function copyNormalizedScroll(source: HTMLDivElement, target: HTMLDivElement) {
  const targetMaxX = Math.max(target.scrollWidth - target.clientWidth, 0)
  const targetMaxY = Math.max(target.scrollHeight - target.clientHeight, 0)
  const centerX = source.scrollWidth > 0
    ? (source.scrollLeft + source.clientWidth / 2) / source.scrollWidth
    : 0.5
  const centerY = source.scrollHeight > 0
    ? (source.scrollTop + source.clientHeight / 2) / source.scrollHeight
    : 0.5
  target.scrollLeft = Math.min(Math.max(centerX * target.scrollWidth - target.clientWidth / 2, 0), targetMaxX)
  target.scrollTop = Math.min(Math.max(centerY * target.scrollHeight - target.clientHeight / 2, 0), targetMaxY)
}

export function useComparisonViewport(currentSize: PixelSize, targetSize: PixelSize | null) {
  const [viewScale, setViewScale] = useState(1)
  const [fitToStage, setFitToStage] = useState(true)
  const targetScrollerRef = useRef<HTMLDivElement | null>(null)
  const currentScrollerRef = useRef<HTMLDivElement | null>(null)
  const syncingRef = useRef(false)

  const fitCanvasToStage = useCallback(() => {
    const candidates = [
      fitScale(currentScrollerRef.current, currentSize),
      fitScale(targetScrollerRef.current, targetSize),
    ].filter((value): value is number => value !== null)
    if (candidates.length > 0) setViewScale(normalizeScale(Math.min(...candidates)))
  }, [currentSize.height, currentSize.width, targetSize?.height, targetSize?.width])

  useEffect(() => {
    if (!fitToStage) return undefined
    fitCanvasToStage()
    const elements = [targetScrollerRef.current, currentScrollerRef.current].filter(
      (value): value is HTMLDivElement => Boolean(value),
    )
    if (typeof ResizeObserver === 'undefined') {
      window.addEventListener('resize', fitCanvasToStage)
      return () => window.removeEventListener('resize', fitCanvasToStage)
    }
    const observer = new ResizeObserver(fitCanvasToStage)
    elements.forEach((element) => observer.observe(element))
    return () => observer.disconnect()
  }, [fitCanvasToStage, fitToStage])

  const setManualScale = useCallback((value: number) => {
    setFitToStage(false)
    setViewScale(normalizeScale(value))
  }, [])

  const synchronize = useCallback((source: HTMLDivElement, target: HTMLDivElement | null) => {
    if (!target || syncingRef.current) return
    syncingRef.current = true
    copyNormalizedScroll(source, target)
    window.requestAnimationFrame(() => { syncingRef.current = false })
  }, [])

  const onTargetScroll: UIEventHandler<HTMLDivElement> = useCallback((event) => {
    synchronize(event.currentTarget, currentScrollerRef.current)
  }, [synchronize])
  const onCurrentScroll: UIEventHandler<HTMLDivElement> = useCallback((event) => {
    synchronize(event.currentTarget, targetScrollerRef.current)
  }, [synchronize])

  useEffect(() => {
    const current = currentScrollerRef.current
    const target = targetScrollerRef.current
    if (!current || !target) return undefined
    const frame = window.requestAnimationFrame(() => copyNormalizedScroll(current, target))
    return () => window.cancelAnimationFrame(frame)
  }, [viewScale])

  const zoomOut = useCallback(() => setManualScale(viewScale - SCALE_STEP), [setManualScale, viewScale])
  const zoomIn = useCallback(() => setManualScale(viewScale + SCALE_STEP), [setManualScale, viewScale])
  const actualSize = useCallback(() => setManualScale(1), [setManualScale])
  const fit = useCallback(() => {
    setFitToStage(true)
    fitCanvasToStage()
  }, [fitCanvasToStage])
  const requestFit = useCallback(() => setFitToStage(true), [])

  return {
    viewScale,
    viewScaleLabel: `${Math.round(viewScale * 100)}%`,
    fitToStage,
    targetScrollerRef,
    currentScrollerRef,
    onTargetScroll,
    onCurrentScroll,
    zoomOut,
    zoomIn,
    actualSize,
    fitCanvasToStage: fit,
    requestFit,
  }
}
