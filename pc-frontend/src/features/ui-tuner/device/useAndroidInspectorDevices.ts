import { useCallback, useEffect, useState } from 'react'
import { LOCAL_NODE_BASE_CHANGED_EVENT } from '../../../api/runtime'
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
import type { AndroidDeviceLeaseProof } from './deviceLeaseApi'

interface UseAndroidInspectorDevicesOptions {
  onCaptured: (snapshot: AndroidInspectorSnapshot) => void
  onNotice: (message: string) => void
  projectRoot?: string
  packageName?: string
  ensureLease?: (hardwareSerial: string) => Promise<AndroidDeviceLeaseProof>
}

const SELECTED_DEVICE_STORAGE_KEY = 'elon.pc.uiTuner.selectedAndroidDevice.v1'
const DEBUG_PACKAGE = 'com.elon.app.uituner'
const DEFAULT_PACKAGE = 'com.elon.app'

function deviceIdentity(device: AndroidInspectorDevice) {
  return device.hardwareSerial?.trim() || device.serial
}

function rememberedDeviceIdentity() {
  return typeof window === 'undefined' ? '' : window.localStorage.getItem(SELECTED_DEVICE_STORAGE_KEY) ?? ''
}

function chooseDeviceId(devices: AndroidInspectorDevice[], current: string) {
  const ready = devices.filter((device) => device.state === 'device')
  const currentDevice = ready.find((device) => device.serial === current)
  if (currentDevice) return currentDevice.serial
  const remembered = rememberedDeviceIdentity()
  const rememberedDevice = ready.find((device) => deviceIdentity(device) === remembered)
  return rememberedDevice?.serial ?? (ready.length === 1 ? ready[0].serial : '')
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
  // 真机调试优先使用同一硬件的无线 transport。USB 插拔或 adb tcpip
  // 切换时，有线 serial 可能在刷新后瞬间消失，无线 endpoint 更稳定。
  const connectionScore = device.connectionType === 'wireless'
    ? 20
    : device.connectionType === 'usb'
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

function isSystemOverlay(snapshot: AndroidInspectorSnapshot) {
  const activity = snapshot.activityName?.toLowerCase() ?? ''
  return activity.includes('notificationshade')
    || activity.includes('statusbar')
    || activity.includes('keyguard')
    || activity.includes('systemui')
}

function foregroundPackage(snapshot: AndroidInspectorSnapshot) {
  return snapshot.activityName
    ?.match(/([A-Za-z0-9_.]+)\/[A-Za-z0-9_.$]+/)?.[1]
}

export function useAndroidInspectorDevices({
  onCaptured,
  onNotice,
  projectRoot,
  packageName,
  ensureLease,
}: UseAndroidInspectorDevicesOptions) {
  const [devices, setDevices] = useState<AndroidInspectorDevice[]>([])
  const [selectedDeviceId, setSelectedDeviceId] = useState('')
  const [deviceBusy, setDeviceBusy] = useState(false)
  const [captureBusy, setCaptureBusy] = useState(false)
  const [captureIssue, setCaptureIssue] = useState('')
  const [wirelessBusy, setWirelessBusy] = useState(false)
  const [deviceDialogOpen, setDeviceDialogOpen] = useState(false)
  const [wirelessStatus, setWirelessStatus] = useState<AndroidWirelessStatus | null>(null)

  const applyWirelessStatus = useCallback((status: AndroidWirelessStatus) => {
    const normalizedDevices = normalizeDeviceInventory(status.devices)
    setWirelessStatus(status)
    setDevices(normalizedDevices)
    setSelectedDeviceId((current) => chooseDeviceId(normalizedDevices, current))
  }, [])

  const refreshDevices = useCallback(async (announce = true): Promise<AndroidInspectorDevice[]> => {
    setDeviceBusy(true)
    try {
      const nextDevices = normalizeDeviceInventory(await listAndroidDevices())
      const readyDevices = nextDevices.filter((device) => device.state === 'device')
      setDevices(nextDevices)
      setSelectedDeviceId((current) => chooseDeviceId(nextDevices, current))
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
    launchApp?: boolean
  }) => {
    setCaptureBusy(true)
    setCaptureIssue('')
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
      const identity = deviceIdentity(targetDevice)
      const lease = override?.launchApp === false ? undefined : await ensureLease?.(identity)
      window.localStorage.setItem(SELECTED_DEVICE_STORAGE_KEY, identity)
      setSelectedDeviceId(targetDevice.serial)
      const preferredPackage = override?.packageName || packageName || DEBUG_PACKAGE
      let snapshot: AndroidInspectorSnapshot
      try {
        snapshot = await captureAndroidSnapshot({
          deviceId: targetDevice.serial,
          packageName: preferredPackage,
          launchApp: override?.launchApp ?? true,
          projectRoot,
          lease,
        })
        if (preferredPackage !== DEFAULT_PACKAGE && foregroundPackage(snapshot) !== preferredPackage) {
          throw new Error(`${preferredPackage} 未在前台运行`)
        }
      } catch (error) {
        if (preferredPackage === DEFAULT_PACKAGE) throw error
        snapshot = await captureAndroidSnapshot({
          deviceId: targetDevice.serial,
          packageName: DEFAULT_PACKAGE,
          launchApp: true,
          projectRoot,
          lease,
        })
      }
      if (isSystemOverlay(snapshot)) {
        const message = `已连接 ${deviceDisplayName(targetDevice)}，但手机当前停在锁屏或通知栏；请解锁并返回调试 APP 后重试`
        setCaptureIssue(message)
        onNotice(message)
        return null
      }
      const actualPackage = foregroundPackage(snapshot)
      if (actualPackage && actualPackage !== DEFAULT_PACKAGE && preferredPackage !== actualPackage) {
        const message = `已连接 ${deviceDisplayName(targetDevice)}，但没有打开一龙调试 APP；请在右侧点击“构建并安装实时调试包”`
        setCaptureIssue(message)
        onNotice(message)
        return null
      }
      onCaptured(snapshot)
      return snapshot
    } catch (error) {
      const message = captureFailureMessage(error)
      setCaptureIssue(message)
      onNotice(message)
      return null
    } finally {
      setCaptureBusy(false)
    }
  }, [ensureLease, onCaptured, onNotice, packageName, projectRoot, refreshDevices, selectedDeviceId])

  const selectDevice = useCallback((deviceId: string) => {
    const selected = devices.find((device) => device.serial === deviceId)
    if (selected) window.localStorage.setItem(SELECTED_DEVICE_STORAGE_KEY, deviceIdentity(selected))
    else if (!deviceId) window.localStorage.removeItem(SELECTED_DEVICE_STORAGE_KEY)
    setSelectedDeviceId(deviceId)
  }, [devices])

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
    const discoverDevices = async () => {
      const status = await refreshWirelessStatus()
      if (status && !status.devices.some((device) => device.state === 'device')) {
        await reconnectWirelessDevices(undefined, false)
      }
    }
    void discoverDevices()
    const reconnectAfterNodeDiscovery = () => {
      void discoverDevices()
    }
    window.addEventListener(LOCAL_NODE_BASE_CHANGED_EVENT, reconnectAfterNodeDiscovery)
    return () => {
      window.removeEventListener(LOCAL_NODE_BASE_CHANGED_EVENT, reconnectAfterNodeDiscovery)
    }
  }, [reconnectWirelessDevices, refreshWirelessStatus])

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
    captureIssue,
    wirelessBusy,
    deviceDialogOpen,
    wirelessStatus,
    selectDevice,
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
