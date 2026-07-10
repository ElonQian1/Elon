import { useCallback, useState } from 'react'
import {
  captureAndroidSnapshot,
  connectAndroidDevice,
  listAndroidDevices,
  type AndroidInspectorDevice,
  type AndroidInspectorSnapshot,
} from './deviceInspectorApi'

interface UseAndroidInspectorDevicesOptions {
  onCaptured: (snapshot: AndroidInspectorSnapshot) => void
  onNotice: (message: string) => void
}

function selectCaptureDevice(devices: AndroidInspectorDevice[], preferredId: string) {
  const preferred = devices.find((device) => device.serial === preferredId)
  if (preferred?.state === 'device') return preferred
  return devices.find((device) => device.state === 'device') ?? preferred ?? devices[0] ?? null
}

function deviceDisplayName(device: AndroidInspectorDevice) {
  return device.model ?? device.serial
}

export function useAndroidInspectorDevices({
  onCaptured,
  onNotice,
}: UseAndroidInspectorDevicesOptions) {
  const [devices, setDevices] = useState<AndroidInspectorDevice[]>([])
  const [selectedDeviceId, setSelectedDeviceId] = useState('')
  const [connectAddress, setConnectAddress] = useState('')
  const [deviceBusy, setDeviceBusy] = useState(false)
  const [connectBusy, setConnectBusy] = useState(false)
  const [captureBusy, setCaptureBusy] = useState(false)

  const refreshDevices = useCallback(async (): Promise<AndroidInspectorDevice[]> => {
    setDeviceBusy(true)
    try {
      const nextDevices = await listAndroidDevices()
      setDevices(nextDevices)
      setSelectedDeviceId((current) => (
        current && nextDevices.some((device) => device.serial === current)
          ? current
          : nextDevices.find((device) => device.state === 'device')?.serial ?? nextDevices[0]?.serial ?? ''
      ))
      onNotice(nextDevices.length ? `已发现 ${nextDevices.length} 台 ADB 设备` : '未发现可用 ADB 设备')
      return nextDevices
    } catch (error) {
      onNotice(error instanceof Error ? error.message : '读取 ADB 设备失败')
      return []
    } finally {
      setDeviceBusy(false)
    }
  }, [onNotice])

  const captureDeviceSnapshot = useCallback(async () => {
    setCaptureBusy(true)
    try {
      let targetDevice = selectCaptureDevice(devices, selectedDeviceId)
      if (!targetDevice || targetDevice.state !== 'device') {
        onNotice('正在检测 ADB 设备并准备捕获')
        const nextDevices = await refreshDevices()
        targetDevice = selectCaptureDevice(nextDevices, selectedDeviceId)
      }
      if (!targetDevice) {
        onNotice('未发现可用 ADB 设备，请先连接手机并确认 USB 调试授权')
        return
      }
      setSelectedDeviceId(targetDevice.serial)
      if (targetDevice.state !== 'device') {
        onNotice(`ADB 设备未就绪：${deviceDisplayName(targetDevice)} · ${targetDevice.state}`)
        return
      }
      const snapshot = await captureAndroidSnapshot({
        deviceId: targetDevice.serial,
        packageName: 'com.elon.app',
      })
      onCaptured(snapshot)
    } catch (error) {
      onNotice(error instanceof Error ? error.message : '真机捕获失败')
    } finally {
      setCaptureBusy(false)
    }
  }, [devices, onCaptured, onNotice, refreshDevices, selectedDeviceId])

  const connectWirelessDevice = useCallback(async () => {
    const address = connectAddress.trim()
    if (!address) return
    setConnectBusy(true)
    try {
      const output = await connectAndroidDevice(address)
      onNotice(output.trim() || `已连接 ${address}`)
      await refreshDevices()
    } catch (error) {
      onNotice(error instanceof Error ? error.message : '无线 ADB 连接失败')
    } finally {
      setConnectBusy(false)
    }
  }, [connectAddress, onNotice, refreshDevices])

  return {
    devices,
    selectedDeviceId,
    connectAddress,
    deviceBusy,
    connectBusy,
    captureBusy,
    setSelectedDeviceId,
    setConnectAddress,
    refreshDevices,
    connectWirelessDevice,
    captureDeviceSnapshot,
  }
}
