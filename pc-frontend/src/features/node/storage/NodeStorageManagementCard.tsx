import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { analyzeNodeCacheArchitecture, cleanupNodeDataRoot, fetchNodeDataRoot, saveNodeDataRoot } from './nodeStorageApi'
import type { NodeCacheAdvisorReport, NodeDataRootCleanupResult, NodeDataRootStatus } from './types'
import styles from './NodeStorageManagementCard.module.css'

interface NodeStorageManagementCardProps {
  adminUrl: string
}

type BusyAction = '' | 'refresh' | 'save' | 'preview' | 'cleanup' | 'analyze'

function formatBytes(value?: number): string {
  if (!Number.isFinite(value) || (value ?? 0) < 0) return '未上报'
  const bytes = Number(value)
  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let amount = bytes
  let unit = -1
  do {
    amount /= 1024
    unit += 1
  } while (amount >= 1024 && unit < units.length - 1)
  return `${amount >= 10 ? amount.toFixed(1) : amount.toFixed(2)} ${units[unit]}`
}

function errorMessage(error: unknown, fallback: string): string {
  return error instanceof Error && error.message.trim() ? error.message : fallback
}

function activeTaskCount(status: NodeDataRootStatus | null): number {
  if (!status) return 0
  const reportedTasks = Array.isArray(status.active_tasks)
    ? status.active_tasks.length
    : Number.isFinite(status.active_tasks) ? Number(status.active_tasks) : 0
  return Math.max(
    0,
    Number.isFinite(status.build_cache?.active_leases) ? Number(status.build_cache?.active_leases) : 0,
    Number.isFinite(status.active_task_count) ? Number(status.active_task_count) : 0,
    reportedTasks,
  )
}

function diskNumbers(status: NodeDataRootStatus | null): { total?: number; free?: number } {
  return {
    total: status?.build_cache?.disk_total_bytes ?? status?.disk_total_bytes ?? status?.disk?.total_bytes,
    free: status?.build_cache?.disk_free_bytes ?? status?.disk_free_bytes ?? status?.disk?.available_bytes ?? status?.disk?.free_bytes,
  }
}

function pathLabel(kind?: string): string {
  if (kind === 'workspace') return '项目工作区'
  if (kind === 'storage') return 'Git 硬盘仓库'
  if (kind === 'cache') return '构建缓存'
  if (kind === 'temp') return '任务临时目录'
  return kind || '目录'
}

export default function NodeStorageManagementCard({ adminUrl }: NodeStorageManagementCardProps) {
  const [status, setStatus] = useState<NodeDataRootStatus | null>(null)
  const [rootPath, setRootPath] = useState('')
  const [busy, setBusy] = useState<BusyAction>('refresh')
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')
  const [cleanupPreview, setCleanupPreview] = useState<NodeDataRootCleanupResult | null>(null)
  const [advisor, setAdvisor] = useState<NodeCacheAdvisorReport | null>(null)
  const rootDirty = useRef(false)

  const refresh = useCallback(async (quiet = false) => {
    if (!quiet) setBusy('refresh')
    if (!quiet) setMessage('')
    setError('')
    try {
      const next = await fetchNodeDataRoot(adminUrl)
      setStatus(next)
      setAdvisor((current) => current?.candidates?.some((item) => item.estimated_bytes !== undefined)
        ? current
        : next.cache_advisor ?? null)
      if (!rootDirty.current) setRootPath(String(next.root_path ?? ''))
    } catch (caught) {
      setStatus(null)
      setError(errorMessage(caught, '无法读取节点数据盘状态'))
    } finally {
      if (!quiet) setBusy('')
    }
  }, [adminUrl])

  useEffect(() => {
    void refresh()
  }, [refresh])

  const tasks = activeTaskCount(status)
  const disk = diskNumbers(status)
  const freeRatio = disk.total && disk.free !== undefined ? disk.free / disk.total : undefined
  const pressure = status?.build_cache?.pressure
    ? 'warning'
    : freeRatio === undefined ? 'unknown' : freeRatio < 0.2 ? 'warning' : 'ok'
  const warnings = useMemo(
    () => [...(status?.warnings ?? []), ...(status?.capacity_warnings ?? [])].filter(Boolean),
    [status],
  )
  const derivedPaths = [
    ['项目工作区', status?.workspace_root],
    ['Git 硬盘仓库', status?.storage_root],
    ['构建缓存', status?.cache_root],
    ['任务临时目录', status?.temp_root],
  ]

  async function saveRoot() {
    const value = rootPath.trim()
    if (!value) {
      setError('请输入绝对路径，例如 D:\\ElonNodeData')
      return
    }
    setBusy('save')
    setMessage('')
    setError('')
    try {
      const response = await saveNodeDataRoot(adminUrl, value)
      if (response.data_root) setStatus(response.data_root)
      setRootPath(String(response.data_root?.root_path ?? value))
      rootDirty.current = false
      setCleanupPreview(null)
      setAdvisor(null)
      setMessage(response.restart_recommended
        ? '推荐数据根已保存。建议重启一龙开发平台；后续新建托管数据使用新目录，旧项目保持不变。'
        : response.message || '数据根已保存。')
      await refresh(true)
    } catch (caught) {
      setError(errorMessage(caught, '保存节点数据根失败'))
    } finally {
      setBusy('')
    }
  }

  async function previewCleanup() {
    setBusy('preview')
    setMessage('')
    setError('')
    try {
      const response = await cleanupNodeDataRoot(adminUrl, false)
      setCleanupPreview(response.cleanup ?? null)
      setMessage(`预计可清理 ${formatBytes(response.cleanup?.estimated_bytes)}，不会触碰项目源码和 Git 仓库；失败任务的诊断 Temp 也会被删除。`)
    } catch (caught) {
      setError(errorMessage(caught, '无法估算可清理空间'))
    } finally {
      setBusy('')
    }
  }

  async function applyCleanup() {
    if (!window.confirm('只删除一龙数据根内由平台创建、可重新生成的 cache 和 temp（包括失败任务诊断 Temp）。外部共享缓存、项目源码、工作区与 Git 仓库都不会删除。继续吗？')) return
    setBusy('cleanup')
    setMessage('')
    setError('')
    try {
      const response = await cleanupNodeDataRoot(adminUrl, true)
      setCleanupPreview(response.cleanup ?? null)
      setMessage(`安全清理完成，共处理 ${formatBytes(response.cleanup?.estimated_bytes)}。`)
      await refresh(true)
    } catch (caught) {
      setError(errorMessage(caught, '清理节点缓存失败'))
    } finally {
      setBusy('')
    }
  }

  async function analyzeCaches() {
    setBusy('analyze')
    setMessage('')
    setError('')
    try {
      const response = await analyzeNodeCacheArchitecture(adminUrl)
      setAdvisor(response.cache_advisor ?? null)
      const count = response.cache_advisor?.candidates?.length ?? 0
      setMessage(`只读体检完成，识别到 ${count} 处缓存架构。没有移动、接管或删除任何目录。`)
    } catch (caught) {
      setError(errorMessage(caught, '项目数据架构体检失败'))
    } finally {
      setBusy('')
    }
  }

  return (
    <section className={styles.card}>
      <div className={styles.header}>
        <div>
          <span className={styles.eyebrow}>项目数据架构体检</span>
          <h4>继承跑通的项目，再由 AI 渐进整理</h4>
          <p>旧项目和外部项目继续使用原目录、原环境和共享缓存。一龙只读识别分散缓存并提出建议；新建托管项目优先使用推荐数据根，但容量建议不会阻止任务。</p>
        </div>
        <span className={styles.state} data-tone={status?.configured ? pressure : 'warning'}>
          {busy === 'refresh' || busy === 'analyze' ? '检测中' : status?.configured ? '建议模式' : '可自动回填'}
        </span>
      </div>

      {status?.invalid_reason && (
        <div className={styles.alert} data-tone="warning" role="status">
          推荐数据根暂不可用：{status.invalid_reason}。旧项目仍按原路径和原缓存继续运行。
        </div>
      )}
      {!status?.configured && !status?.invalid_reason && (
        <div className={styles.alert} data-tone="info" role="status">
          无需手动搬项目。客户端会尝试回填推荐数据根；失败时仍继承原项目和原缓存继续运行。
        </div>
      )}
      {status?.build_cache?.pressure && (
        <div className={styles.alert} data-tone="warning" role="status">
          推荐数据根空间偏低，AI 会把它作为整理建议；不会因此阻止原项目，也不会自动删除缓存。
        </div>
      )}
      {warnings.map((warning, index) => <div className={styles.alert} data-tone="warning" role="alert" key={`${warning}-${index}`}>{warning}</div>)}
      {tasks > 0 && <div className={styles.alert} data-tone="warning" role="alert">当前有 {tasks} 个任务运行，切换数据根和实际清理会被安全阻止。</div>}

      <details className={styles.advancedSettings}>
        <summary>高级设置：手动更换存放位置</summary>
        <div className={styles.rootEditor}>
          <label htmlFor="elon-node-data-root">AI 临时工作区绝对路径</label>
          <div className={styles.rootRow}>
            <input
              id="elon-node-data-root"
              value={rootPath}
              onChange={(event) => {
                rootDirty.current = true
                setRootPath(event.target.value)
              }}
              placeholder="D:\\ElonNodeData"
              spellCheck={false}
              autoComplete="off"
              aria-describedby="elon-node-data-root-help"
            />
            <button type="button" className={styles.primaryButton} onClick={saveRoot} disabled={!!busy || !rootPath.trim() || tasks > 0}>
              {busy === 'save' ? '保存中…' : '更换位置'}
            </button>
          </div>
          <small id="elon-node-data-root-help">仅供高级用户调整。不能直接使用磁盘根目录，也不能放进现有项目目录。</small>
        </div>
      </details>

      <div className={styles.metrics}>
        <div><span>磁盘剩余</span><strong>{formatBytes(disk.free)}</strong></div>
        <div><span>磁盘容量</span><strong>{formatBytes(disk.total)}</strong></div>
        <div><span>建议保留空间</span><strong>{formatBytes(status?.build_cache?.min_free_bytes)}</strong></div>
        <div><span>建议任务余量</span><strong>{formatBytes(status?.build_cache?.build_headroom_bytes)}</strong></div>
        <div><span>缓存占用</span><strong>{formatBytes(status?.build_cache?.cache_bytes ?? status?.cache_bytes)}</strong></div>
        <div><span>临时占用</span><strong>{formatBytes(status?.build_cache?.temp_bytes ?? status?.temp_bytes)}</strong></div>
      </div>

      <div className={styles.advisor}>
        <div className={styles.sectionTitle}>
          <strong>AI 缓存架构建议</strong>
          <span>{advisor?.summary || '运行只读体检后，会识别历史共享缓存、开发检查缓存、Win 节点发布缓存、服务器发布缓存和仓库旧缓存。'}</span>
        </div>
        {(advisor?.candidates ?? []).map((item, index) => (
          <div className={styles.advisorItem} key={`${item.kind}-${item.path}-${index}`}>
            <span>{item.label || item.kind || '缓存目录'}</span>
            <code>{item.path || '未知目录'}</code>
            <b>{item.estimated_bytes == null ? (item.exists ? '已发现' : '未发现') : formatBytes(item.estimated_bytes)}</b>
            <small>{item.recommendation || '保持原地，等待 AI 给出兼容性与迁移建议。'}</small>
          </div>
        ))}
        {(advisor?.suggestions ?? []).map((suggestion, index) => <p key={`${suggestion}-${index}`}>{suggestion}</p>)}
      </div>

      <div className={styles.pathList}>
        {derivedPaths.map(([label, path]) => (
          <div key={label}>
            <span>{label}</span>
            <code>{path || '配置数据根后生成'}</code>
          </div>
        ))}
      </div>

      {status?.migration_required && (
        <div className={styles.migration}>
          <div className={styles.sectionTitle}>
            <strong>旧版数据已进入兼容保护</strong>
            <span>已有项目继续原地运行；只有后续新建的托管数据优先使用推荐根，不会擅自移动 Git 现场</span>
          </div>
          {(status.migration_plan ?? []).filter((item) => item.has_data).map((item, index) => (
            <div className={styles.migrationItem} key={`${item.kind}-${item.source_path}-${index}`}>
              <span>{pathLabel(item.kind)}</span>
              <code>{item.source_path}</code>
              <small>目标：{item.target_path || '请先配置数据根'}</small>
              {item.strategy && <p>{item.strategy}</p>}
            </div>
          ))}
        </div>
      )}

      {cleanupPreview && (
        <div className={styles.cleanupPreview}>
          <strong>{cleanupPreview.apply ? '最近清理结果' : '安全清理预览'} · {formatBytes(cleanupPreview.estimated_bytes)}</strong>
          {(cleanupPreview.entries ?? []).map((entry, index) => (
            <div key={`${entry.kind}-${entry.path}-${index}`}>
              <span>{pathLabel(entry.kind)}</span>
              <code>{entry.path || '未知目录'}</code>
              <b>{formatBytes(entry.estimated_bytes)}</b>
            </div>
          ))}
        </div>
      )}

      <div className={styles.actions}>
        <button type="button" onClick={() => void refresh()} disabled={!!busy}>刷新状态</button>
        <button type="button" onClick={analyzeCaches} disabled={!!busy}>{busy === 'analyze' ? '体检中…' : '分析本机缓存架构'}</button>
        <button type="button" onClick={previewCleanup} disabled={!!busy || !status?.configured}>预估可清理空间</button>
        <button type="button" className={styles.dangerButton} onClick={applyCleanup} disabled={!!busy || !status?.configured || tasks > 0}>
          {busy === 'cleanup' ? '清理中…' : '清理一龙自建缓存'}
        </button>
      </div>

      {message && <p className={styles.success} role="status" aria-live="polite">{message}</p>}
      {error && <p className={styles.error} role="alert">{error}</p>}
    </section>
  )
}
