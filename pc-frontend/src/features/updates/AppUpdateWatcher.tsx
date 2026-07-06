import { useEffect, useRef, useState } from 'react'
import { CheckCircle2, RefreshCw, X } from 'lucide-react'
import {
  consumeReloadNotice,
  dispatchBeforeAppUpdateReload,
  readStoredVersion,
  refreshingToastBody,
  saveReloadNotice,
  type ServerVersionInfo,
  updatedToastBody,
  versionIdentity,
  writeStoredVersion,
} from './appUpdateSession'
import styles from './AppUpdateWatcher.module.css'

const POLL_INTERVAL_MS = 45_000
const RELOAD_DELAY_MS = 1600
const UPDATED_TOAST_MS = 9000
const DEV_WATCH_FLAG = 'elon.pc.enableDevUpdateWatcher'

type ToastState =
  | { kind: 'refreshing'; title: string; body: string }
  | { kind: 'updated'; title: string; body: string }

export default function AppUpdateWatcher() {
  const [toast, setToast] = useState<ToastState | null>(null)
  const currentVersionRef = useRef<ServerVersionInfo | null>(readStoredVersion())
  const reloadingRef = useRef(false)

  useEffect(() => {
    const notice = consumeReloadNotice()
    if (!notice) return
    currentVersionRef.current = notice.to
    writeStoredVersion(notice.to)
    setToast({
      kind: 'updated',
      title: '已刷新到新版功能',
      body: updatedToastBody(notice.to),
    })
  }, [])

  useEffect(() => {
    if (!toast || toast.kind !== 'updated') return
    const timer = window.setTimeout(() => setToast(null), UPDATED_TOAST_MS)
    return () => window.clearTimeout(timer)
  }, [toast])

  useEffect(() => {
    if (!watcherEnabled()) return
    let disposed = false
    let timer: number | undefined

    async function checkVersion() {
      if (disposed || reloadingRef.current) return
      try {
        const nextVersion = await fetchServerVersion()
        if (disposed || !versionIdentity(nextVersion)) return

        const currentVersion = currentVersionRef.current ?? readStoredVersion()
        const currentId = versionIdentity(currentVersion)
        const nextId = versionIdentity(nextVersion)

        if (!currentId) {
          currentVersionRef.current = nextVersion
          writeStoredVersion(nextVersion)
          return
        }
        if (nextId !== currentId) {
          beginReload(currentVersion, nextVersion)
          return
        }

        currentVersionRef.current = nextVersion
        writeStoredVersion(nextVersion)
      } catch {
        // Server may be restarting during deploy. Keep the old page alive and retry.
      } finally {
        if (!disposed && !reloadingRef.current) {
          timer = window.setTimeout(checkVersion, POLL_INTERVAL_MS)
        }
      }
    }

    void checkVersion()
    return () => {
      disposed = true
      if (timer) window.clearTimeout(timer)
    }
  }, [])

  function beginReload(from: ServerVersionInfo | null, to: ServerVersionInfo) {
    if (reloadingRef.current) return
    reloadingRef.current = true
    const notice = {
      from,
      to,
      path: window.location.href,
      createdAt: Date.now(),
    }
    setToast({
      kind: 'refreshing',
      title: '发现新版，正在刷新',
      body: refreshingToastBody(to),
    })
    saveReloadNotice(notice)
    dispatchBeforeAppUpdateReload(notice)
    window.setTimeout(() => window.location.reload(), RELOAD_DELAY_MS)
  }

  if (!toast) return null
  const Icon = toast.kind === 'updated' ? CheckCircle2 : RefreshCw

  return (
    <div className={[styles.toast, toast.kind === 'updated' ? styles.updated : styles.refreshing].join(' ')}>
      <Icon className={styles.icon} size={18} aria-hidden="true" />
      <div className={styles.copy}>
        <strong className={styles.title}>{toast.title}</strong>
        <span className={styles.body}>{toast.body}</span>
      </div>
      {toast.kind === 'updated' && (
        <button className={styles.close} type="button" aria-label="关闭更新提示" onClick={() => setToast(null)}>
          <X size={14} aria-hidden="true" />
        </button>
      )}
    </div>
  )
}

async function fetchServerVersion(): Promise<ServerVersionInfo> {
  const res = await fetch('/api/server/version', { cache: 'no-store' })
  if (!res.ok) throw new Error(`version check failed: ${res.status}`)
  return res.json() as Promise<ServerVersionInfo>
}

function watcherEnabled(): boolean {
  if (!import.meta.env.DEV) return true
  try {
    return window.localStorage.getItem(DEV_WATCH_FLAG) === '1'
  } catch {
    return false
  }
}
