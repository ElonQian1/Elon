import { useCallback, useEffect, useState } from 'react'
import {
  captureAndroidSnapshot,
  connectAndroidDevice,
  enableAndroidTcpIp,
  forgetAndroidDevice,
  getAndroidWirelessStatus,
  listAndroidDevices,
  pairAndroidDevice,
  reconnectAndroidDevices,
  registerAndroidDevice,
  type AndroidInspectorDevice,
  type AndroidInspectorSnapshot,
  type AndroidWirelessStatus,
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
  const [deviceBusy, setDeviceBusy] = useState(false)
  const [captureBusy, setCaptureBusy] = useState(false)
  const [wirelessBusy, setWirelessBusy] = useState(false)
  const [deviceDialogOpen, setDeviceDialogOpen] = useState(false)
  const [wirelessStatus, setWirelessStatus] = useState<AndroidWirelessStatus | null>(null)

  const applyWirelessStatus = useCallback((status: AndroidWirelessStatus) => {
    setWirelessStatus(status)
    setDevices(status.devices)
    setSelectedDeviceId((current) => {
      const connectedProfile = status.profiles.find((profile) => profile.connectedDeviceId)
      if (connectedProfile?.connectedDeviceId) return connectedProfile.connectedDeviceId
      if (current && status.devices.some((device) => device.serial === current)) return current
      return status.devices.find((device) => device.state === 'device')?.serial
        ?? status.devices[0]?.serial
        ?? ''
    })
  }, [])

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

  const refreshWirelessStatus = useCallback(async () => {
    setWirelessBusy(true)
    try {
      const status = await getAndroidWirelessStatus()
      applyWirelessStatus(status)
      return status
    } catch (error) {
      onNotice(error instanceof Error ? error.message : '读取无线 ADB 状态失败')
      return null
    } finally {
      setWirelessBusy(false)
    }
  }, [applyWirelessStatus, onNotice])

  const openDeviceManager = useCallback(() => {
    setDeviceDialogOpen(true)
    void refreshWirelessStatus()
  }, [refreshWirelessStatus])

  const reconnectWirelessDevices = useCallback(async (profileId?: string, announce = true) => {
    setWirelessBusy(true)
    try {
      const status = await reconnectAndroidDevices(profileId)
      applyWirelessStatus(status)
      const connected = status.profiles.find((profile) => (
        profile.connectionState === 'connected_wireless'
        && (!profileId || profile.id === profileId)
      ))
      if (announce) {
        onNotice(connected ? `已无线连接 ${connected.displayName}` : '未发现已配对的在线手机')
      }
      return status
    } catch (error) {
      if (announce) onNotice(error instanceof Error ? error.message : '自动重连无线 ADB 失败')
      return null
    } finally {
      setWirelessBusy(false)
    }
  }, [applyWirelessStatus, onNotice])

  useEffect(() => {
    void reconnectWirelessDevices(undefined, false)
  }, [reconnectWirelessDevices])

  const registerWiredDevice = useCallback(async (deviceId: string, displayName?: string) => {
    setWirelessBusy(true)
    try {
      const result = await registerAndroidDevice({ deviceId, displayName })
      applyWirelessStatus(result.status)
      onNotice(`已记住 ${result.profile.displayName}，可以开始无线配对`)
      return result.profile
    } catch (error) {
      onNotice(error instanceof Error ? error.message : '登记手机失败')
      return null
    } finally {
      setWirelessBusy(false)
    }
  }, [applyWirelessStatus, onNotice])

  const pairWirelessDevice = useCallback(async (input: {
    pairingAddress: string
    pairingCode: string
    profileId?: string
  }) => {
    setWirelessBusy(true)
    try {
      const result = await pairAndroidDevice(input)
      applyWirelessStatus(result.status)
      const profile = result.status.profiles.find((item) => item.id === input.profileId)
      onNotice(profile?.connectionState === 'connected_wireless'
        ? `无线配对成功，已连接 ${profile.displayName}`
        : '无线配对成功，正在等待手机出现在当前网络')
      return true
    } catch (error) {
      onNotice(error instanceof Error ? error.message : '无线 ADB 配对失败')
      return false
    } finally {
      setWirelessBusy(false)
    }
  }, [applyWirelessStatus, onNotice])

  const enableLegacyWireless = useCallback(async (deviceId: string, profileId?: string) => {
    setWirelessBusy(true)
    try {
      const result = await enableAndroidTcpIp({ deviceId, profileId, port: 5555 })
      applyWirelessStatus(result.status)
      onNotice('传统无线 ADB 已启用；手机重启后可能需要再次插线开启')
      return true
    } catch (error) {
      onNotice(error instanceof Error ? error.message : '启用传统无线 ADB 失败')
      return false
    } finally {
      setWirelessBusy(false)
    }
  }, [applyWirelessStatus, onNotice])

  const connectWirelessAddress = useCallback(async (address: string, profileId?: string) => {
    setWirelessBusy(true)
    try {
      const output = await connectAndroidDevice(address, profileId)
      onNotice(output.trim() || `已连接 ${address}`)
      const status = await reconnectAndroidDevices(profileId)
      applyWirelessStatus(status)
      return true
    } catch (error) {
      onNotice(error instanceof Error ? error.message : '无线 ADB 连接失败')
      return false
    } finally {
      setWirelessBusy(false)
    }
  }, [applyWirelessStatus, onNotice])

  const forgetWirelessDevice = useCallback(async (profileId: string) => {
    setWirelessBusy(true)
    try {
      await forgetAndroidDevice(profileId)
      const status = await getAndroidWirelessStatus()
      applyWirelessStatus(status)
      onNotice('已移除本机设备档案；如需彻底撤销，请同时在手机无线调试中忘记此电脑')
    } catch (error) {
      onNotice(error instanceof Error ? error.message : '移除手机档案失败')
    } finally {
      setWirelessBusy(false)
    }
  }, [applyWirelessStatus, onNotice])

  return {
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
  }
}
