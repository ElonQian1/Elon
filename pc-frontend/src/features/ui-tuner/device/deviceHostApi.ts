import { api } from '../../../api/client'
import type { AndroidInspectorDevice } from './deviceInspectorApi'

export interface ProjectAndroidDeviceHost {
  agentId: string
  displayName: string
  deviceName?: string
  version: string
  devices: AndroidInspectorDevice[]
}

export async function listProjectAndroidDeviceHosts(projectId: string) {
  const response = await api.get<{ hosts: ProjectAndroidDeviceHost[] }>(
    `/api/projects/${encodeURIComponent(projectId)}/modules/ui-tuner/android-device-hosts`,
  )
  return response.hosts
}
