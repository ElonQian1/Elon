import { useEffect, useState } from 'react'
import type { UiTunerReferenceImage } from '../types'
import { calibratedContainTransform, mapTargetRectWithCalibration } from './comparisonGeometry'
import type { ComparisonCalibration, ComparisonMode, PixelRect, PixelSize } from './types'
import styles from './UiTunerComparisonWorkspace.module.css'

interface ComparisonOverlayLayerProps {
  mode: Exclude<ComparisonMode, 'split'>
  image: UiTunerReferenceImage
  currentSize: PixelSize
  opacity: number
  targetRect: PixelRect | null
  calibration: ComparisonCalibration
}

export function ComparisonOverlayLayer({
  mode,
  image,
  currentSize,
  opacity,
  targetRect,
  calibration,
}: ComparisonOverlayLayerProps) {
  const [blinkTargetVisible, setBlinkTargetVisible] = useState(true)

  useEffect(() => {
    if (mode !== 'blink') {
      setBlinkTargetVisible(true)
      return undefined
    }
    const reducedMotion = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches
    if (reducedMotion) return undefined
    const timer = window.setInterval(() => {
      if (document.visibilityState === 'visible') setBlinkTargetVisible((visible) => !visible)
    }, 650)
    return () => window.clearInterval(timer)
  }, [mode])

  const transform = calibratedContainTransform(
    { width: image.width, height: image.height },
    currentSize,
    calibration,
  )
  const mappedRect = targetRect
    ? mapTargetRectWithCalibration(
        targetRect,
        { width: image.width, height: image.height },
        currentSize,
        calibration,
      )
    : null
  const imageOpacity = mode === 'blink'
    ? (blinkTargetVisible ? 1 : 0)
    : mode === 'diff'
      ? 1
      : opacity

  return (
    <div className={styles.overlayLayer} aria-hidden="true">
      <img
        className={mode === 'diff' ? styles.diffImage : styles.overlayImage}
        src={image.dataUrl}
        alt=""
        style={{
          left: transform.left,
          top: transform.top,
          width: transform.width,
          height: transform.height,
          opacity: imageOpacity,
        }}
        draggable={false}
      />
      {mappedRect && (
        <span
          className={styles.mappedTargetRect}
          style={{
            left: mappedRect.left,
            top: mappedRect.top,
            width: mappedRect.right - mappedRect.left,
            height: mappedRect.bottom - mappedRect.top,
          }}
        />
      )}
    </div>
  )
}
