export type ComparisonMode = 'split' | 'overlay' | 'blink' | 'diff'

export interface PixelSize {
  width: number
  height: number
}

export interface PixelRect {
  left: number
  top: number
  right: number
  bottom: number
}

export interface TargetCurrentPair {
  id: string
  targetDesignId: string
  targetImageName: string
  targetRect: PixelRect
  runtimeNodeId: string
  definitionId: string
  currentRect: PixelRect
  projectedTargetRect: PixelRect
  calibrationId: string
  createdAt: string
  updatedAt: string
}

export interface ComparisonWorkspaceState {
  version: 1
  targetDesignId: string
  mode: ComparisonMode
  overlayOpacity: number
  targetRect: PixelRect | null
  pair: TargetCurrentPair | null
  calibration: ComparisonCalibration | null
}

export interface ComparisonCalibration {
  id: string
  targetSize: PixelSize
  currentSize: PixelSize
  targetContentRect: PixelRect
  currentContentRect: PixelRect
}

export interface ContainTransform {
  left: number
  top: number
  width: number
  height: number
  scale: number
}
