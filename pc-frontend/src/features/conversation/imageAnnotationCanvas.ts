export type AnnotationTool = 'pen' | 'arrow' | 'rect' | 'text'

export interface NormalizedPoint {
  x: number
  y: number
}

interface BaseAnnotation {
  id: string
  color: string
  sizeRatio: number
}

export interface PenAnnotation extends BaseAnnotation {
  tool: 'pen'
  points: NormalizedPoint[]
}

export interface ShapeAnnotation extends BaseAnnotation {
  tool: 'arrow' | 'rect'
  start: NormalizedPoint
  end: NormalizedPoint
}

export interface TextAnnotation extends BaseAnnotation {
  tool: 'text'
  point: NormalizedPoint
  text: string
}

export type ImageAnnotation = PenAnnotation | ShapeAnnotation | TextAnnotation
export type DrawingDraft = PenAnnotation | ShapeAnnotation

export interface ImageInfo {
  element: HTMLImageElement
  width: number
  height: number
}

export function fitCanvasSize(imageWidth: number, imageHeight: number, viewportWidth: number, viewportHeight: number) {
  const maxWidth = Math.max(280, Math.min(980, viewportWidth - 150))
  const maxHeight = Math.max(240, Math.min(640, viewportHeight - 210))
  const scale = Math.min(maxWidth / imageWidth, maxHeight / imageHeight, 1)
  return {
    width: Math.max(1, Math.round(imageWidth * scale)),
    height: Math.max(1, Math.round(imageHeight * scale)),
  }
}

export function minCanvasSide(size: { width: number; height: number }) {
  return Math.max(1, Math.min(size.width, size.height))
}

export function textFontSizeForCanvas(sizeRatio: number, width: number, height: number) {
  return Math.max(14, sizeRatio * Math.min(width, height) * 4)
}

export function clampUnit(value: number) {
  return Math.max(0, Math.min(1, value))
}

export function annotationId() {
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`
}

export function annotationHasMeaningfulSize(annotation: DrawingDraft) {
  if (annotation.tool === 'pen') return annotation.points.length > 1
  const dx = annotation.end.x - annotation.start.x
  const dy = annotation.end.y - annotation.start.y
  return Math.sqrt(dx * dx + dy * dy) > 0.008
}

export function drawScene(
  context: CanvasRenderingContext2D,
  image: HTMLImageElement,
  width: number,
  height: number,
  annotations: ImageAnnotation[],
  draft: DrawingDraft | null,
) {
  context.clearRect(0, 0, width, height)
  context.drawImage(image, 0, 0, width, height)
  for (const annotation of annotations) drawAnnotation(context, annotation, width, height)
  if (draft) drawAnnotation(context, draft, width, height)
}

export async function exportAnnotatedImage(file: File, image: HTMLImageElement, annotations: ImageAnnotation[]) {
  const canvas = document.createElement('canvas')
  canvas.width = image.naturalWidth || image.width
  canvas.height = image.naturalHeight || image.height
  const context = canvas.getContext('2d')
  if (!context) throw new Error('无法创建图片画布')
  drawScene(context, image, canvas.width, canvas.height, annotations, null)
  const mimeType = exportMimeType(file)
  const blob = await canvasToBlob(canvas, mimeType)
  return new File([blob], editedFileName(file.name, mimeType), { type: mimeType, lastModified: Date.now() })
}

function drawAnnotation(
  context: CanvasRenderingContext2D,
  annotation: ImageAnnotation | DrawingDraft,
  width: number,
  height: number,
) {
  const strokeWidth = Math.max(2, annotation.sizeRatio * Math.min(width, height))
  context.save()
  context.lineWidth = strokeWidth
  context.lineCap = 'round'
  context.lineJoin = 'round'
  context.strokeStyle = annotation.color
  context.fillStyle = annotation.color

  if (annotation.tool === 'pen') {
    drawPen(context, annotation.points, width, height)
  } else if (annotation.tool === 'arrow') {
    drawArrow(context, annotation.start, annotation.end, width, height, strokeWidth)
  } else if (annotation.tool === 'rect') {
    drawRect(context, annotation.start, annotation.end, width, height)
  } else if (annotation.tool === 'text') {
    drawText(context, annotation, width, height)
  }

  context.restore()
}

function drawPen(context: CanvasRenderingContext2D, points: NormalizedPoint[], width: number, height: number) {
  if (points.length < 2) return
  context.beginPath()
  points.forEach((point, index) => {
    const x = point.x * width
    const y = point.y * height
    if (index === 0) context.moveTo(x, y)
    else context.lineTo(x, y)
  })
  context.stroke()
}

function drawArrow(
  context: CanvasRenderingContext2D,
  start: NormalizedPoint,
  end: NormalizedPoint,
  width: number,
  height: number,
  strokeWidth: number,
) {
  const startX = start.x * width
  const startY = start.y * height
  const endX = end.x * width
  const endY = end.y * height
  const angle = Math.atan2(endY - startY, endX - startX)
  const headLength = Math.max(12, strokeWidth * 4.2)
  context.beginPath()
  context.moveTo(startX, startY)
  context.lineTo(endX, endY)
  context.stroke()
  context.beginPath()
  context.moveTo(endX, endY)
  context.lineTo(endX - headLength * Math.cos(angle - Math.PI / 6), endY - headLength * Math.sin(angle - Math.PI / 6))
  context.lineTo(endX - headLength * Math.cos(angle + Math.PI / 6), endY - headLength * Math.sin(angle + Math.PI / 6))
  context.closePath()
  context.fill()
}

function drawRect(context: CanvasRenderingContext2D, start: NormalizedPoint, end: NormalizedPoint, width: number, height: number) {
  const x = Math.min(start.x, end.x) * width
  const y = Math.min(start.y, end.y) * height
  const rectWidth = Math.abs(end.x - start.x) * width
  const rectHeight = Math.abs(end.y - start.y) * height
  context.strokeRect(x, y, rectWidth, rectHeight)
}

function drawText(context: CanvasRenderingContext2D, annotation: TextAnnotation, width: number, height: number) {
  const fontSize = textFontSizeForCanvas(annotation.sizeRatio, width, height)
  const x = annotation.point.x * width
  const y = annotation.point.y * height
  const lineHeight = fontSize * 1.22
  const lines = annotation.text.split(/\r?\n/)
  context.font = `800 ${fontSize}px Inter, "Microsoft YaHei", system-ui, sans-serif`
  context.textBaseline = 'top'
  context.lineWidth = Math.max(3, fontSize / 8)
  context.strokeStyle = 'rgba(0, 0, 0, .64)'
  context.fillStyle = annotation.color
  lines.forEach((line, index) => {
    const lineY = y + index * lineHeight
    context.strokeText(line, x, lineY)
    context.fillText(line, x, lineY)
  })
}

function exportMimeType(file: File) {
  if (file.type === 'image/png') return 'image/png'
  if (file.type === 'image/webp') return 'image/webp'
  return 'image/jpeg'
}

function editedFileName(name: string, mimeType: string) {
  const extension = mimeType === 'image/png' ? 'png' : mimeType === 'image/webp' ? 'webp' : 'jpg'
  const base = name.replace(/\.[^.]+$/, '') || 'image'
  return `${base}-edited.${extension}`
}

function canvasToBlob(canvas: HTMLCanvasElement, mimeType: string) {
  return new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (blob) resolve(blob)
      else reject(new Error('图片导出失败'))
    }, mimeType, 0.92)
  })
}
