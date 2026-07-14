import type { RefObject } from 'react'
import {
  Copy,
  Database,
  Download,
  FileCode2,
  ImagePlus,
  Maximize2,
  Minus,
  MousePointer2,
  PanelsTopLeft,
  PanelLeftClose,
  PanelLeftOpen,
  PanelRightClose,
  PanelRightOpen,
  Plus,
  RefreshCw,
  Redo2,
  Save,
  Smartphone,
  Undo2,
  Wifi,
} from 'lucide-react'
import type { AndroidInspectorDevice } from './device/deviceInspectorApi'
import type { AndroidDeviceLease } from './device/deviceLeaseApi'
import styles from './UiTunerPage.module.css'

function deviceConnectionLabel(device: AndroidInspectorDevice) {
  if (device.connectionType === 'usb') return 'USB'
  if (device.connectionType === 'wireless') return '无线'
  if (device.connectionType === 'emulator') return '模拟器'
  return 'ADB'
}

interface UiTunerToolbarProps {
  canvasName: string
  screenshotInputRef: RefObject<HTMLInputElement>
  devices: AndroidInspectorDevice[]
  selectedDeviceId: string
  deviceBusy: boolean
  captureBusy: boolean
  captureIssue: string
  capturedDeviceId?: string
  liveConnected: boolean
  wirelessConnected: boolean
  deviceLeaseLabel?: string
  deviceLeaseBlocked?: boolean
  deviceLeases: AndroidDeviceLease[]
  viewScaleLabel: string
  fitToStage: boolean
  canUndo: boolean
  canRedo: boolean
  leftPanelOpen: boolean
  rightPanelOpen: boolean
  focusMode: boolean
  onImportScreenshot: (file: File) => void
  onSelectDevice: (deviceId: string) => void
  onRefreshDevices: () => void
  onOpenDeviceManager: () => void
  onCaptureDeviceSnapshot: () => void
  onZoomOut: () => void
  onZoomIn: () => void
  onFitToStage: () => void
  onActualSize: () => void
  onUndo: () => void
  onRedo: () => void
  onSave: () => void
  onCopyExport: () => void
  onCopyCliPatch: () => void
  onCopyStandardPackage: () => void
  onDownloadExport: () => void
  onReset: () => void
  onToggleLeftPanel: () => void
  onToggleRightPanel: () => void
  onToggleFocusMode: () => void
}

export function UiTunerToolbar({
  canvasName,
  screenshotInputRef,
  devices,
  selectedDeviceId,
  deviceBusy,
  captureBusy,
  captureIssue,
  capturedDeviceId,
  liveConnected,
  wirelessConnected,
  deviceLeaseLabel,
  deviceLeaseBlocked,
  deviceLeases,
  viewScaleLabel,
  fitToStage,
  canUndo,
  canRedo,
  leftPanelOpen,
  rightPanelOpen,
  focusMode,
  onImportScreenshot,
  onSelectDevice,
  onRefreshDevices,
  onOpenDeviceManager,
  onCaptureDeviceSnapshot,
  onZoomOut,
  onZoomIn,
  onFitToStage,
  onActualSize,
  onUndo,
  onRedo,
  onSave,
  onCopyExport,
  onCopyCliPatch,
  onCopyStandardPackage,
  onDownloadExport,
  onReset,
  onToggleLeftPanel,
  onToggleRightPanel,
  onToggleFocusMode,
}: UiTunerToolbarProps) {
  const selectedDevice = devices.find((device) => device.serial === selectedDeviceId)
  const selectedName = selectedDevice?.model ?? selectedDevice?.serial
  const deviceStatus = !selectedName
    ? '请选择手机'
    : captureBusy
      ? `正在读取 · ${selectedName}`
      : captureIssue
        ? `需处理 · ${selectedName}`
        : liveConnected && capturedDeviceId === selectedDeviceId
        ? `LIVE · ${selectedName}`
        : capturedDeviceId === selectedDeviceId
          ? `画面已读取 · ${selectedName}`
          : `等待画面 · ${selectedName}`
  return (
    <header className={styles.toolbar}>
      <div className={styles.toolbarTitle}>
        <MousePointer2 size={16} aria-hidden="true" />
        <span>{canvasName}</span>
        <span
          className={captureIssue
            ? styles.deviceStatusIssue
            : liveConnected ? styles.deviceStatusLive : styles.deviceStatus}
          title={captureIssue || deviceStatus}
        >
          {deviceStatus}{deviceLeaseLabel ? ` · ${deviceLeaseLabel}` : ''}
        </span>
      </div>
      <div className={styles.toolbarActions}>
        <input
          ref={screenshotInputRef}
          type="file"
          accept="image/*"
          className={styles.hiddenFileInput}
          onChange={(event) => {
            const file = event.currentTarget.files?.[0]
            event.currentTarget.value = ''
            if (file) onImportScreenshot(file)
          }}
        />
        <select
          className={styles.deviceSelect}
          value={selectedDeviceId}
          onChange={(event) => onSelectDevice(event.currentTarget.value)}
          aria-label="ADB 设备"
        >
          <option value="">ADB 设备</option>
          {devices.map((device) => (
            <option key={device.serial} value={device.serial}>
              {device.model ?? device.serial} · {deviceConnectionLabel(device)} · {device.state}
              {deviceLeases.find((lease) => lease.hardwareSerial === (device.hardwareSerial || device.serial))
                ? ` · ${deviceLeases.find((lease) => lease.hardwareSerial === (device.hardwareSerial || device.serial))?.ownerDisplayName} 使用中`
                : ''}
            </option>
          ))}
        </select>
        <button type="button" onClick={onRefreshDevices} disabled={deviceBusy}>
          <Smartphone size={14} aria-hidden="true" />
          {deviceBusy ? '检测中' : '设备'}
        </button>
        <button
          type="button"
          className={wirelessConnected ? styles.activeViewControl : ''}
          onClick={onOpenDeviceManager}
        >
          <Wifi size={14} aria-hidden="true" />
          {wirelessConnected ? '无线已连' : '无线连接'}
        </button>
        <button
          type="button"
          onClick={onCaptureDeviceSnapshot}
          disabled={captureBusy || deviceBusy || deviceLeaseBlocked}
          title={deviceLeaseBlocked ? '这台手机正被其他用户使用，请切换手机或等待自动释放' : '一台手机会自动识别；多台手机时请先在左侧选择'}
        >
          <Smartphone size={14} aria-hidden="true" />
          {captureBusy ? '读取真机中' : '调试真机'}
        </button>
        <div className={styles.viewControls} aria-label="画布缩放">
          <button type="button" onClick={onZoomOut} aria-label="缩小画布">
            <Minus size={14} aria-hidden="true" />
          </button>
          <strong>{viewScaleLabel}</strong>
          <button type="button" onClick={onZoomIn} aria-label="放大画布">
            <Plus size={14} aria-hidden="true" />
          </button>
          <button
            type="button"
            className={fitToStage ? styles.activeViewControl : ''}
            onClick={onFitToStage}
          >
            <Maximize2 size={14} aria-hidden="true" />
            适屏
          </button>
          <button type="button" onClick={onActualSize}>
            100%
          </button>
        </div>
        <button type="button" onClick={() => screenshotInputRef.current?.click()}>
          <ImagePlus size={14} aria-hidden="true" />
          导入设计图/截图
        </button>
        <div className={styles.workspaceControls} aria-label="工作区布局">
          <button
            type="button"
            className={leftPanelOpen ? styles.activeWorkspaceControl : ''}
            onClick={onToggleLeftPanel}
            aria-label={leftPanelOpen ? '隐藏组件树' : '显示组件树'}
            title={leftPanelOpen ? '隐藏组件树' : '显示组件树'}
          >
            {leftPanelOpen
              ? <PanelLeftClose size={14} aria-hidden="true" />
              : <PanelLeftOpen size={14} aria-hidden="true" />}
          </button>
          <button
            type="button"
            className={rightPanelOpen ? styles.activeWorkspaceControl : ''}
            onClick={onToggleRightPanel}
            aria-label={rightPanelOpen ? '隐藏属性栏' : '显示属性栏'}
            title={rightPanelOpen ? '隐藏属性栏' : '显示属性栏'}
          >
            {rightPanelOpen
              ? <PanelRightClose size={14} aria-hidden="true" />
              : <PanelRightOpen size={14} aria-hidden="true" />}
          </button>
          <button
            type="button"
            className={focusMode ? styles.activeWorkspaceControl : ''}
            onClick={onToggleFocusMode}
            aria-label={focusMode ? '退出专注画布' : '专注画布'}
            title={focusMode ? '退出专注画布' : '隐藏两侧面板，专注画布'}
          >
            <PanelsTopLeft size={14} aria-hidden="true" />
          </button>
        </div>
        <div className={styles.historyControls} aria-label="历史记录">
          <button type="button" onClick={onUndo} disabled={!canUndo} aria-label="撤回一步" title="撤回一步">
            <Undo2 size={14} aria-hidden="true" />
            撤回
          </button>
          <button type="button" onClick={onRedo} disabled={!canRedo} aria-label="重做一步" title="重做一步">
            <Redo2 size={14} aria-hidden="true" />
            重做
          </button>
        </div>
        <button type="button" onClick={onSave}>
          <Save size={14} aria-hidden="true" />
          本机草稿
        </button>
        <button type="button" onClick={onCopyExport}>
          <Copy size={14} aria-hidden="true" />
          复制参数
        </button>
        <button type="button" onClick={onCopyCliPatch}>
          <FileCode2 size={14} aria-hidden="true" />
          CLI 包
        </button>
        <button type="button" onClick={onCopyStandardPackage}>
          <Database size={14} aria-hidden="true" />
          标准草案
        </button>
        <button type="button" onClick={onDownloadExport} aria-label="下载参数 JSON">
          <Download size={15} aria-hidden="true" />
        </button>
        <button type="button" onClick={onReset} aria-label="重置画布">
          <RefreshCw size={15} aria-hidden="true" />
        </button>
      </div>
    </header>
  )
}
