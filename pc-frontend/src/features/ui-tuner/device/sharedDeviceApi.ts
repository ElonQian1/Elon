import { api } from '../../../api/client'
import type { AndroidDeviceProfile } from './deviceInspectorApi'

export interface ProjectSharedAndroidDevice {
  projectId: string
  hardwareSerial: string
  displayName: string
  manufacturer?: string
  model?: string
  androidSdk?: number
  androidRelease?: string
  lastEndpoint: string
  wirelessMode: AndroidDeviceProfile['wirelessMode']
  updatedAt: string
}

function base(projectId: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/modules/ui-tuner/shared-android-devices`
}

export async function listProjectSharedAndroidDevices(projectId: string) {
  const response = await api.get<{ devices: ProjectSharedAndroidDevice[] }>(base(projectId))
  return response.devices
}

export function shareAndroidDeviceWithProject(
  projectId: string,
  profile: AndroidDeviceProfile,
) {
  if (!profile.lastEndpoint) throw new Error('这台手机还没有无线地址，请先完成无线连接')
  return api.post<ProjectSharedAndroidDevice>(base(projectId), {
    hardwareSerial: profile.hardwareSerial,
    displayName: profile.displayName,
    manufacturer: profile.manufacturer,
    model: profile.model,
    androidSdk: profile.androidSdk,
    androidRelease: profile.androidRelease,
    lastEndpoint: profile.lastEndpoint,
    wirelessMode: profile.wirelessMode,
  })
}

export function removeSharedAndroidDevice(projectId: string, hardwareSerial: string) {
  return api.delete<{ removed: boolean }>(
    `${base(projectId)}/${encodeURIComponent(hardwareSerial)}`,
  )
}
