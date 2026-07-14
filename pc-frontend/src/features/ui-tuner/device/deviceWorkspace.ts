import { useEffect, useMemo, useRef, type MutableRefObject } from 'react'
import { loadUiTunerDeviceDocument, saveUiTunerDeviceDocument } from '../uiTunerStorage'
import type { UiTunerDocument } from '../types'
import type { AndroidInspectorDevice, AndroidInspectorSnapshot } from './deviceInspectorApi'

export const UI_TUNER_DEBUG_PACKAGE = 'com.elon.app.uituner'

export function androidDeviceIdentity(device: AndroidInspectorDevice) {
  return device.hardwareSerial?.trim() || device.serial
}

export function createWaitingDeviceDocument(device: AndroidInspectorDevice): UiTunerDocument {
  const name = device.model?.trim() || device.serial
  return {
    version: 1,
    canvas: {
      name: `${name} · 等待真机画面`,
      width: 1080,
      height: 2400,
      background: '#050607',
    },
    elements: [],
    source: {
      kind: 'device_snapshot',
      label: `${name} · 等待真机画面`,
    },
    updatedAt: new Date().toISOString(),
  }
}

interface DeviceWorkspaceOptions {
  devices: AndroidInspectorDevice[]
  selectedDeviceId: string
  documentRef: MutableRefObject<UiTunerDocument>
  onLoadDocument: (document: UiTunerDocument) => void
  onNotice: (message: string) => void
  capture: (override: {
    deviceId: string
    packageName: string
    launchApp: boolean
  }) => Promise<AndroidInspectorSnapshot | null>
}

export function useUiTunerDeviceWorkspace({
  devices,
  selectedDeviceId,
  documentRef,
  onLoadDocument,
  onNotice,
  capture,
}: DeviceWorkspaceOptions) {
  const workspaceDeviceRef = useRef('')
  const selectedDevice = useMemo(
    () => devices.find((device) => device.serial === selectedDeviceId) ?? null,
    [devices, selectedDeviceId],
  )
  const identity = selectedDevice ? androidDeviceIdentity(selectedDevice) : ''

  useEffect(() => {
    if (!selectedDevice || !identity || workspaceDeviceRef.current === identity) return
    if (workspaceDeviceRef.current) {
      saveUiTunerDeviceDocument(workspaceDeviceRef.current, documentRef.current)
    }
    workspaceDeviceRef.current = identity
    onLoadDocument(loadUiTunerDeviceDocument(identity) ?? createWaitingDeviceDocument(selectedDevice))
    onNotice(`已切换到 ${selectedDevice.model ?? selectedDevice.serial}，正在读取这台手机…`)
    void capture({
      deviceId: selectedDevice.serial,
      packageName: UI_TUNER_DEBUG_PACKAGE,
      launchApp: true,
    })
  }, [capture, documentRef, identity, onLoadDocument, onNotice, selectedDevice])

  return { selectedDevice, selectedDeviceIdentity: identity }
}
