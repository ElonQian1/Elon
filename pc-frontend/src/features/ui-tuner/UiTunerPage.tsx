import {
  useEffect,
  useCallback,
  useMemo,
  useRef,
  useState,
} from 'react'
import { APK_STYLE_SOURCE_SIGNATURE, createBlankElement, createInitialTunerDocument } from './presets'
import {
  loadUiTunerDocument,
  saveUiTunerDocument,
  stringifyUiTunerExport,
} from './uiTunerStorage'
import {
  APP_SIDEBAR_TEMPLATE_SOURCE,
  createAppSidebarTemplateElements,
  isAppSidebarTemplateElement,
} from './appSidebarTemplate'
import { getMetrics, touch } from './uiTunerGeometry'
import { type AndroidInspectorSnapshot } from './device/deviceInspectorApi'
import { UiTunerDeviceDialog } from './device/UiTunerDeviceDialog'
import { useAndroidInspectorDevices } from './device/useAndroidInspectorDevices'
import { stringifyCliPatchPackage } from './runtime/cliPatchPackage'
import { snapshotToTunerDocument } from './runtime/snapshotToTunerDocument'
import {
  DEFAULT_UI_TUNER_FILTER,
  filterUiTunerElements,
  type UiTunerFilterState,
} from './filtering'
import { buildStandardInsight, stringifyStandardPackage } from './standards'
import type { UiTunerDocument, UiTunerElement, UiTunerElementKind } from './types'
import { buildDebugFilter, UiTunerInspector } from './UiTunerInspector'
import { prepareLiveDebugRuntime } from './live/liveUiApi'
import { useRuntimeDocumentSync } from './live/useRuntimeDocumentSync'
import { useRuntimeCanvasGesture } from './live/useRuntimeCanvasGesture'
import { UiTunerLayersPanel } from './UiTunerLayersPanel'
import { UiTunerToolbar } from './UiTunerToolbar'
import { UiTunerComparisonWorkspace } from './comparison/UiTunerComparisonWorkspace'
import { useComparisonViewport } from './comparison/useComparisonViewport'
import { useProjectStore } from '../conversation/useProjectStore'
import { mergeProjectRecords } from '../conversation/conversationPageHelpers'
import { clean } from '../../lib/utils'
import type { UiTunerCodexContextPack } from './contextPack'
import {
  createVerificationBaseline,
  failedVerification,
  verifyPostChangeSnapshot,
  type UiTunerVerificationBaseline,
  type UiTunerVerificationReport,
} from './runtime/verification'
import styles from './UiTunerPage.module.css'
import { EvidenceModeSwitch, SourcePreviewWorkspace } from './source-preview/SourcePreviewWorkspace'
import type { SourcePreviewMode } from './source-preview/types'
import { handleCanvasArrowKey } from './uiTunerCanvasKeyboard'

interface HistoryState {
  past: UiTunerDocument[]
  future: UiTunerDocument[]
}

const MIN_SIZE = 24
const HISTORY_LIMIT = 80
const LIVE_DEBUG_SUFFIX = '.uituner'
const DEFAULT_ANDROID_PACKAGE = 'com.elon.app'
const WORKSPACE_MODE_STORAGE_KEY = 'elon.uiTuner.workspaceMode.v2'
function getSelectedId(document: UiTunerDocument, preferredId: string | null) {
  if (preferredId && document.elements.some((element) => element.id === preferredId)) {
    return preferredId
  }
  return document.elements[0]?.id ?? null
}

export default function UiTunerPage() {
  const [workspaceMode, setWorkspaceMode] = useState<SourcePreviewMode>(() => (
    window.localStorage.getItem(WORKSPACE_MODE_STORAGE_KEY) as SourcePreviewMode | null
  ) ?? 'evidence')
  const projects = useProjectStore((state) => state.projects)
  const activeProjectId = useProjectStore((state) => state.activeProjectId)
  const projectSpace = useProjectStore((state) => state.space)
  const [tunerDoc, setTunerDoc] = useState<UiTunerDocument>(() => (
    loadUiTunerDocument(APK_STYLE_SOURCE_SIGNATURE) ?? createInitialTunerDocument()
  ))
  const [selectedId, setSelectedId] = useState<string | null>(() => tunerDoc.elements[0]?.id ?? null)
  const [history, setHistory] = useState<HistoryState>({ past: [], future: [] })
  const [notice, setNotice] = useState('')
  const [layerFilter, setLayerFilter] = useState<UiTunerFilterState>(DEFAULT_UI_TUNER_FILTER)
  const screenshotInputRef = useRef<HTMLInputElement>(null)
  const tunerDocRef = useRef(tunerDoc)
  const selectedIdRef = useRef<string | null>(selectedId)
  const verificationCaptureRef = useRef(false)
  const verificationBaselineRef = useRef<UiTunerVerificationBaseline | null>(null)
  const [verificationReport, setVerificationReport] = useState<UiTunerVerificationReport | null>(null)
  const [liveTargetPackage, setLiveTargetPackage] = useState(() => {
    const captured = tunerDoc.runtimeSnapshot?.packageName ?? ''
    return captured.endsWith(LIVE_DEBUG_SUFFIX) ? captured : ''
  })
  const [livePrepareBusy, setLivePrepareBusy] = useState(false)
  const [livePrepareError, setLivePrepareError] = useState('')
  const comparisonViewport = useComparisonViewport(
    { width: tunerDoc.canvas.width, height: tunerDoc.canvas.height },
    tunerDoc.canvas.targetDesign
      ? { width: tunerDoc.canvas.targetDesign.width, height: tunerDoc.canvas.targetDesign.height }
      : null,
  )
  const { viewScale, viewScaleLabel, fitToStage, requestFit } = comparisonViewport

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
  const activeProject = useMemo(
    () => mergeProjectRecords(
      projects.find((project) => project.id === activeProjectId),
      projectSpace?.project,
    ),
    [activeProjectId, projectSpace?.project, projects],
  )
  const projectRoot = clean(activeProject?.workspace_path ?? activeProject?.storage_worktree_path)
  const [liveProjectRoot, setLiveProjectRoot] = useState(() => (
    window.localStorage.getItem('elon.uiTuner.liveProjectRoot') ?? ''
  ))
  // A manually entered local root is an explicit source-write target and must
  // override the project's last remembered worktree. This also prevents a
  // stale project record from committing LIVE changes into the wrong checkout.
  const effectiveProjectRoot = clean(liveProjectRoot) || projectRoot

  const changeWorkspaceMode = useCallback((mode: SourcePreviewMode) => {
    setWorkspaceMode(mode)
    window.localStorage.setItem(WORKSPACE_MODE_STORAGE_KEY, mode)
  }, [])

  useEffect(() => {
    if (projectRoot && !clean(liveProjectRoot)) setLiveProjectRoot(projectRoot)
  }, [liveProjectRoot, projectRoot])

  const updateLiveProjectRoot = useCallback((value: string) => {
    setLiveProjectRoot(value)
    const cleanValue = value.trim()
    if (cleanValue) window.localStorage.setItem('elon.uiTuner.liveProjectRoot', cleanValue)
    else window.localStorage.removeItem('elon.uiTuner.liveProjectRoot')
  }, [])

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

  const handleDeviceCaptured = useCallback((snapshot: AndroidInspectorSnapshot) => {
    if (verificationCaptureRef.current) return
    const foregroundPackage = snapshot.activityName
      ?.match(/([A-Za-z0-9_.]+)\/[A-Za-z0-9_.$]+/)?.[1]
    setLiveTargetPackage(foregroundPackage || snapshot.packageName || '')
    const next = snapshotToTunerDocument(snapshot)
    commitDocument(() => next, next.elements[0]?.id ?? null)
    requestFit()
    setLayerFilter({ ...DEFAULT_UI_TUNER_FILTER })
    setNotice(snapshot.xml.nodeCount > 0
      ? snapshot.sourceRoot
        ? `已捕获真实手机画面：${snapshot.xml.nodeCount} 个节点，并已绑定项目源码`
        : `已捕获真实手机画面：${snapshot.xml.nodeCount} 个节点；请选择自项目后重新捕获以绑定源码`
      : '已捕获真实手机画面；当前页面未提供可解析控件层级，但截图仍可用于微调')
  }, [commitDocument, requestFit])

  const {
    devices,
    selectedDeviceId,
    deviceBusy,
    captureBusy,
    wirelessBusy,
    deviceDialogOpen,
    wirelessStatus,
    setSelectedDeviceId,
    setDeviceDialogOpen,
    refreshDevices,
    refreshWirelessStatus,
    openDeviceManager,
    reconnectWirelessDevices,
    registerWiredDevice,
    pairWirelessDevice,
    enableLegacyWireless,
    connectWirelessAddress,
    forgetWirelessDevice,
    captureDeviceSnapshot,
  } = useAndroidInspectorDevices({
    onCaptured: handleDeviceCaptured,
    onNotice: setNotice,
    projectRoot: effectiveProjectRoot,
    packageName: liveTargetPackage || undefined,
  })

  const prepareLiveRuntime = useCallback(async () => {
    if (!selectedDeviceId) {
      setDeviceDialogOpen(true)
      setNotice('请先连接并选择一台 Android 手机')
      return
    }
    if (!effectiveProjectRoot) {
      setLivePrepareError('请先在 PC 工作台选择一个本机 Android 项目，才能构建实时调试包。')
      setNotice('缺少本机项目目录，无法构建实时调试包')
      return
    }
    const capturedPackage = tunerDocRef.current.runtimeSnapshot?.packageName ?? DEFAULT_ANDROID_PACKAGE
    const basePackageName = capturedPackage.endsWith(LIVE_DEBUG_SUFFIX)
      ? capturedPackage.slice(0, -LIVE_DEBUG_SUFFIX.length)
      : capturedPackage
    setLivePrepareBusy(true)
    setLivePrepareError('')
    setNotice('正在构建并安装实时调试包；首次构建可能需要几分钟…')
    try {
      const prepared = await prepareLiveDebugRuntime({
        deviceId: selectedDeviceId,
        basePackageName,
        projectRoot: effectiveProjectRoot,
        debugApplicationIdSuffix: LIVE_DEBUG_SUFFIX,
      })
      setLiveTargetPackage(prepared.packageName)
      const snapshot = await captureDeviceSnapshot({
        deviceId: selectedDeviceId,
        packageName: prepared.packageName,
      })
      if (!snapshot) throw new Error('调试包已安装，但自动捕获失败；请保持手机解锁后重试')
      setNotice(`实时调试包 ${prepared.packageName} 已安装，正在连接 Runtime…`)
    } catch (error) {
      const message = error instanceof Error ? error.message : '实时调试包准备失败'
      setLivePrepareError(message)
      setNotice(message)
    } finally {
      setLivePrepareBusy(false)
    }
  }, [captureDeviceSnapshot, effectiveProjectRoot, selectedDeviceId, setDeviceDialogOpen])

  const liveUi = useRuntimeDocumentSync({
    deviceId: liveTargetPackage ? selectedDeviceId : tunerDoc.runtimeSnapshot?.deviceId,
    packageName: liveTargetPackage || tunerDoc.runtimeSnapshot?.packageName,
    projectRoot: effectiveProjectRoot,
    debugApplicationIdSuffix: liveTargetPackage.endsWith(LIVE_DEBUG_SUFFIX) ? LIVE_DEBUG_SUFFIX : undefined,
    document: tunerDoc, selected, workspaceMode, documentRef: tunerDocRef, selectedIdRef,
    setDocument: setTunerDoc, setSelectedId, onNotice: setNotice,
  })
  const runtimeDocument = Boolean(
    liveTargetPackage.endsWith(LIVE_DEBUG_SUFFIX)
    || liveUi.session?.packageName.endsWith(LIVE_DEBUG_SUFFIX)
    || tunerDoc.runtimeSnapshot?.packageName?.endsWith(LIVE_DEBUG_SUFFIX),
  )
  const realRenderer = workspaceMode === 'evidence'
    && runtimeDocument
    && Boolean(liveUi.liveFrame || tunerDoc.canvas.referenceImage?.visible)
  const canvasGesture = useRuntimeCanvasGesture({
    documentRef: tunerDocRef,
    setDocument: setTunerDoc,
    setSelectedId,
    pushHistorySnapshot,
    selectedNode: liveUi.selectedNode,
    realRenderer,
    runtimeConnected: liveUi.state === 'connected',
    viewScale,
    applyRuntimeGesture: liveUi.applyGesture,
    setRuntimeGestureActive: liveUi.setGestureActive,
    onNotice: setNotice,
  })

  const handleLiveOptimisticUpdate = useCallback((patch: Partial<UiTunerElement>) => {
    const currentId = selectedIdRef.current
    if (!currentId) return
    setTunerDoc((current) => touch({
      ...current,
      elements: current.elements.map((element) => (
        element.id === currentId ? { ...element, ...patch } : element
      )),
    }))
  }, [])

  const handleMutationTaskStarted = useCallback(async (pack: UiTunerCodexContextPack) => {
    const currentSelected = tunerDocRef.current.elements.find((element) => element.id === selectedIdRef.current)
    if (!currentSelected) return
    verificationBaselineRef.current = await createVerificationBaseline(
      tunerDocRef.current,
      currentSelected,
      pack,
    )
    setVerificationReport({
      phase: 'waiting_codex',
      message: 'Codex 正在修改、构建并安装；任务结束后会自动重新采集真机。',
      beforePreviewDataUrl: verificationBaselineRef.current.beforePreviewDataUrl,
      requestedAdjustmentCount: pack.requestedAdjustments.length,
      verifiedAdjustmentCount: 0,
      retryable: false,
    })
  }, [])

  const requestPostTaskVerification = useCallback(async () => {
    const baseline = verificationBaselineRef.current
    if (!baseline) {
      setVerificationReport(failedVerification('缺少修改前基线，请重新选择元素并发送一次 Codex 修改任务。'))
      return
    }
    setVerificationReport((current) => ({
      phase: 'capturing',
      message: 'Codex 任务已结束，正在重新采集真机并定位同一组件…',
      beforePreviewDataUrl: baseline.beforePreviewDataUrl,
      requestedAdjustmentCount: baseline.pack.requestedAdjustments.length,
      verifiedAdjustmentCount: current?.verifiedAdjustmentCount ?? 0,
      retryable: false,
    }))
    verificationCaptureRef.current = true
    const snapshot = await captureDeviceSnapshot()
    verificationCaptureRef.current = false
    if (!snapshot) {
      setVerificationReport(failedVerification('真机重新采集失败。请确认手机仍在线并停留在目标页面，然后重试。'))
      return
    }
    try {
      const { report, document } = await verifyPostChangeSnapshot(baseline, snapshot)
      commitDocument(() => document, report.matchedElementId ?? document.elements[0]?.id ?? null)
      setLayerFilter({ ...DEFAULT_UI_TUNER_FILTER })
      requestFit()
      setVerificationReport(report)
      setNotice(report.message)
    } catch (error) {
      setVerificationReport(failedVerification(
        error instanceof Error ? error.message : '前后快照验收失败，请重试',
      ))
    }
  }, [captureDeviceSnapshot, commitDocument, requestFit])

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
    if (!notice) return undefined
    const timer = window.setTimeout(() => setNotice(''), 2200)
    return () => window.clearTimeout(timer)
  }, [notice])

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

  const applyAppSidebarTemplate = () => {
    const templateElements = createAppSidebarTemplateElements(tunerDocRef.current.canvas)
    const selectedTemplate = templateElements.find((element) => element.id.endsWith('.search')) ?? templateElements[0]
    commitDocument((current) => touch({
      ...current,
      source: APP_SIDEBAR_TEMPLATE_SOURCE,
      canvas: {
        ...current.canvas,
        name: current.canvas.referenceImage ? `${current.canvas.referenceImage.name} 侧边栏模板` : 'APP 侧边栏模板画布',
        source: APP_SIDEBAR_TEMPLATE_SOURCE,
      },
      elements: [
        ...current.elements.filter((element) => (
          !isAppSidebarTemplateElement(element) && !element.id.startsWith('apk.')
        )),
        ...templateElements,
      ],
    }), selectedTemplate?.id ?? null)
    setLayerFilter({ ...DEFAULT_UI_TUNER_FILTER })
    setNotice('已生成 APP 侧边栏模板，可调搜索框、项目卡和底部用户栏')
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
        commitDocument((current) => {
          const imported = {
            dataUrl,
            name: file.name,
            width,
            height,
            opacity: current.runtimeSnapshot ? 0.5 : 1,
            visible: true,
          }
          return touch({
            ...current,
            canvas: current.runtimeSnapshot ? {
              ...current.canvas,
              targetDesign: imported,
            } : {
              ...current.canvas,
              name: file.name + ' 调试画布',
              width,
              height,
              background: '#000000',
              referenceImage: imported,
            },
          })
        })
        requestFit()
        setNotice(tunerDocRef.current.runtimeSnapshot
          ? '已导入目标设计图；可叠加对照并对选中节点运行本地视觉求解'
          : '已把 APP 截图放到画布底层')
      }
      image.onerror = () => setNotice('截图读取失败，请换一张图片')
      image.src = dataUrl
    }
    reader.onerror = () => setNotice('截图读取失败，请换一张图片')
    reader.readAsDataURL(file)
  }

  const handleCanvasKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    handleCanvasArrowKey(event, selected, tunerDoc.canvas, updateElement)
  }

  return (
    <>
    <SourcePreviewWorkspace active={workspaceMode === 'source'} initialProjectRoot={effectiveProjectRoot} onModeChange={changeWorkspaceMode} />
    <div className={styles.page} style={{ display: workspaceMode === 'evidence' ? 'grid' : 'none' }}>
      <UiTunerLayersPanel
        realRenderer={realRenderer}
        filter={layerFilter}
        filterResult={filterResult}
        selectedId={selectedId}
        onAddElement={addElement}
        onApplyAppSidebarTemplate={applyAppSidebarTemplate}
        onFilterChange={updateLayerFilter}
        onResetFilter={resetLayerFilter}
        onSelectElement={setSelectedId}
        onToggleElementVisibility={toggleElementVisibility}
        onToggleElementLock={toggleElementLock}
      />

      <section className={styles.stage}>
        <EvidenceModeSwitch initialProjectRoot={effectiveProjectRoot} onModeChange={changeWorkspaceMode} />
        <UiTunerToolbar
          canvasName={tunerDoc.canvas.name}
          screenshotInputRef={screenshotInputRef}
          devices={devices}
          selectedDeviceId={selectedDeviceId}
          deviceBusy={deviceBusy}
          captureBusy={captureBusy}
          wirelessConnected={wirelessStatus?.profiles.some((profile) => (
            profile.connectionState === 'connected_wireless'
          )) ?? false}
          viewScaleLabel={viewScaleLabel}
          fitToStage={fitToStage}
          canUndo={realRenderer ? (liveUi.session?.historyCount ?? 0) > 0 : history.past.length > 0}
          canRedo={realRenderer ? (liveUi.session?.redoCount ?? 0) > 0 : history.future.length > 0}
          onImportScreenshot={importScreenshot}
          onSelectDevice={setSelectedDeviceId}
          onRefreshDevices={refreshDevices}
          onOpenDeviceManager={openDeviceManager}
          onCaptureDeviceSnapshot={captureDeviceSnapshot}
          onZoomOut={comparisonViewport.zoomOut}
          onZoomIn={comparisonViewport.zoomIn}
          onFitToStage={comparisonViewport.fitCanvasToStage}
          onActualSize={comparisonViewport.actualSize}
          onUndo={() => { if (realRenderer) void liveUi.undo(); else undoHistory() }}
          onRedo={() => { if (realRenderer) void liveUi.redo(); else redoHistory() }}
          onSave={saveNow}
          onCopyExport={copyExport}
          onCopyCliPatch={copyCliPatch}
          onCopyStandardPackage={copyStandardPackage}
          onDownloadExport={downloadExport}
          onReset={resetDocument}
        />

        <UiTunerComparisonWorkspace
          document={tunerDoc}
          filterResult={filterResult}
          liveFrame={liveUi.liveFrame}
          realRenderer={realRenderer}
          runtimeConnected={liveUi.state === 'connected'}
          runtimeGestureActive={canvasGesture.runtimeGestureActive}
          runtimeCanMove={canvasGesture.canMove}
          runtimeCanResize={canvasGesture.canResize}
          liveNode={liveUi.selectedNode}
          liveNodes={liveUi.nodes}
          liveSession={liveUi.session}
          previewRequest={liveUi.previewRequest}
          uploadedTarget={liveUi.targetDesign}
          selectedId={selectedId}
          viewScale={viewScale}
          targetScrollerRef={comparisonViewport.targetScrollerRef}
          currentScrollerRef={comparisonViewport.currentScrollerRef}
          onTargetScroll={comparisonViewport.onTargetScroll}
          onCurrentScroll={comparisonViewport.onCurrentScroll}
          onCanvasKeyDown={handleCanvasKeyDown}
          onClearSelection={() => setSelectedId(null)}
          onElementPointerDown={canvasGesture.startGesture}
          onSelectElement={setSelectedId}
          onNotice={setNotice}
        />
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
        verificationReport={verificationReport}
        onMutationTaskStarted={handleMutationTaskStarted}
        onRequestVerification={() => { void requestPostTaskVerification() }}
        liveUi={liveUi}
        onLiveOptimisticUpdate={handleLiveOptimisticUpdate}
        livePrepareBusy={livePrepareBusy}
        livePrepareError={livePrepareError}
        livePrepareReady={Boolean(selectedDeviceId && effectiveProjectRoot)}
        liveDebugPackage={liveTargetPackage || `${DEFAULT_ANDROID_PACKAGE}${LIVE_DEBUG_SUFFIX}`}
        liveProjectRoot={liveProjectRoot}
        onLiveProjectRootChange={updateLiveProjectRoot}
        onPrepareLiveRuntime={() => { void prepareLiveRuntime() }}
      />

      <UiTunerDeviceDialog
        open={deviceDialogOpen}
        busy={wirelessBusy}
        status={wirelessStatus}
        devices={devices}
        selectedDeviceId={selectedDeviceId}
        onClose={() => setDeviceDialogOpen(false)}
        onSelectDevice={setSelectedDeviceId}
        onRefresh={() => { void refreshWirelessStatus() }}
        onRegister={registerWiredDevice}
        onPair={pairWirelessDevice}
        onReconnect={(profileId) => { void reconnectWirelessDevices(profileId) }}
        onEnableLegacy={enableLegacyWireless}
        onConnectAddress={connectWirelessAddress}
        onForget={(profileId) => { void forgetWirelessDevice(profileId) }}
      />

      <div className={styles.notice} aria-live="polite">
        {notice}
      </div>
    </div>
    </>
  )
}
