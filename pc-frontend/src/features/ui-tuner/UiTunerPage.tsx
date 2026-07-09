import {
  useEffect,
  useCallback,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent as ReactPointerEvent,
} from 'react'
import { APK_STYLE_SOURCE_SIGNATURE, createBlankElement, createInitialTunerDocument } from './presets'
import {
  loadUiTunerDocument,
  saveUiTunerDocument,
  stringifyUiTunerExport,
} from './uiTunerStorage'
import { clamp, getMetrics, touch } from './uiTunerGeometry'
import {
  captureAndroidSnapshot,
  connectAndroidDevice,
  listAndroidDevices,
  type AndroidInspectorDevice,
} from './device/deviceInspectorApi'
import { stringifyCliPatchPackage } from './runtime/cliPatchPackage'
import { snapshotToTunerDocument } from './runtime/snapshotToTunerDocument'
import {
  DEFAULT_UI_TUNER_FILTER,
  filterUiTunerElements,
  type UiTunerElementAnalysis,
  type UiTunerFilterState,
} from './filtering'
import { buildStandardInsight, stringifyStandardPackage } from './standards'
import type { UiTunerDocument, UiTunerElement, UiTunerElementKind } from './types'
import { buildDebugFilter, UiTunerInspector } from './UiTunerInspector'
import { UiTunerLayersPanel } from './UiTunerLayersPanel'
import { UiTunerToolbar } from './UiTunerToolbar'
import styles from './UiTunerPage.module.css'

type DragMode = 'move' | 'resize'

interface DragState {
  id: string
  mode: DragMode
  startX: number
  startY: number
  scale: number
  original: UiTunerElement
}

interface HistoryState {
  past: UiTunerDocument[]
  future: UiTunerDocument[]
}

const MIN_SIZE = 24
const HISTORY_LIMIT = 80
const VIEW_SCALE_MIN = 0.08
const VIEW_SCALE_MAX = 2
const VIEW_SCALE_STEP = 0.1
function getSelectedId(document: UiTunerDocument, preferredId: string | null) {
  if (preferredId && document.elements.some((element) => element.id === preferredId)) {
    return preferredId
  }
  return document.elements[0]?.id ?? null
}

function normalizeViewScale(value: number) {
  if (!Number.isFinite(value)) return 1
  const bounded = Math.min(Math.max(value, VIEW_SCALE_MIN), VIEW_SCALE_MAX)
  return Math.round(bounded * 100) / 100
}

export default function UiTunerPage() {
  const [tunerDoc, setTunerDoc] = useState<UiTunerDocument>(() => (
    loadUiTunerDocument(APK_STYLE_SOURCE_SIGNATURE) ?? createInitialTunerDocument()
  ))
  const [selectedId, setSelectedId] = useState<string | null>(() => tunerDoc.elements[0]?.id ?? null)
  const [dragState, setDragState] = useState<DragState | null>(null)
  const [history, setHistory] = useState<HistoryState>({ past: [], future: [] })
  const [viewScale, setViewScale] = useState(1)
  const [fitToStage, setFitToStage] = useState(true)
  const [notice, setNotice] = useState('')
  const [devices, setDevices] = useState<AndroidInspectorDevice[]>([])
  const [selectedDeviceId, setSelectedDeviceId] = useState('')
  const [connectAddress, setConnectAddress] = useState('')
  const [deviceBusy, setDeviceBusy] = useState(false)
  const [connectBusy, setConnectBusy] = useState(false)
  const [captureBusy, setCaptureBusy] = useState(false)
  const [layerFilter, setLayerFilter] = useState<UiTunerFilterState>(DEFAULT_UI_TUNER_FILTER)
  const canvasScrollerRef = useRef<HTMLDivElement | null>(null)
  const screenshotInputRef = useRef<HTMLInputElement>(null)
  const tunerDocRef = useRef(tunerDoc)
  const selectedIdRef = useRef<string | null>(selectedId)
  const dragSnapshotRef = useRef<UiTunerDocument | null>(null)
  const dragMovedRef = useRef(false)

  const selected = useMemo(
    () => tunerDoc.elements.find((element) => element.id === selectedId) ?? null,
    [selectedId, tunerDoc.elements],
  )
  const exportJson = useMemo(() => stringifyUiTunerExport(tunerDoc), [tunerDoc])
  const metrics = useMemo(
    () => (selected ? getMetrics(selected, tunerDoc.elements, tunerDoc.canvas) : []),
    [selected, tunerDoc.canvas, tunerDoc.elements],
  )
  const filterResult = useMemo(
    () => filterUiTunerElements(tunerDoc, layerFilter),
    [layerFilter, tunerDoc],
  )
  const standardInsight = useMemo(
    () => buildStandardInsight(tunerDoc, selected),
    [selected, tunerDoc],
  )
  const viewScaleLabel = `${Math.round(viewScale * 100)}%`

  const fitCanvasToStage = useCallback(() => {
    const scroller = canvasScrollerRef.current
    if (!scroller) return
    const availableWidth = Math.max(scroller.clientWidth - 56, MIN_SIZE)
    const availableHeight = Math.max(scroller.clientHeight - 56, MIN_SIZE)
    const nextScale = Math.min(
      availableWidth / tunerDoc.canvas.width,
      availableHeight / tunerDoc.canvas.height,
      1,
    )
    setViewScale(normalizeViewScale(nextScale))
  }, [tunerDoc.canvas.height, tunerDoc.canvas.width])

  const setManualViewScale = (nextScale: number) => {
    setFitToStage(false)
    setViewScale(normalizeViewScale(nextScale))
  }

  const pushHistorySnapshot = useCallback((snapshot: UiTunerDocument) => {
    setHistory((current) => ({
      past: [...current.past.slice(-(HISTORY_LIMIT - 1)), snapshot],
      future: [],
    }))
  }, [])

  const commitDocument = useCallback((
    update: (current: UiTunerDocument) => UiTunerDocument,
    preferredSelectedId?: string | null,
  ) => {
    setTunerDoc((current) => {
      const next = update(current)
      pushHistorySnapshot(current)
      const preferredId = preferredSelectedId === undefined ? selectedIdRef.current : preferredSelectedId
      setSelectedId(getSelectedId(next, preferredId))
      return next
    })
  }, [pushHistorySnapshot])

  const undoHistory = useCallback(() => {
    setHistory((current) => {
      const previous = current.past[current.past.length - 1]
      if (!previous) return current
      const present = tunerDocRef.current
      setTunerDoc(previous)
      setSelectedId((id) => getSelectedId(previous, id))
      setNotice('已撤回一步')
      return {
        past: current.past.slice(0, -1),
        future: [present, ...current.future].slice(0, HISTORY_LIMIT),
      }
    })
  }, [])

  const redoHistory = useCallback(() => {
    setHistory((current) => {
      const next = current.future[0]
      if (!next) return current
      const present = tunerDocRef.current
      setTunerDoc(next)
      setSelectedId((id) => getSelectedId(next, id))
      setNotice('已重做一步')
      return {
        past: [...current.past.slice(-(HISTORY_LIMIT - 1)), present],
        future: current.future.slice(1),
      }
    })
  }, [])

  useEffect(() => {
    tunerDocRef.current = tunerDoc
  }, [tunerDoc])

  useEffect(() => {
    selectedIdRef.current = selectedId
  }, [selectedId])

  useEffect(() => {
    saveUiTunerDocument(tunerDoc)
  }, [tunerDoc])

  useEffect(() => {
    if (!fitToStage) return undefined
    fitCanvasToStage()
    const scroller = canvasScrollerRef.current
    if (typeof ResizeObserver === 'undefined' || !scroller) {
      window.addEventListener('resize', fitCanvasToStage)
      return () => window.removeEventListener('resize', fitCanvasToStage)
    }
    const observer = new ResizeObserver(fitCanvasToStage)
    observer.observe(scroller)
    return () => observer.disconnect()
  }, [fitCanvasToStage, fitToStage])

  useEffect(() => {
    if (!notice) return undefined
    const timer = window.setTimeout(() => setNotice(''), 2200)
    return () => window.clearTimeout(timer)
  }, [notice])

  useEffect(() => {
    if (!dragState) return undefined

    const handlePointerMove = (event: PointerEvent) => {
      event.preventDefault()
      dragMovedRef.current = true
      const dx = (event.clientX - dragState.startX) / dragState.scale
      const dy = (event.clientY - dragState.startY) / dragState.scale

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

    const handlePointerUp = () => {
      if (dragSnapshotRef.current && dragMovedRef.current) {
        pushHistorySnapshot(dragSnapshotRef.current)
      }
      dragSnapshotRef.current = null
      dragMovedRef.current = false
      setDragState(null)
    }
    window.addEventListener('pointermove', handlePointerMove)
    window.addEventListener('pointerup', handlePointerUp)
    return () => {
      window.removeEventListener('pointermove', handlePointerMove)
      window.removeEventListener('pointerup', handlePointerUp)
    }
  }, [dragState, pushHistorySnapshot])

  const updateCanvas = (patch: Partial<UiTunerDocument['canvas']>) => {
    commitDocument((current) => touch({ ...current, canvas: { ...current.canvas, ...patch } }))
  }

  const updateElement = (id: string, patch: Partial<UiTunerElement>) => {
    commitDocument((current) => touch({
      ...current,
      elements: current.elements.map((element) => (
        element.id === id ? { ...element, ...patch } : element
      )),
    }), id)
  }

  const updateLayerFilter = (patch: Partial<UiTunerFilterState>) => {
    setLayerFilter((current) => ({ ...current, ...patch }))
  }

  const resetLayerFilter = () => {
    setLayerFilter({ ...DEFAULT_UI_TUNER_FILTER })
  }

  const toggleElementVisibility = (id: string) => {
    commitDocument((current) => touch({
      ...current,
      elements: current.elements.map((element) => {
        if (element.id !== id) return element
        return { ...element, visibility: element.visibility === 'hidden' ? 'visible' : 'hidden' }
      }),
    }), id)
  }

  const toggleElementLock = (id: string) => {
    commitDocument((current) => touch({
      ...current,
      elements: current.elements.map((element) => {
        if (element.id !== id) return element
        return { ...element, visibility: element.visibility === 'locked' ? 'visible' : 'locked' }
      }),
    }), id)
  }

  const addElement = (kind: UiTunerElementKind) => {
    const next = createBlankElement(kind, tunerDocRef.current.elements.length + 1)
    commitDocument((current) => touch({ ...current, elements: [...current.elements, next] }), next.id)
  }

  const deleteSelected = () => {
    if (!selected) return
    commitDocument((current) => touch({
      ...current,
      elements: current.elements.filter((element) => element.id !== selected.id),
    }), null)
  }

  const resetDocument = () => {
    if (!window.confirm('重置后会清空当前微调记录，并重新读取当前 APK 样式源码，确定继续吗？')) return
    const next = createInitialTunerDocument()
    commitDocument(() => next, next.elements[0]?.id ?? null)
    setNotice('已恢复当前 APK 样式')
  }

  const saveNow = () => {
    saveUiTunerDocument(tunerDoc)
    setNotice('已保存到本机草稿')
  }

  const refreshDevices = async () => {
    setDeviceBusy(true)
    try {
      const nextDevices = await listAndroidDevices()
      setDevices(nextDevices)
      setSelectedDeviceId((current) => (
        current && nextDevices.some((device) => device.serial === current)
          ? current
          : nextDevices.find((device) => device.state === 'device')?.serial ?? nextDevices[0]?.serial ?? ''
      ))
      setNotice(nextDevices.length ? `已发现 ${nextDevices.length} 台 ADB 设备` : '未发现可用 ADB 设备')
    } catch (error) {
      setNotice(error instanceof Error ? error.message : '读取 ADB 设备失败')
    } finally {
      setDeviceBusy(false)
    }
  }

  const captureDeviceSnapshot = async () => {
    const deviceId = selectedDeviceId || devices[0]?.serial || ''
    if (!deviceId) {
      await refreshDevices()
      return
    }
    setCaptureBusy(true)
    try {
      const snapshot = await captureAndroidSnapshot({ deviceId, packageName: 'com.elon.app' })
      const next = snapshotToTunerDocument(snapshot)
      commitDocument(() => next, next.elements[0]?.id ?? null)
      setFitToStage(true)
      setLayerFilter({ ...DEFAULT_UI_TUNER_FILTER })
      setNotice(`已捕获真机画面：${snapshot.xml.nodeCount} 个 XML 节点`)
    } catch (error) {
      setNotice(error instanceof Error ? error.message : '真机捕获失败')
    } finally {
      setCaptureBusy(false)
    }
  }

  const connectWirelessDevice = async () => {
    const address = connectAddress.trim()
    if (!address) return
    setConnectBusy(true)
    try {
      const output = await connectAndroidDevice(address)
      setNotice(output.trim() || `已连接 ${address}`)
      await refreshDevices()
    } catch (error) {
      setNotice(error instanceof Error ? error.message : '无线 ADB 连接失败')
    } finally {
      setConnectBusy(false)
    }
  }
  const copyExport = async () => {
    try {
      await navigator.clipboard.writeText(exportJson)
      setNotice('参数 JSON 已复制')
    } catch {
      setNotice('复制失败，可手动选中文本')
    }
  }

  const copyCliPatch = async () => {
    try {
      await navigator.clipboard.writeText(stringifyCliPatchPackage(tunerDoc))
      setNotice('CLI 修改包已复制')
    } catch {
      setNotice('复制失败，可先复制参数 JSON')
    }
  }

  const copyStandardPackage = async () => {
    try {
      await navigator.clipboard.writeText(stringifyStandardPackage(tunerDoc, selected))
      setNotice('组件标准草案已复制')
    } catch {
      setNotice('复制失败，可先复制 CLI 包')
    }
  }

  const applySelectedStandard = (standard: UiTunerElement['standard']) => {
    if (!selected || !standard) return
    updateElement(selected.id, { standard })
    setNotice('已把选中节点标记为标准草案')
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

  const importScreenshot = (file: File) => {
    const reader = new FileReader()
    reader.onload = () => {
      const dataUrl = typeof reader.result === 'string' ? reader.result : ''
      if (!dataUrl) return
      const image = new Image()
      image.onload = () => {
        const width = Math.max(Math.round(image.naturalWidth), MIN_SIZE)
        const height = Math.max(Math.round(image.naturalHeight), MIN_SIZE)
        commitDocument((current) => touch({
          ...current,
          canvas: {
            ...current.canvas,
            name: `${file.name} 调试画布`,
            width,
            height,
            background: '#000000',
            referenceImage: {
              dataUrl,
              name: file.name,
              width,
              height,
              opacity: 1,
              visible: true,
            },
          },
        }))
        setFitToStage(true)
        setNotice('已把 APP 截图放到画布底层')
      }
      image.onerror = () => setNotice('截图读取失败，请换一张图片')
      image.src = dataUrl
    }
    reader.onerror = () => setNotice('截图读取失败，请换一张图片')
    reader.readAsDataURL(file)
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
    dragSnapshotRef.current = tunerDocRef.current
    dragMovedRef.current = false
    setDragState({
      id: element.id,
      mode,
      startX: event.clientX,
      startY: event.clientY,
      scale: viewScale,
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

  const renderElement = (element: UiTunerElement, analysis: UiTunerElementAnalysis) => {
    const selectedClass = element.id === selectedId ? styles.selectedElement : ''
    const appearanceClass = analysis.appearance === 'ghost'
      ? styles.ghostElement
      : analysis.appearance === 'outline'
        ? styles.outlineElement
        : ''
    const lockedClass = analysis.isLocked ? styles.lockedElement : ''
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
        className={[
          styles.canvasElement,
          selectedClass,
          appearanceClass,
          lockedClass,
          styles[`kind_${element.kind}`],
        ].join(' ')}
        style={elementStyle}
        onPointerDown={(event) => {
          if (analysis.isLocked) {
            event.stopPropagation()
            setSelectedId(element.id)
            return
          }
          startDrag(event, element, 'move')
        }}
      >
        <span>{analysis.appearance === 'outline' ? analysis.role : element.text}</span>
        {element.id === selectedId && !analysis.isLocked && (
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
      <UiTunerLayersPanel
        filter={layerFilter}
        filterResult={filterResult}
        selectedId={selectedId}
        onAddElement={addElement}
        onFilterChange={updateLayerFilter}
        onResetFilter={resetLayerFilter}
        onSelectElement={setSelectedId}
        onToggleElementVisibility={toggleElementVisibility}
        onToggleElementLock={toggleElementLock}
      />

      <section className={styles.stage}>
        <UiTunerToolbar
          canvasName={tunerDoc.canvas.name}
          screenshotInputRef={screenshotInputRef}
          devices={devices}
          selectedDeviceId={selectedDeviceId}
          connectAddress={connectAddress}
          deviceBusy={deviceBusy}
          connectBusy={connectBusy}
          captureBusy={captureBusy}
          viewScaleLabel={viewScaleLabel}
          fitToStage={fitToStage}
          canUndo={history.past.length > 0}
          canRedo={history.future.length > 0}
          onImportScreenshot={importScreenshot}
          onSelectDevice={setSelectedDeviceId}
          onConnectAddressChange={setConnectAddress}
          onRefreshDevices={refreshDevices}
          onConnectWirelessDevice={connectWirelessDevice}
          onCaptureDeviceSnapshot={captureDeviceSnapshot}
          onZoomOut={() => setManualViewScale(viewScale - VIEW_SCALE_STEP)}
          onZoomIn={() => setManualViewScale(viewScale + VIEW_SCALE_STEP)}
          onFitToStage={() => {
            setFitToStage(true)
            fitCanvasToStage()
          }}
          onActualSize={() => setManualViewScale(1)}
          onUndo={undoHistory}
          onRedo={redoHistory}
          onSave={saveNow}
          onCopyExport={copyExport}
          onCopyCliPatch={copyCliPatch}
          onCopyStandardPackage={copyStandardPackage}
          onDownloadExport={downloadExport}
          onReset={resetDocument}
        />

        <div className={styles.canvasScroller} ref={canvasScrollerRef}>
          <div
            className={styles.canvasViewport}
            style={{
              width: tunerDoc.canvas.width * viewScale,
              height: tunerDoc.canvas.height * viewScale,
            }}
          >
          <div
            className={styles.canvas}
            style={{
              width: tunerDoc.canvas.width,
              height: tunerDoc.canvas.height,
              background: tunerDoc.canvas.background,
              transform: `scale(${viewScale})`,
            }}
            tabIndex={0}
            onKeyDown={handleCanvasKeyDown}
            onPointerDown={(event) => {
              if (event.target === event.currentTarget) setSelectedId(null)
            }}
          >
            <div className={styles.canvasGrid} aria-hidden="true" />
            {tunerDoc.canvas.referenceImage?.visible && (
              <img
                className={styles.referenceImage}
                src={tunerDoc.canvas.referenceImage.dataUrl}
                alt=""
                style={{ opacity: tunerDoc.canvas.referenceImage.opacity }}
              />
            )}
            {filterResult.visible.map(({ element, analysis }) => renderElement(element, analysis))}
          </div>
          </div>
        </div>
      </section>

      <UiTunerInspector
        tunerDoc={tunerDoc}
        selected={selected}
        metrics={metrics}
        filterResult={filterResult}
        standardInsight={standardInsight}
        exportJson={exportJson}
        onUpdateCanvas={updateCanvas}
        onUpdateElement={updateElement}
        onDeleteSelected={deleteSelected}
        onCopyCliPatch={copyCliPatch}
        onCopyStandardPackage={copyStandardPackage}
        onApplyStandard={applySelectedStandard}
        onSetProductMode={() => setLayerFilter({ ...DEFAULT_UI_TUNER_FILTER })}
        onSetDebugMode={() => setLayerFilter(buildDebugFilter())}
      />

      <div className={styles.notice} aria-live="polite">
        {notice}
      </div>
    </div>
  )
}
