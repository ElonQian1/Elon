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
  projectRoot?: string
  packageName?: string
}

function selectCaptureDevice(devices: AndroidInspectorDevice[], preferredId: string) {
  const preferred = devices.find((device) => device.serial === preferredId)
  if (preferred?.state === 'device') return preferred
  const readyDevices = devices.filter((device) => device.state === 'device')
  return readyDevices.length === 1 ? readyDevices[0] : null
}

function deviceDisplayName(device: AndroidInspectorDevice) {
  return device.model ?? device.serial
}

function devicePriority(device: AndroidInspectorDevice) {
  const stateScore = device.state === 'device' ? 100 : 0
  const connectionScore = device.connectionType === 'usb'
    ? 20
    : device.connectionType === 'wireless'
      ? 10
      : 0
  return stateScore + connectionScore
}

function normalizeDeviceInventory(devices: AndroidInspectorDevice[]) {
  const physicalDevices = new Map<string, AndroidInspectorDevice>()
  for (const device of devices) {
    const key = device.hardwareSerial?.trim() || device.serial
    const current = physicalDevices.get(key)
    if (!current || devicePriority(device) > devicePriority(current)) {
      physicalDevices.set(key, device)
    }
  }
  return [...physicalDevices.values()]
}

function captureFailureMessage(error: unknown) {
  const message = error instanceof Error ? error.message : String(error ?? '')
  const lower = message.toLowerCase()
  if (lower.includes('unauthorized')) {
    return '手机尚未授权：请解锁手机，勾选“始终允许这台电脑”，点击允许后重试'
  }
  if (lower.includes('offline') || lower.includes('device not found') || lower.includes('closed')) {
    return '手机连接已断开：请检查数据线或无线网络，然后再次点击“调试真机”'
  }
  return message || '真机捕获失败，请检查手机是否解锁并保持连接'
}

export function useAndroidInspectorDevices({
  onCaptured,
  onNotice,
  projectRoot,
  packageName,
}: UseAndroidInspectorDevicesOptions) {
  const [devices, setDevices] = useState<AndroidInspectorDevice[]>([])
  const [selectedDeviceId, setSelectedDeviceId] = useState('')
  const [deviceBusy, setDeviceBusy] = useState(false)
  const [captureBusy, setCaptureBusy] = useState(false)
  const [wirelessBusy, setWirelessBusy] = useState(false)
  const [deviceDialogOpen, setDeviceDialogOpen] = useState(false)
  const [wirelessStatus, setWirelessStatus] = useState<AndroidWirelessStatus | null>(null)

  const applyWirelessStatus = useCallback((status: AndroidWirelessStatus) => {
    const normalizedDevices = normalizeDeviceInventory(status.devices)
    setWirelessStatus(status)
    setDevices(normalizedDevices)
    setSelectedDeviceId((current) => {
      const connectedProfile = status.profiles.find((profile) => profile.connectedDeviceId)
      const profileDevice = connectedProfile
        ? normalizedDevices.find((device) => device.hardwareSerial === connectedProfile.hardwareSerial)
        : null
      if (profileDevice?.serial) return profileDevice.serial
      if (current && normalizedDevices.some((device) => device.serial === current)) return current
      const readyDevices = normalizedDevices.filter((device) => device.state === 'device')
      return readyDevices.length === 1 ? readyDevices[0].serial : ''
    })
  }, [])

  const refreshDevices = useCallback(async (announce = true): Promise<AndroidInspectorDevice[]> => {
    setDeviceBusy(true)
    try {
      const nextDevices = normalizeDeviceInventory(await listAndroidDevices())
      const readyDevices = nextDevices.filter((device) => device.state === 'device')
      setDevices(nextDevices)
      setSelectedDeviceId((current) => (
        current && nextDevices.some((device) => device.serial === current)
          ? current
          : readyDevices.length === 1 ? readyDevices[0].serial : ''
      ))
      if (announce) {
        const unauthorized = nextDevices.some((device) => device.state === 'unauthorized')
        if (readyDevices.length > 1) {
          onNotice(`检测到 ${readyDevices.length} 台手机，请先在设备下拉框选择要调试的手机`)
        } else if (readyDevices.length === 1) {
          onNotice(`已自动识别 ${deviceDisplayName(readyDevices[0])}`)
        } else if (unauthorized) {
          onNotice('已发现手机，但还未授权 USB 调试；请在手机弹窗中点击允许')
        } else {
          onNotice('未发现手机：请连接数据线、解锁手机并开启 USB 调试')
        }
      }
      return nextDevices
    } catch (error) {
      onNotice(error instanceof Error ? error.message : '读取 ADB 设备失败')
      return []
    } finally {
      setDeviceBusy(false)
    }
  }, [onNotice])

  const captureDeviceSnapshot = useCallback(async (override?: {
    deviceId?: string
    packageName?: string
  }) => {
    setCaptureBusy(true)
    try {
      onNotice('正在自动识别并读取真机画面…')
      const nextDevices = await refreshDevices(false)
      const readyDevices = nextDevices.filter((device) => device.state === 'device')
      const targetDevice = selectCaptureDevice(nextDevices, override?.deviceId || selectedDeviceId)
      if (!targetDevice) {
        if (readyDevices.length > 1) {
          setDeviceDialogOpen(true)
          onNotice(`检测到 ${readyDevices.length} 台手机，请选择一台后再次点击“调试真机”`)
        } else if (nextDevices.some((device) => device.state === 'unauthorized')) {
          onNotice('手机已连接但未授权：请解锁手机，勾选“始终允许这台电脑”并点击允许')
        } else {
          onNotice('未发现手机：请连接数据线、解锁手机并开启 USB 调试，然后直接重试')
        }
        return null
      }
      setSelectedDeviceId(targetDevice.serial)
      const snapshot = await captureAndroidSnapshot({
        deviceId: targetDevice.serial,
        packageName: override?.packageName || packageName || 'com.elon.app',
        projectRoot,
      })
      onCaptured(snapshot)
      return snapshot
    } catch (error) {
      onNotice(captureFailureMessage(error))
      return null
    } finally {
      setCaptureBusy(false)
    }
  }, [onCaptured, onNotice, packageName, projectRoot, refreshDevices, selectedDeviceId])

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
