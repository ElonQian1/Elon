import { useCallback, useEffect, useMemo, useState } from 'react'
import { nodeApi, probeLocalNode } from './localNodeApi'
import { fetchNodeAgentVersion, formatBytes } from './nodeHelpers'
import type { ClientMaintenanceStatus, LocalNodeStatus, NodeAgentVersion } from './types'
import { buildWinClientUpdateState } from './winClientUpdateModel'
import styles from './NodePage.module.css'

interface Props {
  adminUrl: string
  status: LocalNodeStatus
  onStatus: (status: LocalNodeStatus) => void
}

export default function NodeClientUpdateCard({ adminUrl, status, onStatus }: Props) {
  const [latest, setLatest] = useState<NodeAgentVersion | null>(null)
  const [maintenance, setMaintenance] = useState<ClientMaintenanceStatus | null>(null)
  const [busy, setBusy] = useState(false)
  const [polling, setPolling] = useState(false)
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')

  const load = useCallback(async (quiet = true) => {
    if (!quiet) { setMessage('正在检查客户端版本…'); setError('') }
    const [latestResult, maintenanceResult] = await Promise.allSettled([
      fetchNodeAgentVersion(),
      nodeApi<ClientMaintenanceStatus>(adminUrl, '/api/client-maintenance', {}, 10000),
    ])
    if (latestResult.status === 'fulfilled') setLatest(latestResult.value)
    if (maintenanceResult.status === 'fulfilled') {
      setMaintenance(maintenanceResult.value)
      if (!quiet) setMessage('客户端维护状态已刷新。')
    } else if (!quiet) {
      setError(maintenanceResult.reason instanceof Error ? maintenanceResult.reason.message : '读取客户端维护状态失败。')
    }
  }, [adminUrl])

  useEffect(() => {
    load()
    const timer = window.setInterval(() => load(true), 120_000)
    return () => window.clearInterval(timer)
  }, [load])

  const localVersion = maintenance?.installed_package_version || status.version || maintenance?.version || ''
  const localGitSha = maintenance?.installed_git_sha || maintenance?.version_manifest?.gitSha || ''
  const updateState = useMemo(
    () => buildWinClientUpdateState(localVersion, localGitSha, latest),
    [localVersion, localGitSha, latest],
  )
  const updateAction = maintenance?.maintenance_actions?.find((action) => action.kind === 'update' || action.id === 'check_update')
  const canUpdate = updateAction?.enabled !== false && maintenance?.supported !== false

  useEffect(() => {
    if (!polling) return
    let canceled = false
    let attempts = 0
    const timer = window.setInterval(async () => {
      attempts += 1
      try {
        const nextStatus = await probeLocalNode(adminUrl) as LocalNodeStatus
        if (canceled) return
        onStatus(nextStatus)
        const nextMaintenance = await nodeApi<ClientMaintenanceStatus>(adminUrl, '/api/client-maintenance', {}, 10000)
        if (canceled) return
        setMaintenance(nextMaintenance)
        const nextLatest = latest ?? await fetchNodeAgentVersion().catch(() => null)
        if (nextLatest) setLatest(nextLatest)
        const nextState = buildWinClientUpdateState(
          nextMaintenance.installed_package_version || nextStatus.version || nextMaintenance.version || '',
          nextMaintenance.installed_git_sha || nextMaintenance.version_manifest?.gitSha || '',
          nextLatest,
        )
        if (nextState.kind !== 'update') {
          setPolling(false)
          setBusy(false)
          setMessage('Win 端更新完成，节点已重新连接。')
        } else if (attempts >= 30) {
          setPolling(false)
          setBusy(false)
          setMessage('更新已触发，但还没确认到新版本；请稍后手动刷新状态。')
        } else {
          setMessage('Win 端正在更新升级，通信临时中断，会自动恢复。')
        }
      } catch {
        if (attempts >= 30) {
          setPolling(false)
          setBusy(false)
          setError('更新后仍未重新连接本机节点，请手动启动 Win 端或重新下载。')
        } else {
          setMessage('Win 端正在更新升级，通信临时中断，会自动恢复。')
        }
      }
    }, 3000)
    return () => {
      canceled = true
      window.clearInterval(timer)
    }
  }, [adminUrl, latest, onStatus, polling])

  async function updateClient() {
    setBusy(true)
    setError('')
    setMessage('正在让本机 Win 端检查更新…')
    try {
      const data = await nodeApi<{ message?: string }>(adminUrl, '/api/client-maintenance/update', { method: 'POST' }, 12000)
      setMessage(data.message || 'Win 端正在更新升级，通信临时中断，会自动恢复。')
      setPolling(true)
    } catch (err) {
      setBusy(false)
      setError((err as Error).message)
    }
  }

  return (
    <section className={[styles.updateCard, styles[`update_${updateState.kind}`]].join(' ')}>
      <div className={styles.updateHead}>
        <div>
          <span className={styles.codexLabel}>Win 端更新</span>
          <h4>{updateState.title}</h4>
        </div>
        <span className={styles.updateBadge}>
          {updateState.kind === 'update' ? '可更新' : updateState.kind === 'current' ? '最新' : '待确认'}
        </span>
      </div>
      <p>{updateState.detail}</p>
      <div className={styles.updateGrid}>
        <div><span>本机</span><strong>{updateState.localLabel}</strong></div>
        <div><span>服务器</span><strong>{updateState.latestLabel}</strong></div>
        <div><span>安装状态</span><strong>{maintenance?.installed === false ? '未完整安装' : maintenance?.installed === true ? '已安装' : '检测中'}</strong></div>
        <div><span>安装包</span><strong>{latest?.windowsClientFileSize ? formatBytes(latest.windowsClientFileSize) : '未知'}</strong></div>
      </div>
      {maintenance?.maintenance_overview?.detail && <p className={styles.updateHint}>{maintenance.maintenance_overview.detail}</p>}
      <div className={styles.actions}>
        <button className={[styles.btn, updateState.kind === 'update' ? styles.primary : ''].join(' ')} onClick={updateClient} disabled={busy || !canUpdate}>
          {busy || polling ? '更新中…' : updateState.kind === 'update' ? '更新并重启 Win 端' : '检查更新'}
        </button>
        <button className={styles.btn} onClick={() => load(false)} disabled={busy}>重新检测</button>
      </div>
      {updateAction?.description && <p className={styles.updateHint}>{updateAction.description}</p>}
      {message && <p className={styles.resultOk}>{message}</p>}
      {error && <p className={styles.resultErr}>{error}</p>}
    </section>
  )
}
