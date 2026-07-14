import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  acquireAndroidDeviceLease,
  deviceLeaseError,
  heartbeatAndroidDeviceLease,
  leaseProof,
  listAndroidDeviceLeases,
  releaseAndroidDeviceLease,
  type AndroidDeviceLease,
  type AndroidDeviceLeaseProof,
} from './deviceLeaseApi'

const CLIENT_ID_KEY = 'elon.pc.uiTuner.deviceLeaseClient.v1'

function clientId() {
  const current = window.sessionStorage.getItem(CLIENT_ID_KEY)
  if (current) return current
  const created = `uit_${crypto.randomUUID().replace(/-/g, '')}`
  window.sessionStorage.setItem(CLIENT_ID_KEY, created)
  return created
}

export function useAndroidDeviceLease(
  projectId: string | undefined,
  hardwareSerial: string | undefined,
  onNotice: (message: string) => void,
) {
  const [leases, setLeases] = useState<AndroidDeviceLease[]>([])
  const [ownedLease, setOwnedLease] = useState<AndroidDeviceLease | null>(null)
  const [busy, setBusy] = useState(false)
  const ownedRef = useRef<AndroidDeviceLease | null>(null)
  ownedRef.current = ownedLease

  const refresh = useCallback(async () => {
    if (!projectId) {
      setLeases([])
      return []
    }
    try {
      const next = await listAndroidDeviceLeases(projectId)
      setLeases(next)
      const current = ownedRef.current
      if (current && !next.some((lease) => lease.leaseId === current.leaseId)) {
        setOwnedLease(null)
        onNotice('公共测试手机使用权已释放；实时修改已暂停，请重新取得使用权')
      }
      return next
    } catch {
      return []
    }
  }, [onNotice, projectId])

  useEffect(() => {
    void refresh()
    const timer = window.setInterval(() => { void refresh() }, 5_000)
    return () => window.clearInterval(timer)
  }, [refresh])

  useEffect(() => {
    const current = ownedRef.current
    if (current && (current.projectId !== projectId || current.hardwareSerial !== hardwareSerial)) {
      setOwnedLease(null)
      void releaseAndroidDeviceLease(current).catch(() => undefined)
    }
  }, [hardwareSerial, projectId])

  useEffect(() => {
    if (!ownedLease) return undefined
    const heartbeat = async () => {
      try {
        const renewed = await heartbeatAndroidDeviceLease(ownedLease)
        setOwnedLease(renewed)
        setLeases((current) => current.map((lease) => lease.leaseId === renewed.leaseId ? renewed : lease))
      } catch (error) {
        setOwnedLease(null)
        onNotice(deviceLeaseError(error))
      }
    }
    const timer = window.setInterval(() => { void heartbeat() }, 10_000)
    return () => window.clearInterval(timer)
  }, [onNotice, ownedLease?.leaseId])

  useEffect(() => () => {
    const current = ownedRef.current
    if (current) void releaseAndroidDeviceLease(current).catch(() => undefined)
  }, [])

  const ensureLease = useCallback(async (serial?: string): Promise<AndroidDeviceLeaseProof> => {
    const target = serial?.trim() || hardwareSerial?.trim()
    if (!projectId) throw new Error('请先选择项目，再使用公共测试手机')
    if (!target) throw new Error('无法识别手机硬件序列号，请刷新设备或重新登记')
    const current = ownedRef.current
    if (current && current.projectId === projectId && current.hardwareSerial === target) {
      return leaseProof(current)
    }
    setBusy(true)
    try {
      const acquired = await acquireAndroidDeviceLease(projectId, target, clientId())
      setOwnedLease(acquired)
      setLeases((items) => [...items.filter((item) => item.hardwareSerial !== target), acquired])
      onNotice(`已取得 ${acquired.hardwareSerial} 的使用权；离开或断线后会自动释放`)
      return leaseProof(acquired)
    } catch (error) {
      void refresh()
      throw new Error(deviceLeaseError(error))
    } finally {
      setBusy(false)
    }
  }, [hardwareSerial, onNotice, projectId, refresh])

  const release = useCallback(async () => {
    const current = ownedRef.current
    if (!current) return
    setOwnedLease(null)
    await releaseAndroidDeviceLease(current).catch(() => undefined)
    await refresh()
  }, [refresh])

  const activeLease = useMemo(
    () => leases.find((lease) => lease.hardwareSerial === hardwareSerial) ?? null,
    [hardwareSerial, leases],
  )
  const proof = ownedLease && ownedLease.hardwareSerial === hardwareSerial
    ? leaseProof(ownedLease)
    : undefined

  return { leases, activeLease, ownedLease, proof, busy, ensureLease, release, refresh }
}
