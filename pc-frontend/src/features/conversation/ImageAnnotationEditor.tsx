import { useEffect, useMemo, useRef, useState } from 'react'
import type { CSSProperties, KeyboardEvent, PointerEvent } from 'react'
import { ArrowUpRight, Check, Image, Pencil, Redo2, Square, Trash2, Type, Undo2, X } from 'lucide-react'
import {
  annotationHasMeaningfulSize,
  annotationId,
  clampUnit,
  drawScene,
  exportAnnotatedImage,
  fitCanvasSize,
  minCanvasSide,
  textFontSizeForCanvas,
} from './imageAnnotationCanvas'
import type {
  AnnotationTool,
  DrawingDraft,
  ImageAnnotation,
  ImageInfo,
  NormalizedPoint,
} from './imageAnnotationCanvas'
import styles from './ImageAnnotationEditor.module.css'

interface ImageAnnotationEditorProps {
  file: File
  queueIndex: number
  queueCount: number
  uploading: boolean
  error?: string
  onApply: (file: File) => Promise<void> | void
  onSendOriginal: () => Promise<void> | void
  onDiscard: () => void
}

interface TextDraft {
  id: string
  point: NormalizedPoint
  value: string
  color: string
  sizeRatio: number
}

interface AnnotationTimeline {
  items: ImageAnnotation[]
  past: ImageAnnotation[][]
  future: ImageAnnotation[][]
}

const COLORS = ['#ff5c5c', '#ffd166', '#25d366', '#5aa2ff', '#ffffff', '#111111']
const TOOLS: Array<{ id: AnnotationTool; title: string; icon: typeof Pencil }> = [
  { id: 'pen', title: '画笔', icon: Pencil },
  { id: 'arrow', title: '箭头', icon: ArrowUpRight },
  { id: 'rect', title: '矩形', icon: Square },
  { id: 'text', title: '文字', icon: Type },
]

const HISTORY_LIMIT = 80

function emptyTimeline(): AnnotationTimeline {
  return { items: [], past: [], future: [] }
}

function trimHistory(items: ImageAnnotation[][]) {
  return items.length > HISTORY_LIMIT ? items.slice(items.length - HISTORY_LIMIT) : items
}

export default function ImageAnnotationEditor({
  file,
  queueIndex,
  queueCount,
  uploading,
  error,
  onApply,
  onSendOriginal,
  onDiscard,
}: ImageAnnotationEditorProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const draftRef = useRef<DrawingDraft | null>(null)
  const textAreaRef = useRef<HTMLTextAreaElement>(null)
  const [viewport, setViewport] = useState(() => ({ width: window.innerWidth, height: window.innerHeight }))
  const [imageInfo, setImageInfo] = useState<ImageInfo | null>(null)
  const [loadError, setLoadError] = useState('')
  const [timeline, setTimeline] = useState<AnnotationTimeline>(() => emptyTimeline())
  const [draft, setDraft] = useState<DrawingDraft | null>(null)
  const [textDraft, setTextDraft] = useState<TextDraft | null>(null)
  const [tool, setTool] = useState<AnnotationTool>('pen')
  const [color, setColor] = useState(COLORS[0])
  const [strokeSize, setStrokeSize] = useState(5)
  const [busyAction, setBusyAction] = useState('')

  const annotations = timeline.items
  const canUndo = timeline.past.length > 0
  const canRedo = timeline.future.length > 0

  const canvasSize = useMemo(() => {
    if (!imageInfo) return null
    return fitCanvasSize(imageInfo.width, imageInfo.height, viewport.width, viewport.height)
  }, [imageInfo, viewport])

  const busy = uploading || Boolean(busyAction)
  const currentError = loadError || error || ''

  useEffect(() => {
    function onResize() {
      setViewport({ width: window.innerWidth, height: window.innerHeight })
    }
    window.addEventListener('resize', onResize)
    return () => window.removeEventListener('resize', onResize)
  }, [])

  useEffect(() => {
    setTimeline(emptyTimeline())
    setDraftState(null)
    setTextDraft(null)
    setLoadError('')
    const url = URL.createObjectURL(file)
    const imageElement = new window.Image()
    imageElement.onload = () => {
      setImageInfo({
        element: imageElement,
        width: imageElement.naturalWidth || imageElement.width,
        height: imageElement.naturalHeight || imageElement.height,
      })
    }
    imageElement.onerror = () => {
      setLoadError('图片加载失败')
      setImageInfo(null)
    }
    imageElement.src = url
    return () => URL.revokeObjectURL(url)
  }, [file])

  useEffect(() => {
    if (!textDraft || busy) return
    const timer = window.setTimeout(() => {
      textAreaRef.current?.focus()
    }, 0)
    return () => window.clearTimeout(timer)
  }, [busy, textDraft?.id])

  useEffect(() => {
    if (!imageInfo || !canvasSize) return
    const canvas = canvasRef.current
    const context = canvas?.getContext('2d')
    if (!canvas || !context) return
    drawScene(context, imageInfo.element, canvasSize.width, canvasSize.height, annotations, draft)
  }, [annotations, canvasSize, draft, imageInfo])

  function setDraftState(next: DrawingDraft | null) {
    draftRef.current = next
    setDraft(next)
  }

  function commitAnnotations(nextItems: ImageAnnotation[]) {
    setTimeline((current) => ({
      items: nextItems,
      past: trimHistory(current.past.concat([current.items])),
      future: [],
    }))
  }

  function addAnnotation(annotation: ImageAnnotation) {
    setTimeline((current) => ({
      items: current.items.concat(annotation),
      past: trimHistory(current.past.concat([current.items])),
      future: [],
    }))
  }

  function undoAnnotations() {
    setTimeline((current) => {
      if (current.past.length === 0) return current
      const previous = current.past[current.past.length - 1]
      return {
        items: previous,
        past: current.past.slice(0, -1),
        future: [current.items, ...current.future].slice(0, HISTORY_LIMIT),
      }
    })
  }

  function redoAnnotations() {
    setTimeline((current) => {
      if (current.future.length === 0) return current
      const next = current.future[0]
      return {
        items: next,
        past: trimHistory(current.past.concat([current.items])),
        future: current.future.slice(1),
      }
    })
  }

  function clearAnnotations() {
    if (annotations.length === 0) return
    commitAnnotations([])
  }

  function textAnnotationFromDraft(current: TextDraft): ImageAnnotation | null {
    const text = current.value.trim()
    if (!text) return null
    return {
      id: current.id,
      tool: 'text',
      point: current.point,
      text,
      color: current.color,
      sizeRatio: current.sizeRatio,
    }
  }

  function finishTextDraft() {
    if (!textDraft) return
    const annotation = textAnnotationFromDraft(textDraft)
    setTextDraft(null)
    if (annotation) addAnnotation(annotation)
  }

  function handleDialogPointerDownCapture(event: PointerEvent<HTMLElement>) {
    if (!textDraft) return
    const target = event.target
    if (!(target instanceof Node)) return
    if (textAreaRef.current?.contains(target)) return
    if (canvasRef.current?.contains(target)) return
    finishTextDraft()
  }

  function handlePointerDown(event: PointerEvent<HTMLCanvasElement>) {
    if (busy || !canvasSize || !imageInfo) return
    event.preventDefault()
    const point = pointFromEvent(event)
    if (tool === 'text') {
      finishTextDraft()
      setTextDraft({
        id: annotationId(),
        point,
        value: '',
        color,
        sizeRatio: strokeSize / minCanvasSide(canvasSize),
      })
      return
    }

    finishTextDraft()
    event.currentTarget.setPointerCapture(event.pointerId)
    const sizeRatio = strokeSize / minCanvasSide(canvasSize)
    const nextDraft: DrawingDraft = tool === 'pen'
      ? { id: annotationId(), tool: 'pen', points: [point], color, sizeRatio }
      : { id: annotationId(), tool, start: point, end: point, color, sizeRatio }
    setDraftState(nextDraft)
  }

  function handlePointerMove(event: PointerEvent<HTMLCanvasElement>) {
    if (busy || !draftRef.current) return
    event.preventDefault()
    const point = pointFromEvent(event)
    const current = draftRef.current
    const nextDraft: DrawingDraft = current.tool === 'pen'
      ? { ...current, points: current.points.concat(point) }
      : { ...current, end: point }
    setDraftState(nextDraft)
  }

  function handlePointerUp(event: PointerEvent<HTMLCanvasElement>) {
    const current = draftRef.current
    if (!current) return
    event.preventDefault()
    try {
      event.currentTarget.releasePointerCapture(event.pointerId)
    } catch {
      // Pointer capture may already be released by the browser.
    }
    setDraftState(null)
    if (!annotationHasMeaningfulSize(current)) return
    addAnnotation(current)
  }

  function handleTextDraftKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault()
      finishTextDraft()
    } else if (event.key === 'Escape') {
      event.preventDefault()
      setTextDraft(null)
    }
  }

  async function applyEditedImage() {
    if (!imageInfo || busy) return
    setBusyAction('apply')
    try {
      const pendingText = textDraft ? textAnnotationFromDraft(textDraft) : null
      const finalAnnotations = pendingText ? annotations.concat(pendingText) : annotations
      setTextDraft(null)
      const editedFile = await exportAnnotatedImage(file, imageInfo.element, finalAnnotations)
      await onApply(editedFile)
    } finally {
      setBusyAction('')
    }
  }

  async function sendOriginalImage() {
    if (busy) return
    setBusyAction('original')
    try {
      await onSendOriginal()
    } finally {
      setBusyAction('')
    }
  }

  return (
    <div className={styles.backdrop} role="presentation">
      <section
        className={styles.dialog}
        role="dialog"
        aria-modal="true"
        aria-label="编辑图片"
        onPointerDownCapture={handleDialogPointerDownCapture}
      >
        <header className={styles.header}>
          <div className={styles.title}>
            <Image size={18} aria-hidden="true" />
            <div>
              <strong>编辑图片</strong>
              <span title={file.name}>{queueCount > 1 ? `${queueIndex + 1}/${queueCount} · ` : ''}{file.name}</span>
            </div>
          </div>
          <button className={styles.iconBtn} type="button" title="取消" aria-label="取消编辑" onClick={onDiscard} disabled={busy}>
            <X size={18} aria-hidden="true" />
          </button>
        </header>

        <div className={styles.body}>
          <aside className={styles.toolbar}>
            <div className={styles.toolGroup}>
              {TOOLS.map((item) => {
                const Icon = item.icon
                return (
                  <button
                    key={item.id}
                    className={styles.toolBtn}
                    data-active={tool === item.id ? 'true' : 'false'}
                    type="button"
                    title={item.title}
                    aria-label={item.title}
                    aria-pressed={tool === item.id}
                    onClick={() => setTool(item.id)}
                    disabled={busy}
                  >
                    <Icon size={18} aria-hidden="true" />
                  </button>
                )
              })}
            </div>

            <div className={styles.swatches} aria-label="颜色">
              {COLORS.map((item) => (
                <button
                  key={item}
                  className={styles.swatch}
                  data-active={color === item ? 'true' : 'false'}
                  type="button"
                  title={item}
                  aria-label={`颜色 ${item}`}
                  style={{ '--swatch-color': item } as CSSProperties}
                  onClick={() => setColor(item)}
                  disabled={busy}
                />
              ))}
            </div>

            <label className={styles.sizeControl} title="粗细">
              <input
                type="range"
                min="3"
                max="18"
                value={strokeSize}
                onChange={(event) => setStrokeSize(Number(event.target.value))}
                disabled={busy}
              />
            </label>

            <div className={styles.toolGroup}>
              <button
                className={styles.toolBtn}
                type="button"
                title="撤销"
                aria-label="撤销"
                onClick={undoAnnotations}
                disabled={busy || !canUndo}
              >
                <Undo2 size={18} aria-hidden="true" />
              </button>
              <button
                className={styles.toolBtn}
                type="button"
                title="前进"
                aria-label="前进"
                onClick={redoAnnotations}
                disabled={busy || !canRedo}
              >
                <Redo2 size={18} aria-hidden="true" />
              </button>
              <button
                className={styles.toolBtn}
                type="button"
                title="清空"
                aria-label="清空"
                onClick={clearAnnotations}
                disabled={busy || annotations.length === 0}
              >
                <Trash2 size={18} aria-hidden="true" />
              </button>
            </div>
          </aside>

          <main className={styles.stageWrap}>
            {canvasSize && imageInfo ? (
              <div className={styles.canvasLayer} style={{ width: canvasSize.width, height: canvasSize.height }}>
                <canvas
                  ref={canvasRef}
                  className={styles.canvas}
                  width={canvasSize.width}
                  height={canvasSize.height}
                  style={{ width: canvasSize.width, height: canvasSize.height }}
                  onPointerDown={handlePointerDown}
                  onPointerMove={handlePointerMove}
                  onPointerUp={handlePointerUp}
                  onPointerCancel={() => setDraftState(null)}
                />
                {textDraft && (
                  <textarea
                    ref={textAreaRef}
                    className={styles.textDraftInput}
                    value={textDraft.value}
                    aria-label="标注文字"
                    rows={1}
                    style={textDraftStyle(textDraft, canvasSize)}
                    onChange={(event) => setTextDraft((current) => current ? { ...current, value: event.target.value } : current)}
                    onKeyDown={handleTextDraftKeyDown}
                    disabled={busy}
                  />
                )}
              </div>
            ) : (
              <div className={styles.loading}>{loadError || '图片加载中'}</div>
            )}
          </main>
        </div>

        <footer className={styles.footer}>
          <div className={styles.errorSlot}>{currentError}</div>
          <div className={styles.actions}>
            <button className={styles.secondaryBtn} type="button" onClick={onDiscard} disabled={busy}>取消</button>
            <button className={styles.secondaryBtn} type="button" onClick={sendOriginalImage} disabled={busy || !imageInfo}>
              发送原图
            </button>
            <button className={styles.primaryBtn} type="button" onClick={applyEditedImage} disabled={busy || !imageInfo}>
              <Check size={16} aria-hidden="true" />
              <span>{busyAction === 'apply' || uploading ? '处理中' : '完成'}</span>
            </button>
          </div>
        </footer>
      </section>
    </div>
  )
}

function textDraftStyle(draft: TextDraft, canvasSize: { width: number; height: number }): CSSProperties {
  const fontSize = textFontSizeForCanvas(draft.sizeRatio, canvasSize.width, canvasSize.height)
  const width = Math.min(360, Math.max(140, canvasSize.width * 0.38))
  const lineCount = Math.max(1, draft.value.split(/\r?\n/).length)
  const height = fontSize * 1.22 * lineCount + 10
  const left = Math.min(draft.point.x * canvasSize.width, Math.max(0, canvasSize.width - width - 8))
  const top = Math.min(draft.point.y * canvasSize.height, Math.max(0, canvasSize.height - height - 8))
  return {
    left,
    top,
    width,
    height,
    color: draft.color,
    fontSize,
    lineHeight: `${fontSize * 1.22}px`,
  }
}

function pointFromEvent(event: PointerEvent<HTMLCanvasElement>): NormalizedPoint {
  const rect = event.currentTarget.getBoundingClientRect()
  return {
    x: clampUnit((event.clientX - rect.left) / rect.width),
    y: clampUnit((event.clientY - rect.top) / rect.height),
  }
}
