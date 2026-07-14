import { useCallback, useEffect, useState } from 'react'
import type { AndroidDeviceProfile } from './deviceInspectorApi'
import {
  listProjectSharedAndroidDevices,
  removeSharedAndroidDevice,
  shareAndroidDeviceWithProject,
} from './sharedDeviceApi'

export function useProjectSharedAndroidDevices(
  projectId: string | undefined,
  onNotice: (message: string) => void,
) {
  const [busy, setBusy] = useState(false)
  const [hardwareSerials, setHardwareSerials] = useState<string[]>([])

  useEffect(() => {
    let cancelled = false
    if (!projectId) {
      setHardwareSerials([])
      return () => { cancelled = true }
    }
    void listProjectSharedAndroidDevices(projectId)
      .then((devices) => {
        if (!cancelled) setHardwareSerials(devices.map((device) => device.hardwareSerial))
      })
      .catch(() => {
        if (!cancelled) setHardwareSerials([])
      })
    return () => { cancelled = true }
  }, [projectId])

  const toggle = useCallback(async (profile: AndroidDeviceProfile, shared: boolean) => {
    if (!projectId) {
      onNotice('请先选择一个项目，再共享测试手机')
      return
    }
    setBusy(true)
    try {
      if (shared) {
        await removeSharedAndroidDevice(projectId, profile.hardwareSerial)
        setHardwareSerials((current) => current.filter((serial) => serial !== profile.hardwareSerial))
        onNotice(`已停止向当前项目共享 ${profile.displayName}；其他项目和本机档案不受影响`)
      } else {
        await shareAndroidDeviceWithProject(projectId, profile)
        setHardwareSerials((current) => current.includes(profile.hardwareSerial)
          ? current
          : [...current, profile.hardwareSerial])
        onNotice(`已把 ${profile.displayName} 共享到当前项目；获授权的 PC 节点将自动记住并尝试重连`)
      }
    } catch (error) {
      onNotice(error instanceof Error ? error.message : '更新项目测试手机失败')
    } finally {
      setBusy(false)
    }
  }, [onNotice, projectId])

  return { busy, hardwareSerials, toggle }
}
