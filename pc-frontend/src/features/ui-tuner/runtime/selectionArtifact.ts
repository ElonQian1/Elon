import {
  persistAndroidSelectionArtifact,
  type AndroidSelectionArtifact,
} from '../device/deviceInspectorApi'
import type { UiTunerDocument, UiTunerElement } from '../types'

const CROP_PADDING = 24
const MAX_CROP_EDGE = 1200

export interface UiTunerSelectionVisualContext {
  previewDataUrl: string
  artifact?: AndroidSelectionArtifact
  error?: string
}

export async function buildSelectionVisualContext(
  document: UiTunerDocument,
  element: UiTunerElement,
): Promise<UiTunerSelectionVisualContext | null> {
  const reference = document.canvas.referenceImage
  if (!reference?.dataUrl) return null
  try {
    const previewDataUrl = await cropUiTunerSelectionPreview(document, element)
    const snapshotId = document.runtimeSnapshot?.artifact?.id
    if (!snapshotId) return { previewDataUrl }
    const artifact = await persistAndroidSelectionArtifact({
      snapshotId,
      selectionId: element.id,
      cropDataUrl: previewDataUrl,
      bounds: {
        left: Math.round(element.x),
        top: Math.round(element.y),
        right: Math.round(element.x + element.width),
        bottom: Math.round(element.y + element.height),
        width: Math.round(element.width),
        height: Math.round(element.height),
      },
      resourceId: element.runtime?.resourceId,
      componentKey: element.source?.componentKey,
    })
    return { previewDataUrl, artifact }
  } catch (error) {
    return {
      previewDataUrl: '',
      error: error instanceof Error ? error.message : '无法生成选区截图',
    }
  }
}

export async function cropUiTunerSelectionPreview(
  document: UiTunerDocument,
  element: UiTunerElement,
): Promise<string> {
  const dataUrl = document.canvas.referenceImage?.dataUrl
  if (!dataUrl) throw new Error('当前画布没有可用于验收的真机截图')
  const image = await loadImage(dataUrl)
  const left = clamp(Math.floor(element.x - CROP_PADDING), 0, image.naturalWidth)
  const top = clamp(Math.floor(element.y - CROP_PADDING), 0, image.naturalHeight)
  const right = clamp(Math.ceil(element.x + element.width + CROP_PADDING), left + 1, image.naturalWidth)
  const bottom = clamp(Math.ceil(element.y + element.height + CROP_PADDING), top + 1, image.naturalHeight)
  const sourceWidth = right - left
  const sourceHeight = bottom - top
  const scale = Math.min(1, MAX_CROP_EDGE / Math.max(sourceWidth, sourceHeight))
  const canvas = window.document.createElement('canvas')
  canvas.width = Math.max(1, Math.round(sourceWidth * scale))
  canvas.height = Math.max(1, Math.round(sourceHeight * scale))
  const context = canvas.getContext('2d')
  if (!context) throw new Error('浏览器不支持选区截图')
  context.drawImage(
    image,
    left,
    top,
    sourceWidth,
    sourceHeight,
    0,
    0,
    canvas.width,
    canvas.height,
  )
  context.strokeStyle = '#4caf78'
  context.lineWidth = Math.max(2, Math.round(3 * scale))
  context.strokeRect(
    (element.x - left) * scale,
    (element.y - top) * scale,
    element.width * scale,
    element.height * scale,
  )
  return canvas.toDataURL('image/png')
}

function loadImage(dataUrl: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image()
    image.onload = () => resolve(image)
    image.onerror = () => reject(new Error('真机截图无法读取'))
    image.src = dataUrl
  })
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max)
}
