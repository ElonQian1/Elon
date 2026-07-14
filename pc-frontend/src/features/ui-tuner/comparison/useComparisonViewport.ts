import { useCallback, useEffect, useRef, useState, type UIEventHandler } from 'react'
import type { PixelSize } from './types'
import { canvasZoomCommand } from './canvasZoomShortcuts'

const MIN_SCALE = 0.08
const MAX_SCALE = 2
const SCALE_STEP = 0.1

type FitMode = 'stage' | 'width' | 'manual'

function normalizeScale(value: number) {
  if (!Number.isFinite(value)) return 1
  return Math.round(Math.min(Math.max(value, MIN_SCALE), MAX_SCALE) * 100) / 100
}

function availableViewport(scroller: HTMLDivElement) {
  const style = window.getComputedStyle(scroller)
  return {
    width: Math.max(
      scroller.clientWidth - Number.parseFloat(style.paddingLeft) - Number.parseFloat(style.paddingRight),
      24,
    ),
    height: Math.max(
      scroller.clientHeight - Number.parseFloat(style.paddingTop) - Number.parseFloat(style.paddingBottom),
      24,
    ),
  }
}

function fitScale(scroller: HTMLDivElement | null, size: PixelSize | null, mode: Exclude<FitMode, 'manual'>) {
  if (!scroller || !size || size.width <= 0 || size.height <= 0) return null
  const viewport = availableViewport(scroller)
  const widthScale = viewport.width / size.width
  return mode === 'width'
    ? Math.min(widthScale, 1)
    : Math.min(widthScale, viewport.height / size.height, 1)
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
  const [fitMode, setFitMode] = useState<FitMode>('stage')
  const targetScrollerRef = useRef<HTMLDivElement | null>(null)
  const currentScrollerRef = useRef<HTMLDivElement | null>(null)
  const syncingRef = useRef(false)

  const applyFit = useCallback((mode: Exclude<FitMode, 'manual'>) => {
    const candidates = [
      fitScale(currentScrollerRef.current, currentSize, mode),
      fitScale(targetScrollerRef.current, targetSize, mode),
    ].filter((value): value is number => value !== null)
    if (candidates.length > 0) setViewScale(normalizeScale(Math.min(...candidates)))
  }, [currentSize.height, currentSize.width, targetSize?.height, targetSize?.width])

  useEffect(() => {
    if (fitMode === 'manual') return undefined
    applyFit(fitMode)
    const elements = [targetScrollerRef.current, currentScrollerRef.current].filter(
      (value): value is HTMLDivElement => Boolean(value),
    )
    if (typeof ResizeObserver === 'undefined') {
      const refit = () => applyFit(fitMode)
      window.addEventListener('resize', refit)
      return () => window.removeEventListener('resize', refit)
    }
    const observer = new ResizeObserver(() => applyFit(fitMode))
    elements.forEach((element) => observer.observe(element))
    return () => observer.disconnect()
  }, [applyFit, fitMode])

  const setManualScale = useCallback((value: number) => {
    setFitMode('manual')
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
    setFitMode('stage')
    applyFit('stage')
  }, [applyFit])
  const fitWidth = useCallback(() => {
    setFitMode('width')
    applyFit('width')
  }, [applyFit])
  const requestFit = useCallback(() => setFitMode('stage'), [])

  useEffect(() => {
    const handleShortcut = (event: KeyboardEvent) => {
      const command = canvasZoomCommand(event)
      if (!command) return
      event.preventDefault()
      if (command === 'zoom-in') zoomIn()
      else if (command === 'zoom-out') zoomOut()
      else actualSize()
    }
    window.addEventListener('keydown', handleShortcut)
    return () => window.removeEventListener('keydown', handleShortcut)
  }, [actualSize, zoomIn, zoomOut])

  return {
    viewScale,
    viewScaleLabel: `${Math.round(viewScale * 100)}%`,
    fitToStage: fitMode === 'stage',
    fitToWidth: fitMode === 'width',
    targetScrollerRef,
    currentScrollerRef,
    onTargetScroll,
    onCurrentScroll,
    zoomOut,
    zoomIn,
    actualSize,
    fitCanvasToStage: fit,
    fitCanvasToWidth: fitWidth,
    requestFit,
  }
}
