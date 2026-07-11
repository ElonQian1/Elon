import type {
  ComparisonCalibration,
  ContainTransform,
  PixelRect,
  PixelSize,
} from './types'

export function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(Math.max(value, minimum), maximum)
}

export function rectFromPoints(
  start: { x: number; y: number },
  end: { x: number; y: number },
  bounds: PixelSize,
): PixelRect {
  const left = clamp(Math.min(start.x, end.x), 0, bounds.width)
  const top = clamp(Math.min(start.y, end.y), 0, bounds.height)
  const right = clamp(Math.max(start.x, end.x), 0, bounds.width)
  const bottom = clamp(Math.max(start.y, end.y), 0, bounds.height)
  return {
    left: Math.round(left),
    top: Math.round(top),
    right: Math.round(right),
    bottom: Math.round(bottom),
  }
}

export function normalizeRect(rect: PixelRect, bounds: PixelSize): PixelRect {
  return rectFromPoints(
    { x: rect.left, y: rect.top },
    { x: rect.right, y: rect.bottom },
    bounds,
  )
}

export function hasUsableArea(rect: PixelRect | null, minimumSize = 2) {
  return Boolean(rect
    && rect.right - rect.left >= minimumSize
    && rect.bottom - rect.top >= minimumSize)
}

export function containTransform(source: PixelSize, destination: PixelSize): ContainTransform {
  if (source.width <= 0 || source.height <= 0 || destination.width <= 0 || destination.height <= 0) {
    return { left: 0, top: 0, width: 0, height: 0, scale: 0 }
  }
  const scale = Math.min(destination.width / source.width, destination.height / source.height)
  const width = source.width * scale
  const height = source.height * scale
  return {
    left: (destination.width - width) / 2,
    top: (destination.height - height) / 2,
    width,
    height,
    scale,
  }
}

export function mapTargetRectToDestination(
  rect: PixelRect,
  source: PixelSize,
  destination: PixelSize,
): PixelRect {
  const transform = containTransform(source, destination)
  return {
    left: transform.left + rect.left * transform.scale,
    top: transform.top + rect.top * transform.scale,
    right: transform.left + rect.right * transform.scale,
    bottom: transform.top + rect.bottom * transform.scale,
  }
}

export function fullRect(size: PixelSize): PixelRect {
  return { left: 0, top: 0, right: size.width, bottom: size.height }
}

export function calibratedContainTransform(
  source: PixelSize,
  destination: PixelSize,
  calibration: ComparisonCalibration,
): ContainTransform {
  const target = normalizeRect(calibration.targetContentRect, source)
  const current = normalizeRect(calibration.currentContentRect, destination)
  const targetWidth = Math.max(1, target.right - target.left)
  const targetHeight = Math.max(1, target.bottom - target.top)
  const currentWidth = Math.max(1, current.right - current.left)
  const currentHeight = Math.max(1, current.bottom - current.top)
  const scale = Math.min(currentWidth / targetWidth, currentHeight / targetHeight)
  const mappedWidth = targetWidth * scale
  const mappedHeight = targetHeight * scale
  const contentLeft = current.left + (currentWidth - mappedWidth) / 2
  const contentTop = current.top + (currentHeight - mappedHeight) / 2
  return {
    left: contentLeft - target.left * scale,
    top: contentTop - target.top * scale,
    width: source.width * scale,
    height: source.height * scale,
    scale,
  }
}

export function mapTargetRectWithCalibration(
  rect: PixelRect,
  source: PixelSize,
  destination: PixelSize,
  calibration: ComparisonCalibration,
): PixelRect {
  const transform = calibratedContainTransform(source, destination, calibration)
  return {
    left: Math.round(transform.left + rect.left * transform.scale),
    top: Math.round(transform.top + rect.top * transform.scale),
    right: Math.round(transform.left + rect.right * transform.scale),
    bottom: Math.round(transform.top + rect.bottom * transform.scale),
  }
}

export function createCalibration(
  targetSize: PixelSize,
  currentSize: PixelSize,
  targetContentRect = fullRect(targetSize),
  currentContentRect = fullRect(currentSize),
): ComparisonCalibration {
  const target = normalizeRect(targetContentRect, targetSize)
  const current = normalizeRect(currentContentRect, currentSize)
  const signature = [
    target.left, target.top, target.right, target.bottom,
    current.left, current.top, current.right, current.bottom,
  ].join(':')
  return {
    id: `calibration:${signature}`,
    targetSize: { ...targetSize },
    currentSize: { ...currentSize },
    targetContentRect: target,
    currentContentRect: current,
  }
}

export function clientPointToNaturalPoint(
  client: { x: number; y: number },
  elementBounds: DOMRect,
  naturalSize: PixelSize,
) {
  const scaleX = elementBounds.width > 0 ? naturalSize.width / elementBounds.width : 0
  const scaleY = elementBounds.height > 0 ? naturalSize.height / elementBounds.height : 0
  return {
    x: clamp((client.x - elementBounds.left) * scaleX, 0, naturalSize.width),
    y: clamp((client.y - elementBounds.top) * scaleY, 0, naturalSize.height),
  }
}

export function rectEquals(left: PixelRect | null, right: PixelRect | null) {
  if (left === right) return true
  if (!left || !right) return false
  return left.left === right.left
    && left.top === right.top
    && left.right === right.right
    && left.bottom === right.bottom
}
