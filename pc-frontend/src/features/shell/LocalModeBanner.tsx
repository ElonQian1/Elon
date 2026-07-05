import { useEffect, useState } from 'react'
import { MonitorCheck, RefreshCw, WifiOff } from 'lucide-react'
import { cloudBaseUrl, cloudWorkbenchUrl, isLocalWorkbench, localNodeUrl } from '../../api/runtime'
import styles from './Shell.module.css'

type CloudState = 'checking' | 'online' | 'offline'

interface LocalStatus {
  connected?: boolean
  last_event?: string
  cloud_http_url?: string
}

export default function LocalModeBanner() {
  const [cloudState, setCloudState] = useState<CloudState>('checking')
  const [localStatus, setLocalStatus] = useState<LocalStatus | null>(null)
  const localMode = isLocalWorkbench()

  useEffect(() => {
    let cancelled = false
    async function refresh() {
      if (!localMode) {
        const cloudOk = await probeCloudHealth()
        if (cancelled) return
        setCloudState(cloudOk ? 'online' : 'offline')
        setLocalStatus(null)
        return
      }
      const [cloudOk, status] = await Promise.all([
        probeCloudHealth(),
        probeLocalStatus(),
      ])
      if (cancelled) return
      setCloudState(cloudOk ? 'online' : 'offline')
      setLocalStatus(status)
    }
    refresh()
    const timer = setInterval(refresh, 12_000)
    return () => {
      cancelled = true
      clearInterval(timer)
    }
  }, [localMode])

  useEffect(() => {
    if (!localMode || cloudState !== 'online') return
    const target = cloudWorkbenchUrl()
    if (new URL(target).origin === location.origin) return
    window.location.replace(target)
  }, [cloudState, localMode])

  if (!localMode) {
    if (cloudState !== 'offline') return null
    return (
      <div className={[styles.nodeBanner, styles.localModeOffline].join(' ')}>
        <WifiOff className={styles.nodeBannerIcon} aria-hidden="true" size={14} />
        <span>云端连接异常 · 当前显示的是一龙 PC 工作台缓存壳，本机 Win 端可用于诊断网络或防火墙问题。</span>
        <a href={localNodeUrl('/pc')}>打开本机工作台</a>
      </div>
    )
  }

  const nodeConnected = localStatus?.connected !== false
  const bannerClass = [
    styles.nodeBanner,
    cloudState === 'offline' ? styles.localModeOffline : styles.localModeBanner,
  ].join(' ')
  const Icon = cloudState === 'offline' ? WifiOff : cloudState === 'checking' ? RefreshCw : MonitorCheck
  const cloudHost = hostLabel(localStatus?.cloud_http_url || cloudBaseUrl())
  const copy = cloudState === 'offline'
    ? `本地模式 · 工作台由这台电脑提供，云端 ${cloudHost} 暂时不可达。`
    : cloudState === 'checking'
      ? '本地模式 · 正在确认云端连接…'
      : nodeConnected
        ? `本地模式 · 本机节点正常，云端 ${cloudHost} 已连接。`
        : `本地模式 · 本机工作台正常，正在等待云端 ${cloudHost} 恢复连接。`

  return (
    <div className={bannerClass}>
      <Icon className={styles.nodeBannerIcon} aria-hidden="true" size={14} />
      <span title={localStatus?.last_event || copy}>{copy}</span>
    </div>
  )
}

async function probeCloudHealth(): Promise<boolean> {
  const ctrl = new AbortController()
  const timer = setTimeout(() => ctrl.abort(), 2600)
  try {
    const res = await fetch(new URL('/health', cloudBaseUrl()).toString(), {
      cache: 'no-store',
      signal: ctrl.signal,
    })
    return res.ok
  } catch {
    return false
  } finally {
    clearTimeout(timer)
  }
}

async function probeLocalStatus(): Promise<LocalStatus | null> {
  const ctrl = new AbortController()
  const timer = setTimeout(() => ctrl.abort(), 2000)
  try {
    const res = await fetch(localNodeUrl('/api/status'), {
      cache: 'no-store',
      signal: ctrl.signal,
    })
    if (!res.ok) return null
    return await res.json() as LocalStatus
  } catch {
    return null
  } finally {
    clearTimeout(timer)
  }
}

function hostLabel(raw: string): string {
  try {
    return new URL(raw).host
  } catch {
    return raw
  }
}
