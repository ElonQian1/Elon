import { api, type ApiError } from '../../../api/client'

export interface AndroidDeviceLease {
  leaseId: string
  projectId: string
  hardwareSerial: string
  ownerUserId: string
  ownerDisplayName: string
  clientInstanceId: string
  createdAt: string
  heartbeatAt: string
  expiresAt: string
}

export interface AndroidDeviceLeaseProof {
  leaseId: string
  projectId: string
  hardwareSerial: string
}

function base(projectId: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/modules/ui-tuner/android-device-leases`
}

export async function listAndroidDeviceLeases(projectId: string) {
  const response = await api.get<{ leases: AndroidDeviceLease[] }>(base(projectId))
  return response.leases
}

export function acquireAndroidDeviceLease(
  projectId: string,
  hardwareSerial: string,
  clientInstanceId: string,
) {
  return api.post<AndroidDeviceLease>(
    `${base(projectId)}/${encodeURIComponent(hardwareSerial)}/acquire`,
    { clientInstanceId },
  )
}

export function heartbeatAndroidDeviceLease(lease: AndroidDeviceLease) {
  return api.post<AndroidDeviceLease>(
    `${base(lease.projectId)}/${encodeURIComponent(lease.hardwareSerial)}/heartbeat`,
    { leaseId: lease.leaseId, clientInstanceId: lease.clientInstanceId },
  )
}

export function releaseAndroidDeviceLease(lease: AndroidDeviceLease) {
  return api.post<{ released: boolean }>(
    `${base(lease.projectId)}/${encodeURIComponent(lease.hardwareSerial)}/release`,
    { leaseId: lease.leaseId, clientInstanceId: lease.clientInstanceId },
  )
}

export function deviceLeaseError(error: unknown) {
  const apiError = error as Partial<ApiError> | null
  return apiError?.message || (error instanceof Error ? error.message : '无法取得公共测试手机使用权')
}

export function leaseProof(lease: AndroidDeviceLease): AndroidDeviceLeaseProof {
  return {
    leaseId: lease.leaseId,
    projectId: lease.projectId,
    hardwareSerial: lease.hardwareSerial,
  }
}
