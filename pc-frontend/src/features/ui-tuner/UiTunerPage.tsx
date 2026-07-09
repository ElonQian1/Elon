import {
  useEffect,
  useMemo,
  useState,
  type CSSProperties,
  type PointerEvent as ReactPointerEvent,
} from 'react'
import {
  Copy,
  Download,
  MousePointer2,
  Move,
  Plus,
  RefreshCw,
  Save,
  Trash2,
  Type,
} from 'lucide-react'
import { createBlankElement, createInitialTunerDocument } from './presets'
import {
  loadUiTunerDocument,
  saveUiTunerDocument,
  stringifyUiTunerExport,
} from './uiTunerStorage'
import { ColorField, NumberField } from './UiTunerFields'
import { clamp, getMetrics, kindLabel, touch } from './uiTunerGeometry'
import type { UiTunerDocument, UiTunerElement, UiTunerElementKind } from './types'
import styles from './UiTunerPage.module.css'

type DragMode = 'move' | 'resize'

interface DragState {
  id: string
  mode: DragMode
  startX: number
  startY: number
  original: UiTunerElement
}

const MIN_SIZE = 24
const DEFAULT_CANVAS_MAX = 10000

export default function UiTunerPage() {
  const [tunerDoc, setTunerDoc] = useState<UiTunerDocument>(() => (
    loadUiTunerDocument() ?? createInitialTunerDocument()
  ))
  const [selectedId, setSelectedId] = useState<string | null>(() => tunerDoc.elements[0]?.id ?? null)
  const [dragState, setDragState] = useState<DragState | null>(null)
  const [notice, setNotice] = useState('')

  const selected = useMemo(
    () => tunerDoc.elements.find((element) => element.id === selectedId) ?? null,
    [selectedId, tunerDoc.elements],
  )
  const exportJson = useMemo(() => stringifyUiTunerExport(tunerDoc), [tunerDoc])
  const metrics = useMemo(
    () => (selected ? getMetrics(selected, tunerDoc.elements, tunerDoc.canvas) : []),
    [selected, tunerDoc.canvas, tunerDoc.elements],
  )

  useEffect(() => {
    saveUiTunerDocument(tunerDoc)
  }, [tunerDoc])

  useEffect(() => {
    if (!notice) return undefined
    const timer = window.setTimeout(() => setNotice(''), 2200)
    return () => window.clearTimeout(timer)
  }, [notice])

  useEffect(() => {
    if (!dragState) return undefined

    const handlePointerMove = (event: PointerEvent) => {
      event.preventDefault()
      const dx = event.clientX - dragState.startX
      const dy = event.clientY - dragState.startY

      setTunerDoc((current) => {
        const elements = current.elements.map((element) => {
          if (element.id !== dragState.id) return element
          if (dragState.mode === 'move') {
            return {
              ...element,
              x: clamp(dragState.original.x + dx, 0, current.canvas.width - element.width),
              y: clamp(dragState.original.y + dy, 0, current.canvas.height - element.height),
            }
          }
          return {
            ...element,
            width: clamp(dragState.original.width + dx, MIN_SIZE, current.canvas.width - element.x),
            height: clamp(dragState.original.height + dy, MIN_SIZE, current.canvas.height - element.y),
          }
        })
        return touch({ ...current, elements })
      })
    }

    const handlePointerUp = () => setDragState(null)
    window.addEventListener('pointermove', handlePointerMove)
    window.addEventListener('pointerup', handlePointerUp)
    return () => {
      window.removeEventListener('pointermove', handlePointerMove)
      window.removeEventListener('pointerup', handlePointerUp)
    }
  }, [dragState])

  const updateCanvas = (patch: Partial<UiTunerDocument['canvas']>) => {
    setTunerDoc((current) => touch({ ...current, canvas: { ...current.canvas, ...patch } }))
  }

  const updateElement = (id: string, patch: Partial<UiTunerElement>) => {
    setTunerDoc((current) => touch({
      ...current,
      elements: current.elements.map((element) => (
        element.id === id ? { ...element, ...patch } : element
      )),
    }))
  }

  const addElement = (kind: UiTunerElementKind) => {
    const next = createBlankElement(kind, tunerDoc.elements.length + 1)
    setTunerDoc((current) => touch({ ...current, elements: [...current.elements, next] }))
    setSelectedId(next.id)
  }

  const deleteSelected = () => {
    if (!selected) return
    setTunerDoc((current) => touch({
      ...current,
      elements: current.elements.filter((element) => element.id !== selected.id),
    }))
    setSelectedId(null)
  }

  const resetDocument = () => {
    if (!window.confirm('重置后会清空当前微调记录，确定继续吗？')) return
    const next = createInitialTunerDocument()
    setTunerDoc(next)
    setSelectedId(next.elements[0]?.id ?? null)
    setNotice('已恢复默认画布')
  }

  const saveNow = () => {
    saveUiTunerDocument(tunerDoc)
    setNotice('已保存到本机')
  }

  const copyExport = async () => {
    try {
      await navigator.clipboard.writeText(exportJson)
      setNotice('参数 JSON 已复制')
    } catch {
      setNotice('复制失败，可手动选中文本')
    }
  }

  const downloadExport = () => {
    const blob = new Blob([exportJson], { type: 'application/json;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const link = window.document.createElement('a')
    link.href = url
    link.download = 'ui-tuner-adjustments.json'
    link.click()
    URL.revokeObjectURL(url)
  }

  const startDrag = (
    event: ReactPointerEvent<HTMLElement>,
    element: UiTunerElement,
    mode: DragMode,
  ) => {
    if (event.button !== 0) return
    event.stopPropagation()
    event.preventDefault()
    setSelectedId(element.id)
    setDragState({
      id: element.id,
      mode,
      startX: event.clientX,
      startY: event.clientY,
      original: element,
    })
  }

  const handleCanvasKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (!selected) return
    const step = event.shiftKey ? 8 : 1
    if (event.key === 'ArrowLeft') {
      event.preventDefault()
      updateElement(selected.id, { x: clamp(selected.x - step, 0, tunerDoc.canvas.width - selected.width) })
    } else if (event.key === 'ArrowRight') {
      event.preventDefault()
      updateElement(selected.id, { x: clamp(selected.x + step, 0, tunerDoc.canvas.width - selected.width) })
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      updateElement(selected.id, { y: clamp(selected.y - step, 0, tunerDoc.canvas.height - selected.height) })
    } else if (event.key === 'ArrowDown') {
      event.preventDefault()
      updateElement(selected.id, { y: clamp(selected.y + step, 0, tunerDoc.canvas.height - selected.height) })
    }
  }

  const renderElement = (element: UiTunerElement) => {
    const selectedClass = element.id === selectedId ? styles.selectedElement : ''
    const elementStyle: CSSProperties = {
      left: element.x,
      top: element.y,
      width: element.width,
      height: element.height,
      padding: `${element.paddingY}px ${element.paddingX}px`,
      borderRadius: element.borderRadius,
      borderWidth: element.borderWidth,
      borderColor: element.borderColor,
      color: element.color,
      background: element.background,
      opacity: element.opacity,
      fontSize: element.fontSize,
      lineHeight: `${element.lineHeight}px`,
      fontWeight: element.fontWeight,
      letterSpacing: element.letterSpacing,
    }

    return (
      <button
        key={element.id}
        type="button"
        className={[styles.canvasElement, selectedClass, styles[`kind_${element.kind}`]].join(' ')}
        style={elementStyle}
        onPointerDown={(event) => startDrag(event, element, 'move')}
      >
        <span>{element.text}</span>
        {element.id === selectedId && (
          <span
            className={styles.resizeHandle}
            aria-hidden="true"
            onPointerDown={(event) => startDrag(event, element, 'resize')}
          />
        )}
      </button>
    )
  }

  return (
    <div className={styles.page}>
      <aside className={styles.layersPanel}>
        <div className={styles.panelHeader}>
          <h1>微调画布</h1>
          <p>把设计稿还原后的板块拖到合适位置，再导出参数给我落代码。</p>
        </div>

        <div className={styles.addGroup}>
          <button type="button" onClick={() => addElement('text')}>
            <Type size={14} aria-hidden="true" />
            文字
          </button>
          <button type="button" onClick={() => addElement('card')}>
            <Plus size={14} aria-hidden="true" />
            卡片
          </button>
          <button type="button" onClick={() => addElement('button')}>
            <Plus size={14} aria-hidden="true" />
            按钮
          </button>
        </div>

        <div className={styles.layerList} aria-label="画布图层">
          {tunerDoc.elements.map((element) => (
            <button
              key={element.id}
              type="button"
              className={[styles.layerItem, element.id === selectedId ? styles.activeLayer : ''].join(' ')}
              onClick={() => setSelectedId(element.id)}
            >
              <span>{kindLabel(element.kind)}</span>
              <strong>{element.name}</strong>
              <small>{element.width} x {element.height}</small>
            </button>
          ))}
        </div>
      </aside>

      <section className={styles.stage}>
        <header className={styles.toolbar}>
          <div className={styles.toolbarTitle}>
            <MousePointer2 size={16} aria-hidden="true" />
            <span>{tunerDoc.canvas.name}</span>
          </div>
          <div className={styles.toolbarActions}>
            <button type="button" onClick={saveNow}>
              <Save size={14} aria-hidden="true" />
              保存调整
            </button>
            <button type="button" onClick={copyExport}>
              <Copy size={14} aria-hidden="true" />
              复制参数
            </button>
            <button type="button" onClick={downloadExport} aria-label="下载参数 JSON">
              <Download size={15} aria-hidden="true" />
            </button>
            <button type="button" onClick={resetDocument} aria-label="重置画布">
              <RefreshCw size={15} aria-hidden="true" />
            </button>
          </div>
        </header>

        <div className={styles.canvasScroller}>
          <div
            className={styles.canvas}
            style={{
              width: tunerDoc.canvas.width,
              height: tunerDoc.canvas.height,
              background: tunerDoc.canvas.background,
            }}
            tabIndex={0}
            onKeyDown={handleCanvasKeyDown}
            onPointerDown={(event) => {
              if (event.target === event.currentTarget) setSelectedId(null)
            }}
          >
            <div className={styles.canvasGrid} aria-hidden="true" />
            {tunerDoc.elements.map(renderElement)}
          </div>
        </div>
      </section>

      <aside className={styles.inspector}>
        <section className={styles.section}>
          <h2>画布</h2>
          <label className={styles.fieldFull}>
            <span>名称</span>
            <input
              value={tunerDoc.canvas.name}
              onChange={(event) => updateCanvas({ name: event.currentTarget.value })}
            />
          </label>
          <div className={styles.gridFields}>
            <NumberField
              label="宽"
              value={tunerDoc.canvas.width}
              min={280}
              max={DEFAULT_CANVAS_MAX}
              onChange={(width) => updateCanvas({ width })}
            />
            <NumberField
              label="高"
              value={tunerDoc.canvas.height}
              min={360}
              max={DEFAULT_CANVAS_MAX}
              onChange={(height) => updateCanvas({ height })}
            />
          </div>
          <ColorField
            label="背景"
            value={tunerDoc.canvas.background}
            onChange={(background) => updateCanvas({ background })}
          />
        </section>

        {selected ? (
          <>
            <section className={styles.section}>
              <div className={styles.sectionHeader}>
                <h2>{selected.name}</h2>
                <button type="button" onClick={deleteSelected} aria-label="删除选中元素">
                  <Trash2 size={14} aria-hidden="true" />
                </button>
              </div>
              <label className={styles.fieldFull}>
                <span>图层名</span>
                <input
                  value={selected.name}
                  onChange={(event) => updateElement(selected.id, { name: event.currentTarget.value })}
                />
              </label>
              <label className={styles.fieldFull}>
                <span>文本</span>
                <textarea
                  value={selected.text}
                  onChange={(event) => updateElement(selected.id, { text: event.currentTarget.value })}
                />
              </label>
              <div className={styles.metricGrid}>
                {metrics.map((metric) => (
                  <div key={metric.label}>
                    <span>{metric.label}</span>
                    <strong>{metric.value}</strong>
                  </div>
                ))}
              </div>
            </section>

            <section className={styles.section}>
              <h2>位置和尺寸</h2>
              <div className={styles.gridFields}>
                <NumberField label="X" value={selected.x} min={0} max={tunerDoc.canvas.width} onChange={(x) => updateElement(selected.id, { x })} />
                <NumberField label="Y" value={selected.y} min={0} max={tunerDoc.canvas.height} onChange={(y) => updateElement(selected.id, { y })} />
                <NumberField label="W" value={selected.width} min={MIN_SIZE} max={tunerDoc.canvas.width} onChange={(width) => updateElement(selected.id, { width })} />
                <NumberField label="H" value={selected.height} min={MIN_SIZE} max={tunerDoc.canvas.height} onChange={(height) => updateElement(selected.id, { height })} />
              </div>
            </section>

            <section className={styles.section}>
              <h2>文字和间距</h2>
              <div className={styles.gridFields}>
                <NumberField label="字号" value={selected.fontSize} min={8} max={96} onChange={(fontSize) => updateElement(selected.id, { fontSize })} />
                <NumberField label="行高" value={selected.lineHeight} min={8} max={120} onChange={(lineHeight) => updateElement(selected.id, { lineHeight })} />
                <NumberField label="字距" value={selected.letterSpacing} min={-2} max={12} onChange={(letterSpacing) => updateElement(selected.id, { letterSpacing })} />
                <NumberField label="内距 X" value={selected.paddingX} min={0} max={80} onChange={(paddingX) => updateElement(selected.id, { paddingX })} />
                <NumberField label="内距 Y" value={selected.paddingY} min={0} max={80} onChange={(paddingY) => updateElement(selected.id, { paddingY })} />
                <NumberField label="圆角" value={selected.borderRadius} min={0} max={48} onChange={(borderRadius) => updateElement(selected.id, { borderRadius })} />
              </div>
              <label className={styles.fieldFull}>
                <span>字重</span>
                <select
                  value={selected.fontWeight}
                  onChange={(event) => updateElement(selected.id, { fontWeight: Number(event.currentTarget.value) })}
                >
                  <option value={400}>400</option>
                  <option value={500}>500</option>
                  <option value={600}>600</option>
                  <option value={700}>700</option>
                  <option value={800}>800</option>
                </select>
              </label>
            </section>

            <section className={styles.section}>
              <h2>外观</h2>
              <div className={styles.gridFields}>
                <ColorField label="文字" value={selected.color} onChange={(color) => updateElement(selected.id, { color })} />
                <ColorField label="背景" value={selected.background} onChange={(background) => updateElement(selected.id, { background })} />
                <ColorField label="边框" value={selected.borderColor} onChange={(borderColor) => updateElement(selected.id, { borderColor })} />
                <NumberField label="边框" value={selected.borderWidth} min={0} max={8} onChange={(borderWidth) => updateElement(selected.id, { borderWidth })} />
              </div>
              <label className={styles.rangeField}>
                <span>透明度</span>
                <input
                  type="range"
                  min={0.2}
                  max={1}
                  step={0.05}
                  value={selected.opacity}
                  onChange={(event) => updateElement(selected.id, { opacity: Number(event.currentTarget.value) })}
                />
                <strong>{Math.round(selected.opacity * 100)}%</strong>
              </label>
            </section>
          </>
        ) : (
          <section className={styles.emptyState}>
            <Move size={18} aria-hidden="true" />
            <p>点击画布上的板块后，可在这里调位置、字号、行高、内边距和颜色。</p>
          </section>
        )}

        <section className={styles.section}>
          <h2>导出参数</h2>
          <textarea className={styles.exportBox} value={exportJson} readOnly />
        </section>
      </aside>

      <div className={styles.notice} aria-live="polite">
        {notice}
      </div>
    </div>
  )
}
